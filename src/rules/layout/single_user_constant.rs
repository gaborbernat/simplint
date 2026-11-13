use itertools::Itertools;
use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::{StoredNameFinder, is_dunder};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_codegen::Generator;
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::fold::is_literal;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, fix_of, keep_debug_text, statement_lines};
use crate::rules::rename::appears_in_strings;
use crate::rules::{is_class_like, is_named_by_doctest};
use crate::semantic::{Binding, BindingKind, Evaluation};
use crate::syntax::{Exports, Visibility, is_repeating_expr, last_segment, needs_parentheses, single_assignment};

pub fn single_user_constant(checker: &Checker) {
    let body = checker.body();
    let in_block = sibling_blocks(checker, body);
    for (stmt, _) in body.iter().zip(in_block).filter(|(_, in_block)| !in_block) {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        let private = match checker.dunder_all() {
            Exports::Listed(names) => !names.contains(name),
            Exports::Absent | Exports::Dynamic => Visibility::of(name) != Visibility::Public,
        };
        if !private
            || !is_literal(value)
            || checker.project().is_referenced(name)
            // A string that spells the name may patch or look up the constant, which inlining would break.
            || appears_in_strings(checker, name)
            // A doctest reads the constant in the module's globals, which inlining it into the function would break.
            || is_named_by_doctest(checker, name)
            || checker.project().is_attribute_assigned(name)
            || is_dunder(name)
            || is_class_like(name)
            || checker.comment_above(stmt).is_some()
            // `ruff format` wraps a value too long for its line in parentheses and moves the comment trailing it
            // inside them, so a comment anywhere after the value counts.
            || checker.has_comment_in(TextRange::new(value.end(), checker.source.line_end(stmt.end())))
        {
            continue;
        }
        let semantic = checker.semantic();
        if !matches!(semantic.bindings(name), [Binding { kind: BindingKind::Assignment | BindingKind::Annotated, .. }])
        {
            continue;
        }
        let references = semantic.references_to(name);
        // A reference inside a module-level lambda reports the lambda as its function, and a constant cannot
        // move there.
        let Some(function) = references
            .first()
            .and_then(|reference| reference.function)
            .and_then(|function| checker.function_defs().iter().copied().find(|candidate| candidate.range == function))
        else {
            continue;
        };
        if !references
            .iter()
            .all(|reference| reference.evaluation == Evaluation::Deferred && reference.function == Some(function.range))
        {
            continue;
        }
        let references: Vec<TextRange> = references.iter().map(|reference| reference.range).collect();
        let constant = Constant { stmt, name, annotation, value, function, references };
        let (fix, adds_local) = match move_into_function(checker, &constant) {
            Some((fix, adds_local)) => (Some(fix), adds_local),
            None => (None, false),
        };
        let from_ruff = &checker.settings.from_ruff;
        if adds_local && from_ruff.ruff_runs(RuffRule::TooManyLocals) {
            let mut locals = StoredNameFinder::default();
            locals.visit_body(&function.body);
            if locals.names.len() >= from_ruff.max_locals {
                continue;
            }
        }
        checker.report_fix(
            Rule::SingleUserConstant,
            stmt.range(),
            format!("`{name}` is used by one function only; define it there"),
            fix,
            format!("Move `{name}` into `{}`", constant.function.name),
        );
    }
}

struct Constant<'a> {
    stmt: &'a Stmt,
    name: &'a str,
    annotation: Option<&'a Expr>,
    value: &'a Expr,
    function: &'a ast::StmtFunctionDef,
    references: Vec<TextRange>,
}

