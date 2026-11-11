use std::cell::{OnceCell, RefCell};

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::layout::joined_width;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::statement_deletion;
use crate::rules::logic::inline::{
    LOCALS_READERS, declared_outer, fits_line, for_each_scoped_body, inlined_text, leading_name,
};
use crate::rules::{expr_mentions, has_version_gate, needs_parentheses};

pub fn walrus_candidate(checker: &Checker) {
    // `:=` arrived in Python 3.8.
    if checker.target_version() < PythonVersion::PY38 || has_version_gate(checker.body()) {
        return;
    }
    let mentions = OnceCell::new();
    let declared = RefCell::new(FxHashMap::default());
    for_each_scoped_body(
        checker.body(),
        Block { scope: Scope::Module, owner: None },
        0,
        &enter_block,
        &mut |body, block, depth| {
            for pair in body.windows(2) {
                if let [Stmt::Assign(assign), if_stmt @ Stmt::If(ast::StmtIf { test, .. })] = pair
                    && let Some(candidate) =
                        candidate(checker, &mentions, &declared, block.scope, assign, if_stmt, test)
                {
                    report(checker, &candidate, depth, !collapse_blocked(checker, body, block.owner, if_stmt));
                }
            }
        },
    );
}

// The innermost `def` or `class` around a body.
#[derive(Clone, Copy)]
enum Scope<'a> {
    Module,
    Function(&'a Stmt),
    // An assignment here defines a class attribute, which `Class.name` reads without a name reference.
    Class,
}

// The scope a body sits in, and the statement it belongs to.
#[derive(Clone, Copy)]
struct Block<'a> {
    scope: Scope<'a>,
    owner: Option<&'a Stmt>,
}

fn enter_block<'a>(block: Block<'a>, stmt: &'a Stmt) -> Block<'a> {
    let scope = match stmt {
        Stmt::FunctionDef(_) => Scope::Function(stmt),
        Stmt::ClassDef(_) => Scope::Class,
        _ => block.scope,
    };
    Block { scope, owner: Some(stmt) }
}

// Ruff's nested-`if` rule merges an `if` whose only statement is another `if`, but not past a comment in the outer
// header. The fix loop would withdraw this fix, yet `check` would still mark it fixable, so it is not offered.
fn collapse_blocked(checker: &Checker, body: &[Stmt], owner: Option<&Stmt>, if_stmt: &Stmt) -> bool {
    let (Some(Stmt::If(outer)), Stmt::If(inner)) = (owner, if_stmt) else {
        return false;
    };
    body.len() == 2
        && outer.elif_else_clauses.is_empty()
        && inner.elif_else_clauses.is_empty()
        && checker.has_comment_in(TextRange::new(outer.start(), inner.start()))
}

fn report(checker: &Checker, candidate: &Candidate, depth: usize, fixable: bool) {
    let &Candidate { assign, deleted, name, target, bare, single_use, .. } = candidate;
    let value_range = checker.parenthesized_range(&assign.value, assign.into());
    let value = checker.text(value_range);
    let replacement = match (single_use, bare) {
        (true, _) => inlined_text(checker, &assign.value, value_range, !bare && needs_parentheses(&assign.value)),
        (false, true) => format!("{name} := {value}"),
        (false, false) => format!("({name} := {value})"),
    };
    let wrapping = replacement.chars().count() - value.chars().count();
    if !header_fits(checker, candidate, value_range, wrapping, depth) {
        return;
    }
    let (message, title) = if single_use {
        (format!("Inline single-use `{name}` into the `if` test"), format!("Inline `{name}` into the `if` test"))
    } else {
        (
            format!("Assign `{name}` with `:=` inside the `if` test that consumes it"),
            format!("Assign `{name}` with `:=` in the `if` test"),
        )
    };
    let deletion = Edit::range_deletion(deleted);
    let fix = Fix::safe_edits(deletion, [Edit::range_replacement(replacement, target)]);
    checker.report_fix(Rule::WalrusCandidate, assign.range, message, fixable.then_some(fix), title);
}

