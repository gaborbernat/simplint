use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::{
    StoredNameFinder, any_over_body, any_over_expr, body_without_leading_docstring, is_dunder,
};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_codegen::Generator;
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::Exports;
use crate::rules::structure::needs_parentheses;
use crate::rules::{
    indentation, is_alone_on_lines, is_class_like, last_segment, lines_with_trailing_blanks, single_assignment,
};
use crate::semantic::{Binding, BindingKind, Evaluation};

pub fn single_user_constant(checker: &Checker) {
    for stmt in checker.body() {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        let private = match checker.dunder_all() {
            Exports::Listed(names) => !names.contains(&name),
            Exports::Absent | Exports::Dynamic => name.starts_with('_'),
        };
        if !private
            || checker.project().is_referenced(name)
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
        // A reference inside a module-level lambda reports the lambda as its function, and a constant cannot move there.
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
        if is_mutated(&function.body, &references) {
            continue;
        }
        let constant = Constant { stmt, name, annotation, value, function, references };
        let fix = move_into_function(checker, &constant);
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

// A module value the function mutates is state that must outlive each call, such as a set of warnings already shown.
fn is_mutated(body: &[Stmt], references: &[TextRange]) -> bool {
    let mut finder = Mutation { references, mutated: false };
    finder.visit_body(body);
    finder.mutated
}

struct Mutation<'a> {
    references: &'a [TextRange],
    mutated: bool,
}

impl<'a> Visitor<'a> for Mutation<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::With(with) = stmt
            && with.items.iter().any(|item| self.references.contains(&item.context_expr.range()))
        {
            self.mutated = true;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let target = match expr {
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if func.as_name_expr().is_some_and(|name| name.id.as_str() == "next") =>
            {
                arguments.args.first()
            }
            Expr::Call(ast::ExprCall { func, .. }) => func
                .as_attribute_expr()
                .filter(|attribute| {
                    matches!(
                        attribute.attr.as_str(),
                        "add"
                            | "append"
                            | "appendleft"
                            | "clear"
                            | "discard"
                            | "extend"
                            | "extendleft"
                            | "insert"
                            | "pop"
                            | "popitem"
                            | "popleft"
                            | "remove"
                            | "reverse"
                            | "setdefault"
                            | "sort"
                            | "update"
                    )
                })
                .map(|attribute| attribute.value.as_ref()),
            Expr::Subscript(ast::ExprSubscript {
                value, ctx: ast::ExprContext::Store | ast::ExprContext::Del, ..
            })
            | Expr::Attribute(ast::ExprAttribute {
                value, ctx: ast::ExprContext::Store | ast::ExprContext::Del, ..
            }) => Some(value.as_ref()),
            _ => None,
        };
        if target.is_some_and(|target| self.references.contains(&target.range())) {
            self.mutated = true;
        }
        visitor::walk_expr(self, expr);
    }
}

fn move_into_function(checker: &Checker, constant: &Constant) -> Option<Fix> {
    let source = checker.source;
    let function = constant.function;
    // Built inside the function, the value reads the module's names when the function runs; a name the module rebinds
    // or deletes later, such as a helper class removed with `del`, would no longer hold what it held at import.
    // A parameter or local of the function with the same name as a module name the value reads, such as an `operator`
    // parameter next to `operator.eq`, would capture that name once the value moves inside.
    let mut stores = StoredNameFinder::default();
    stores.visit_body(&function.body);
    let is_local = |name: &str| {
        stores.names.contains_key(name) || function.parameters.iter().any(|parameter| parameter.name().as_str() == name)
    };
    if !is_alone_on_lines(source, constant.stmt.range())
        || any_over_expr(constant.value, |expr| {
            expr.as_name_expr()
                .is_some_and(|name| checker.semantic().is_rebound(name.id.as_str()) || is_local(name.id.as_str()))
        })
    {
        return None;
    }
    let deletion = Edit::range_deletion(deletion_keeping_separation(source, constant.stmt.range()));
    let value_text = checker.parenthesized_text(constant.value, constant.stmt.into());
    // Inside an f-string replacement field a value with quotes is a syntax error before Python 3.12, and any longer
    // expression buries the text the f-string builds, so only a plain name or number goes there.
    let simple = matches!(constant.value, Expr::Name(_) | Expr::Attribute(_) | Expr::NumberLiteral(_));
    // A comment or a triple-quoted string keeps a value on several lines whatever the layout, so it gets a local name.
    let spans_lines =
        value_text.contains("\"\"\"") || value_text.contains("'''") || checker.has_comment_in(constant.value.range());
    if let [reference] = constant.references.as_slice()
        && !spans_lines
        && !is_repeated(&function.body, *reference)
        && (simple || !in_fstring(&function.body, *reference))
        && !(constant.value.is_string_literal_expr() && is_format_receiver(&function.body, *reference))
    {
        return Some(inline_fix(checker, constant.value, value_text, deletion, *reference));
    }
    local_fix(checker, constant, value_text, deletion)
}