fn move_into_function(checker: &Checker, constant: &Constant) -> Option<(Fix, bool)> {
    // A computed write may set the constant, which a copy in the function would not see.
    if checker.project().is_computed_written() {
        return None;
    }
    let source = checker.source;
    let function = constant.function;
    // A value or a renamed local would change the source text a t-string keeps.
    let source_spans = checker.state().source_text_spans(checker);
    if constant.references.iter().any(|reference| source_spans.iter().any(|span| span.contains_range(*reference))) {
        return None;
    }
    if !checker.is_alone_on_lines(constant.stmt.range()) {
        return None;
    }
    let deletion = Edit::range_deletion(statement_lines(source, constant.stmt.range(), Blanks::WiderSide));
    let value_text = checker.parenthesized_text(constant.value, constant.stmt.into());
    // A comment or a triple-quoted string keeps a value on several lines whatever the layout, so it gets a local name.
    let spans_lines =
        value_text.contains("\"\"\"") || value_text.contains("'''") || checker.has_comment_in(constant.value.range());
    // An inlined literal is inferred on its own, which may not match the annotation the constant declares, so an
    // annotated constant keeps its name as a local.
    // Inside an f-string replacement field a value with quotes is a syntax error before Python 3.12, and any longer
    // expression buries the text the f-string builds, so only a number goes there.
    if let [reference] = constant.references.as_slice()
        && !spans_lines
        && local_annotation(checker, constant).is_none()
    {
        let context = ReferenceContext::of(&function.body, *reference);
        if !context.repeated
            && (constant.value.is_number_literal_expr() || !context.in_fstring)
            && (!context.format_receiver || !constant.value.is_string_literal_expr())
        {
            return inline_fix(checker, constant.value, value_text, deletion, *reference).map(|fix| (fix, false));
        }
    }
    local_fix(checker, constant, value_text, deletion).map(|fix| (fix, true))
}

// The type a local keeps from the constant's annotation. `Final` belongs to module scope only, so `Final[T]` keeps `T`
// and a bare `Final` keeps nothing.
fn local_annotation<'a>(checker: &Checker<'a>, constant: &Constant) -> Option<&'a str> {
    match constant.annotation {
        Some(Expr::Subscript(ast::ExprSubscript { value, slice, .. })) if last_segment(value) == Some("Final") => {
            Some(checker.text(slice.range()))
        }
        Some(annotation) if last_segment(annotation) == Some("Final") => None,
        Some(annotation) => Some(checker.text(annotation.range())),
        None => None,
    }
}

fn inline_fix(checker: &Checker, value: &Expr, value_text: &str, deletion: Edit, reference: TextRange) -> Option<Fix> {
    // A value the layout spreads over lines goes in on one, so the result does not depend on the layout.
    let (text, parenthesized) = if value_text.contains('\n') {
        (Generator::from(checker.stylist).expr(value), false)
    } else {
        (value_text.to_owned(), value_text != checker.text(value.range()))
    };
    // An integer followed by `.` lexes as the point of a float, so the attribute needs parentheses around the number.
    let int_attribute = matches!(value, Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(_), .. }))
        && checker.source[reference.end().to_usize()..].trim_start_matches(' ').starts_with('.');
    // Ruff's code generator writes a tuple without the parentheses its source may have.
    let wrap = !parenthesized
        && (needs_parentheses(value) || value_text.contains('\n') && value.is_tuple_expr() || int_attribute);
    let inlined = if wrap { format!("({text})") } else { text };
    fix_of(
        keep_debug_text(checker, vec![deletion, Edit::range_replacement(inlined, reference)])?,
        Applicability::Unsafe,
    )
}