// The formatter splits an `if` header wider than its line width over several lines, which reads worse than the
// assignment and the `if` the fix joins. The width is judged on `if <test>:` joined on one line at its block's
// indentation, without parentheses around the test, which the formatter drops from a header that fits, so the answer
// does not depend on how the code is laid out now.
fn header_fits(
    checker: &Checker,
    candidate: &Candidate,
    value_range: TextRange,
    wrapping: usize,
    depth: usize,
) -> bool {
    let (source, tokens) = (checker.source, checker.parsed.tokens());
    let (Some(test), Some(value)) =
        (joined_width(source, tokens, candidate.test.range()), joined_width(source, tokens, value_range))
    else {
        return false;
    };
    let header = "if ".len() + test + ":".len();
    fits_line(checker, depth, header, candidate.name.chars().count(), value, wrapping)
}

struct Candidate<'a> {
    assign: &'a ast::StmtAssign,
    test: &'a Expr,
    deleted: TextRange,
    name: &'a str,
    target: TextRange,
    bare: bool,
    single_use: bool,
}

fn candidate<'a>(
    checker: &Checker<'a>,
    mentions: &OnceCell<Mentions<'a>>,
    declared: &RefCell<FxHashMap<TextRange, FxHashSet<&'a str>>>,
    scope: Scope<'a>,
    assign: &'a ast::StmtAssign,
    if_stmt: &'a Stmt,
    test: &'a Expr,
) -> Option<Candidate<'a>> {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return None;
    };
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    let deleted = statement_deletion(checker, assign.range, None, false)?;
    // Type checkers treat a guard named `TYPE_CHECKING` as true whatever its value, so the name must stay.
    if name == "TYPE_CHECKING"
        || matches!(value, Expr::Lambda(_) | Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Named(_) | Expr::Starred(_))
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
        || expr_mentions(value, name)
        || checker.has_comment_in(TextRange::new(assign.start(), if_stmt.start()))
    {
        return None;
    }
    let (first, bare) = leading_name(test, true)?;
    if first.id.as_str() != name {
        return None;
    }
    // Inlining deletes the binding, so any other reader of it, earlier in the scope (a closure, the previous loop
    // iteration), in another module, or through `global`/`nonlocal`, would lose it.
    let (scope_range, shared) = match scope {
        Scope::Class => return None,
        Scope::Function(function) => (
            function.range(),
            function.as_function_def_stmt().is_some_and(|definition| {
                declared
                    .borrow_mut()
                    .entry(function.range())
                    .or_insert_with(|| declared_outer(&definition.body))
                    .contains(name)
            }),
        ),
        Scope::Module => (checker.file_range(), checker.is_exported(name)),
    };
    let mentions = mentions.get_or_init(|| Mentions::of(checker.body()));
    // `globals()` reads the module's names, so it keeps only a module-level name alive.
    if shared
        || LOCALS_READERS
            .into_iter()
            .chain(matches!(scope, Scope::Module).then_some("globals"))
            .any(|reader| mentions.within(reader, scope_range))
        || mentions.within(name, TextRange::new(scope_range.start(), assign.start()))
        || mentions.within(name, TextRange::new(if_stmt.end(), scope_range.end()))
    {
        return None;
    }
    Some(Candidate {
        assign,
        test,
        deleted,
        name,
        target: first.range,
        bare,
        single_use: !mentions.within(name, TextRange::new(first.end(), if_stmt.end())),
    })
}

// Where each name is mentioned in the module, so checking for a name inside a range is a lookup rather than a walk of
// every statement before it.
struct Mentions<'a>(FxHashMap<&'a str, Vec<TextRange>>);

impl<'a> Mentions<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut mentions = Self(FxHashMap::default());
        mentions.visit_body(body);
        for ranges in mentions.0.values_mut() {
            ranges.sort_unstable_by_key(Ranged::start);
        }
        mentions
    }

    // Names never overlap, so the first mention starting inside `range` is the only one that can decide it.
    fn within(&self, name: &str, range: TextRange) -> bool {
        self.0.get(name).is_some_and(|ranges| {
            let first = ranges.partition_point(|mention| mention.start() < range.start());
            ranges.get(first).is_some_and(|mention| mention.end() <= range.end())
        })
    }
}

impl<'a> Visitor<'a> for Mentions<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.0.entry(name.id.as_str()).or_default().push(name.range);
        }
        visitor::walk_expr(self, expr);
    }
}
