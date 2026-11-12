use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, Import, above_comments, ensure_import, fix_of, preamble_len, statement_lines};
use crate::rules::logic::regex_pattern::{Call, Context, Facts, Literal};
use crate::rules::typing::missing_final::final_qualifier;

const METACHARACTERS: &[u8] = b".^$*+?{[\\|()";
const ESCAPABLE: &[u8] = b"abfnrtv";

pub fn inline_regex(checker: &Checker) {
    let facts = Facts::collect(checker);
    let sites: Vec<(&Literal, &Call)> = facts
        .sites()
        .into_iter()
        .filter(|(literal, call)| {
            call.scope.is_some() && call.verbose.is_some() && !left_to_ruff(checker, literal, call)
        })
        .collect();
    let reports = sites
        .iter()
        .map(|(_, call)| {
            let message = if call.attr == "compile" {
                "Move this `re.compile` to a module-level constant".to_owned()
            } else {
                format!("Compile this pattern once at module level and call its `{}`", call.attr)
            };
            (call.node.range(), message, "Move the pattern to a module-level constant".to_owned())
        })
        .collect();
    checker.report_shared_fix(Rule::InlineRegex, reports, || shared_fix(checker, &sites));
}

// Ruff's RUF055 rewrites these calls to `str` methods, and this rule would undo that, so Ruff keeps them. Each
// condition is RUF055's own, so a plain-text call Ruff does not rewrite, such as a value of `re.match`, is reported.
fn left_to_ruff(checker: &Checker, literal: &Literal, call: &Call) -> bool {
    let decoded = literal.decoded();
    let plain = !decoded.iter().any(|byte| METACHARACTERS.contains(byte));
    let is_re =
        checker.semantic().qualified_name(&call.node.func).is_some_and(|name| matches!(name.segments(), ["re", _]));
    let arguments = &call.node.arguments;
    let reported = match call.attr {
        "split" => !decoded.is_empty() && arguments.len() == 2 && arguments.find_argument_value("string", 1).is_some(),
        "sub" => {
            arguments.len() == 3
                && arguments.find_argument_value("string", 2).is_some()
                && arguments
                    .find_argument_value("repl", 1)
                    .and_then(|repl| literal_value(checker, repl))
                    .is_some_and(|value| !has_unescapable_escape(&value))
        }
        "match" | "prefixmatch" | "search" | "fullmatch" => {
            arguments.len() == 2
                && arguments.find_argument_value("string", 1).is_some()
                && call.context == Context::Test
        }
        _ => false,
    };
    is_re && plain && reported
}

// The bytes a replacement literal, or a name bound once to one, holds, as RUF055 resolves them.
fn literal_value(checker: &Checker, repl: &Expr) -> Option<Vec<u8>> {
    let value = match repl {
        Expr::Name(name) => checker.semantic().only_binding_value(name)?,
        _ => repl,
    };
    Literal::of(value).map(|literal| literal.decoded())
}

// A backslash before a letter or digit that is not a character escape makes `re.sub` raise, so RUF055 leaves that
// call out; the escapes it does report carry no fix there.
fn has_unescapable_escape(value: &[u8]) -> bool {
    value.windows(2).any(|pair| pair[0] == b'\\' && pair[1].is_ascii_alphanumeric() && !ESCAPABLE.contains(&pair[1]))
}