fn local_fix(checker: &Checker, constant: &Constant, value_text: &str, deletion: Edit) -> Option<Fix> {
    let source = checker.source;
    let function = constant.function;
    let local = constant.name.trim_start_matches('_').to_lowercase();
    if !is_identifier(&local) || checker.name_token_count(&local) > 0 {
        return None;
    }
    // A deferred read of this function lies in its body, so the statement holding it is the last one starting before
    // it.
    let anchor =
        &function.body[function.body.partition_point(|stmt| stmt.start() <= constant.references[0].start()) - 1];
    // A body on the `def` line, as in `def f(): return _X`, has no line of its own to declare the local on, and its
    // statements move onto lines of their own, where `ruff format` would put them.
    let on_header = has_leading_content(function.body[0].start(), source);
    let indent = if on_header {
        format!("{}{}", checker.indentation(function), checker.stylist.indentation().as_str())
    } else {
        checker.indentation(anchor).to_owned()
    };
    let triple_quoted = value_text.contains("\"\"\"") || value_text.contains("'''");
    // A backslash continues a single-quoted string onto the next line, so the indentation added there becomes part of
    // the value.
    if !triple_quoted
        && checker.parsed.tokens().in_range(constant.value.range()).iter().any(|token| {
            matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle)
                && source.contains_line_break(token.range())
        })
    {
        return None;
    }
    let value_text =
        if triple_quoted { value_text.to_owned() } else { value_text.replace('\n', &format!("\n{indent}")) };
    let declared = local_annotation(checker, constant)
        .map_or_else(|| local.clone(), |annotation| format!("{local}: {annotation}"));
    let declaration = format!("{declared} = {value_text}");
    let eol = checker.line_ending();
    let placements = if on_header {
        let mut gap_start = TextSize::of(source[..function.body[0].start().to_usize()].trim_end());
        let mut placements = Vec::new();
        for stmt in &function.body {
            let declared_here =
                if std::ptr::eq(stmt, anchor) { format!("{declaration}{eol}{indent}") } else { String::new() };
            placements.push(Edit::range_replacement(
                format!("{eol}{indent}{declared_here}"),
                TextRange::new(gap_start, stmt.start()),
            ));
            gap_start = stmt.end();
        }
        placements
    } else {
        vec![Edit::insertion(format!("{indent}{declaration}{eol}"), source.line_start(anchor.start()))]
    };
    let renames = constant.references.iter().map(|range| Edit::range_replacement(local.clone(), *range));
    let edits = std::iter::once(deletion).chain(placements).chain(renames).collect();
    fix_of(keep_debug_text(checker, edits)?, Applicability::Unsafe)
}

// Where the one reference to a constant sits, which decides whether the value can replace it.
struct ReferenceContext {
    reference: TextRange,
    // How many loops, functions, classes, lambdas and comprehensions the walk is in.
    depth: usize,
    // Inside one of those, the value would be evaluated again on every pass instead of once.
    repeated: bool,
    in_fstring: bool,
    // A string literal calling `format` is Ruff's f-string (UP032) candidate, which Ruff reports only while no argument
    // spans lines, so whether it does would depend on the layout `ruff format` gives the call.
    format_receiver: bool,
}

impl ReferenceContext {
    fn of(body: &[Stmt], reference: TextRange) -> Self {
        let mut context = Self { reference, depth: 0, repeated: false, in_fstring: false, format_receiver: false };
        context.visit_body(body);
        context
    }
}

impl<'a> Visitor<'a> for ReferenceContext {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let repeats = matches!(stmt, Stmt::For(_) | Stmt::While(_) | Stmt::FunctionDef(_) | Stmt::ClassDef(_));
        self.depth += usize::from(repeats);
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(repeats);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.range() == self.reference {
            self.repeated = self.depth > 0;
        }
        self.in_fstring |=
            matches!(expr, Expr::FString(_) | Expr::TString(_)) && expr.range().contains_range(self.reference);
        self.format_receiver |= expr
            .as_call_expr()
            .and_then(|call| call.func.as_attribute_expr())
            .is_some_and(|attribute| attribute.attr.as_str() == "format" && attribute.value.range() == self.reference);
        let repeats = is_repeating_expr(expr);
        self.depth += usize::from(repeats);
        visitor::walk_expr(self, expr);
        self.depth -= usize::from(repeats);
    }
}

// Whether each statement belongs to a run of at least three adjacent literal constants that different functions read,
// such as a palette or a set of limits, which reads as one unit even when each constant has a single user.
fn sibling_blocks(checker: &Checker, body: &[Stmt]) -> Vec<bool> {
    const BLOCK: usize = 3;
    let user = |name: &str| checker.semantic().references_to(name).first()?.function;
    let mut in_block = vec![false; body.len()];
    let mut start = 0;
    while start < body.len() {
        let run: Vec<&str> = body[start..].iter().map_while(literal_constant).collect();
        if run.len() >= BLOCK && run.iter().filter_map(|name| user(name)).unique().nth(1).is_some() {
            in_block[start..start + run.len()].fill(true);
        }
        start += run.len().max(1);
    }
    in_block
}

fn literal_constant(stmt: &Stmt) -> Option<&str> {
    single_assignment(stmt).filter(|(_, _, value)| is_literal(value)).map(|(target, _, _)| target.id.as_str())
}