fn inline_fix(checker: &Checker, value: &Expr, value_text: &str, deletion: Edit, reference: TextRange) -> Fix {
    // A value the layout spreads over lines goes in on one, so the result does not depend on the layout.
    let (text, parenthesized) = if value_text.contains('\n') {
        (Generator::from(checker.stylist).expr(value), false)
    } else {
        (value_text.to_owned(), value_text != checker.text(value.range()))
    };
    // Ruff's code generator writes a tuple without the parentheses its source may have.
    let wrap = !parenthesized && (needs_parentheses(value) || value_text.contains('\n') && value.is_tuple_expr());
    let inlined = if wrap { format!("({text})") } else { text };
    Fix::unsafe_edits(deletion, [Edit::range_replacement(inlined, reference)])
}

fn local_fix(checker: &Checker, constant: &Constant, value_text: &str, deletion: Edit) -> Option<Fix> {
    let source = checker.source;
    let function = constant.function;
    let local = constant.name.trim_start_matches('_').to_ascii_lowercase();
    if !is_identifier(&local) || checker.name_token_count(&local) > 0 {
        return None;
    }
    let anchor = body_without_leading_docstring(&function.body)
        .iter()
        .find(|candidate| candidate.range().contains_range(constant.references[0]))?;
    // A body on the `def` line, as in `def f(): return _X`, has no line of its own to declare the local on, and its
    // statements move onto lines of their own, where `ruff format` would put them.
    let on_header = has_leading_content(function.body[0].start(), source);
    let indent = if on_header {
        format!("{}{}", indentation(source, function), checker.stylist.indentation().as_str())
    } else {
        indentation(source, anchor).to_owned()
    };
    let value_text = if value_text.contains("\"\"\"") || value_text.contains("'''") {
        value_text.to_owned()
    } else {
        value_text.replace('\n', &format!("\n{indent}"))
    };
    // An empty container needs its declared type to type check, while `Final` belongs to module scope only.
    let annotation = match constant.annotation {
        Some(Expr::Subscript(ast::ExprSubscript { value, slice, .. })) if last_segment(value) == Some("Final") => {
            Some(checker.text(slice.range()))
        }
        Some(annotation) if last_segment(annotation) == Some("Final") => None,
        Some(annotation) => Some(checker.text(annotation.range())),
        None => None,
    };
    let declared = annotation.map_or_else(|| local.clone(), |annotation| format!("{local}: {annotation}"));
    let declaration = format!("{declared} = {value_text}");
    let placements = if on_header {
        let mut gap_start = TextSize::of(source[..function.body[0].start().to_usize()].trim_end());
        let mut placements = Vec::new();
        for stmt in &function.body {
            let declared_here =
                if std::ptr::eq(stmt, anchor) { format!("{declaration}\n{indent}") } else { String::new() };
            placements.push(Edit::range_replacement(
                format!("\n{indent}{declared_here}"),
                TextRange::new(gap_start, stmt.start()),
            ));
            gap_start = stmt.end();
        }
        placements
    } else {
        vec![Edit::insertion(format!("{indent}{declaration}\n"), source.line_start(anchor.start()))]
    };
    let renames = constant.references.iter().map(|range| Edit::range_replacement(local.clone(), *range));
    Some(Fix::unsafe_edits(deletion, placements.into_iter().chain(renames)))
}
// Deleting a statement leaves the blank lines on both sides of it next to each other; drop the shorter run so the code
// around it keeps the wider separation, and keep everything when the statement directly follows code.
fn deletion_keeping_separation(source: &str, range: TextRange) -> TextRange {
    let lines = source.full_lines_range(range);
    let before = &source[..lines.start().to_usize()];
    let leading_text = before.trim_end_matches([' ', '\t', '\n']);
    let leading = before[leading_text.len()..].matches('\n').count().saturating_sub(1);
    let with_trailing = lines_with_trailing_blanks(source, range);
    let trailing = source[lines.end().to_usize()..with_trailing.end().to_usize()].matches('\n').count();
    if leading_text.is_empty() || (leading > 0 && trailing <= leading) {
        with_trailing
    } else if leading > 0 {
        let line_end = leading_text.len() + before[leading_text.len()..].find('\n').map_or(0, |offset| offset + 1);
        TextRange::new(TextSize::try_from(line_end).unwrap_or_default(), lines.end())
    } else {
        lines
    }
}

fn in_fstring(body: &[Stmt], reference: TextRange) -> bool {
    any_over_body(body, |expr| {
        matches!(expr, Expr::FString(_) | Expr::TString(_)) && expr.range().contains_range(reference)
    })
}

// A string literal calling `format` is Ruff's f-string (UP032) candidate, which Ruff reports only while no argument
// spans lines, so whether it does would depend on the layout `ruff format` gives the call.
fn is_format_receiver(body: &[Stmt], reference: TextRange) -> bool {
    any_over_body(body, |expr| {
        expr.as_call_expr()
            .and_then(|call| call.func.as_attribute_expr())
            .is_some_and(|attribute| attribute.attr.as_str() == "format" && attribute.value.range() == reference)
    })
}

fn is_repeated(body: &[Stmt], reference: TextRange) -> bool {
    let mut finder = RepeatedContext { reference, depth: 0, repeated: false };
    finder.visit_body(body);
    finder.repeated
}

struct RepeatedContext {
    reference: TextRange,
    depth: usize,
    repeated: bool,
}

impl<'a> Visitor<'a> for RepeatedContext {
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
        let repeats = matches!(
            expr,
            Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_)
        );
        self.depth += usize::from(repeats);
        visitor::walk_expr(self, expr);
        self.depth -= usize::from(repeats);
    }
}