// One fix for the whole module. Each constant goes above the top-level statement its call sits in, with one blank line
// between constants and two after the last, the spacing `ruff format` keeps before a `def`. The constants sharing a
// statement share one insertion, so they come out in source order, and a later pass lands below the ones before it.
fn shared_fix(checker: &Checker, sites: &[(&Literal, &Call)]) -> Option<Fix> {
    let spelling = final_qualifier(checker, TextRange::default())?;
    let body = checker.body();
    let index = {
        let preamble = preamble_len(body);
        preamble + body[preamble..].iter().take_while(|stmt| is_import(stmt)).count()
    };
    if body[index..].iter().any(is_import) {
        return None;
    }

    let modules: BTreeSet<String> = sites.iter().map(|(_, call)| module_of(checker, call)).collect();
    let module_imports: BTreeMap<String, Import> = modules
        .into_iter()
        .filter_map(|module| {
            let import = ensure_import(checker, &module, None, TextRange::default())?;
            Some((module, import))
        })
        .collect();

    let mut constants = FxHashSet::default();
    let mut included: Vec<TextRange> = Vec::new();
    let mut blocks: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut replacements = Vec::new();
    for (literal, call) in sites {
        let range = call.node.range();
        let module = module_of(checker, call);
        if !module_imports.contains_key(&module)
            || checker.has_comment_in(range)
            || included.iter().any(|outer| outer.contains_range(range))
        {
            continue;
        }
        let Some(function) = enclosing_function(checker, range) else {
            continue;
        };
        let anchor = body
            .iter()
            .position(|stmt| stmt.range().contains_range(range))
            .expect("a call sits in a statement of the module body");
        let constant = constant_name(checker, call, function, &constants);
        constants.insert(constant.clone());
        let alias = alias_edits(checker, call, function, &constant, sites);

        let value_type = if literal.bytes { "bytes" } else { "str" };
        let mut compile_arguments = vec![checker.text(literal.range)];
        compile_arguments.extend(call.flags.map(|flags| checker.text(flags)));
        blocks.entry(anchor).or_default().push(format!(
            "{constant}: {}[{module}.Pattern[{value_type}]] = {module}.compile({})",
            spelling.qualifier,
            compile_arguments.join(", "),
        ));
        if let Some(edits) = alias {
            replacements.extend(edits);
        } else {
            let replacement = if call.attr == "compile" {
                constant
            } else {
                format!("{constant}.{}({})", call.attr, remaining_arguments(checker, call).join(", "))
            };
            replacements.push(Edit::range_replacement(replacement, range));
        }
        included.push(range);
    }
    if included.is_empty() {
        return None;
    }

    let eol = checker.line_ending();
    let separator = eol.repeat(2);
    let insertions = blocks.into_iter().map(|(anchor, declarations)| {
        let block = format!("{}{}", declarations.join(separator.as_str()), eol.repeat(3));
        Edit::insertion(block, above_comments(checker, &body[anchor]))
    });

    let imports: Vec<Import> = module_imports.into_values().chain([spelling.import]).collect();
    let isolated = imports.iter().any(|import| matches!(import, Import::Add(_)));
    let import_edits = imports.into_iter().filter_map(|import| match import {
        Import::Add(edit) => Some(edit),
        Import::Present => None,
    });
    let fix = fix_of(import_edits.chain(insertions).chain(replacements), Applicability::Unsafe)?;
    Some(if isolated { fix.isolate(IsolationLevel::Group(1)) } else { fix })
}

fn is_import(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_))
}

// The module a recorded call's function comes from, as `re` or `regex`; the facts record only calls that resolve.
fn module_of(checker: &Checker, call: &Call) -> String {
    let name = checker
        .semantic()
        .qualified_name(&call.node.func)
        .expect("the facts record only calls that resolve to a regex module");
    name.segments()[0].to_owned()
}

// The arguments the rewritten method call keeps, in source order: all but the pattern and the flags, which move into
// the constant.
fn remaining_arguments<'a>(checker: &Checker<'a>, call: &Call) -> Vec<&'a str> {
    let pattern = call.node.arguments.find_argument("pattern", 0).map(|argument| argument.range());
    let flags = call.flags.map(|argument| argument.range());
    let mut ranges: Vec<TextRange> = call
        .node
        .arguments
        .args
        .iter()
        .map(Ranged::range)
        .chain(call.node.arguments.keywords.iter().map(Ranged::range))
        .filter(|range| Some(*range) != pattern && Some(*range) != flags)
        .collect();
    ranges.sort_by_key(|&range| range.start());
    ranges.into_iter().map(|range| checker.text(range)).collect()
}

// Targets that name the pattern's role rather than its content, so the function's name says more about the constant.
const GENERIC_TARGETS: [&str; 11] =
    ["compile", "compiled", "expr", "expression", "matcher", "pat", "pattern", "re", "regex", "regexp", "rx"];

// The name a constant takes: the assignment target the call's result goes to, unless it is generic or shorter than
// three letters, then the enclosing function's name, then that name with a number after it. Each is in upper case,
// with `_PATTERN` after it unless the last word already says pattern or regex. A name the module already spells, or
// a constant already chosen, is skipped.
fn constant_name(checker: &Checker, call: &Call, function: &ast::StmtFunctionDef, taken: &FxHashSet<String>) -> String {
    let function_stem = pattern_stem(function.name.as_str().trim_matches('_'));
    let target = match call.context {
        Context::Bind { name, .. } => {
            let name = name.trim_matches('_');
            (name.len() >= 3 && !GENERIC_TARGETS.contains(&name.to_lowercase().as_str())).then(|| pattern_stem(name))
        }
        _ => None,
    };
    target
        .into_iter()
        .chain([function_stem.clone()])
        .chain((2..=usize::MAX).map(|index| format!("{function_stem}_{index}")))
        .map(|stem| format!("_{stem}"))
        .find(|name| checker.name_token_count(name) == 0 && !taken.contains(name))
        .expect("some number after the function's stem is unused")
}

fn pattern_stem(base: &str) -> String {
    let upper = base.to_uppercase();
    if matches!(upper.rsplit('_').next(), Some("PATTERN" | "RE" | "REGEX" | "REGEXP")) {
        upper
    } else {
        [upper.as_str(), "PATTERN"].into_iter().filter(|word| !word.is_empty()).join("_")
    }
}

// The edits that drop a plain local the call's pattern is assigned to, once the constant replaces every read of it:
// the assignment and each read, which then reads the constant. Only a `re.compile` result qualifies, since a match
// result is not the pattern. The local must be bound once in the function, which must not delete it or declare it
// `global` or `nonlocal`; no other call of the run may read it, and its text may not be kept as written.
fn alias_edits(
    checker: &Checker,
    call: &Call,
    function: &ast::StmtFunctionDef,
    constant: &str,
    sites: &[(&Literal, &Call)],
) -> Option<Vec<Edit>> {
    let Context::Bind { name, .. } = call.context else {
        return None;
    };
    if call.attr != "compile" {
        return None;
    }
    let assign = function.body.iter().find_map(|stmt| match stmt {
        Stmt::Assign(assign) if assign.value.range() == call.node.range() => Some(assign),
        _ => None,
    })?;
    let statement = assign.range();
    if !checker.is_alone_on_lines(statement)
        || checker.comment_above(assign).is_some()
        || checker.has_comment_in(statement)
        || checker.trailing_comment(assign).is_some()
        || checker.identifier_count_in(call.node.range(), name) > 0
    {
        return None;
    }
    let bindings =
        checker.semantic().local_bindings(name).iter().filter(|range| function.range.contains_range(**range));
    if bindings.count() != 1 {
        return None;
    }
    let mut uses = LocalUses { name, reads: Vec::new(), fstrings: Vec::new(), declared: false };
    uses.visit_body(&function.body);
    let source_spans = checker.state().source_text_spans(checker);
    // An f-string's `=` form prints the expression as written, so a read in one keeps the local.
    let kept_as_written = |read: &TextRange| {
        sites.iter().any(|(_, site)| site.node.range().contains_range(*read))
            || source_spans.iter().any(|span| span.contains_range(*read))
            || uses.fstrings.iter().any(|fstring| fstring.contains_range(*read))
    };
    if uses.declared || uses.reads.is_empty() || uses.reads.iter().any(kept_as_written) {
        return None;
    }
    let deletion = Edit::range_deletion(statement_lines(checker.source, statement, Blanks::WiderSide));
    let renames = uses.reads.into_iter().map(|range| Edit::range_replacement(constant.to_owned(), range));
    Some(std::iter::once(deletion).chain(renames).collect())
}

// The reads of a local, in the function and any function nested in it, and whether any declares it `global` or
// `nonlocal`. A `del` is a second binding of the local, which the caller's binding count already refuses.
struct LocalUses<'a> {
    name: &'a str,
    reads: Vec<TextRange>,
    fstrings: Vec<TextRange>,
    declared: bool,
}

impl<'a> Visitor<'a> for LocalUses<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) = stmt {
            self.declared |= names.iter().any(|name| name.as_str() == self.name);
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) if name.ctx.is_load() && name.id.as_str() == self.name => self.reads.push(name.range()),
            Expr::FString(_) | Expr::TString(_) => self.fstrings.push(expr.range()),
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// The innermost `def` around a call, whose name a constant takes; `None` for a call only a lambda at module level
// holds.
fn enclosing_function<'a>(checker: &Checker<'a>, range: TextRange) -> Option<&'a ast::StmtFunctionDef> {
    checker
        .function_defs()
        .iter()
        .copied()
        .filter(|function| function.range.contains_range(range))
        .min_by_key(|function| function.range.len())
}
