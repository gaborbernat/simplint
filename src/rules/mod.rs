pub(crate) mod api;
pub(crate) mod classes;
pub(crate) mod comments;
pub(crate) mod docstrings;
pub(crate) mod imports;
pub(crate) mod layout;
pub(crate) mod logic;
mod module_variable;
pub(crate) mod naming;
mod rename;
pub(crate) mod testing;
pub(crate) mod typing;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, Stmt, StringFlags};
use ruff_python_trivia::{has_leading_content, has_trailing_content, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::syntax::{is_name, last_segment};

pub fn is_alone_on_lines(source: &str, range: TextRange) -> bool {
    !has_leading_content(range.start(), source) && !has_trailing_content(range.end(), source)
}

pub fn delete_statement(source: &str, stmt: &Stmt, body: &[Stmt]) -> Edit {
    if body.len() == 1 {
        return Edit::range_replacement("pass".to_owned(), stmt.range());
    }
    Edit::range_deletion(source.full_lines_range(stmt.range()))
}

// The text deleting a statement removes: its lines when it has them to itself, with the blank lines after them when
// `trailing_blanks` is set. A statement sharing its line goes from the `;` that joins it to the statement before it, as
// in `run(); x = 1`, or up to the `next` statement, as in `"""Doc."""; run()`, unless a comment in between would go
// with it; `ruff format` puts joined statements on lines of their own either way.
pub fn statement_deletion(
    checker: &Checker,
    range: TextRange,
    next: Option<TextRange>,
    trailing_blanks: bool,
) -> Option<TextRange> {
    let source = checker.source;
    if is_alone_on_lines(source, range) {
        return Some(if trailing_blanks {
            lines_with_trailing_blanks(source, range)
        } else {
            source.full_lines_range(range)
        });
    }
    let line_start = source.line_start(range.start());
    let before = source[TextRange::new(line_start, range.start())].trim_end();
    let after = &source[TextRange::new(range.end(), source.line_end(range.end()))];
    if before.ends_with(';') && after.trim().is_empty() {
        return Some(TextRange::new(line_start + before.text_len() - TextSize::from(1), range.end()));
    }
    let up_to_next = TextRange::new(range.start(), next?.start());
    (!checker.has_comment_in(up_to_next)).then_some(up_to_next)
}

pub fn lines_with_trailing_blanks(source: &str, range: TextRange) -> TextRange {
    let mut end = source.full_line_end(range.end());
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
    }
    let mut start = source.line_start(range.start());
    // At the end of the file the blank lines above go too: after a module left with comments only, `ruff format` keeps
    // them, and Ruff reports blank lines ending the file.
    if end.to_usize() == source.len() {
        while start > TextSize::default() && source.full_line_str(start - TextSize::from(1)).trim().is_empty() {
            start = source.line_start(start - TextSize::from(1));
        }
    }
    TextRange::new(start, end)
}

pub fn docstring(body: &[Stmt]) -> Option<&ast::ExprStringLiteral> {
    match body.first()? {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_string_literal_expr(),
        _ => None,
    }
}

pub fn is_main_guard(stmt: &Stmt) -> bool {
    let Stmt::If(ast::StmtIf { test, .. }) = stmt else {
        return false;
    };
    let Expr::Compare(ast::ExprCompare { ops, operands, .. }) = test.as_ref() else {
        return false;
    };
    let is_main =
        |expr: &Expr| expr.as_string_literal_expr().is_some_and(|literal| literal.value.to_str() == "__main__");
    match (&**ops, &**operands) {
        ([ast::CmpOp::Eq], [left, right]) => {
            (is_name(left, "__name__") && is_main(right)) || (is_main(left) && is_name(right, "__name__"))
        }
        _ => false,
    }
}

// The leading whitespace of the line a node starts on; a node after a `;` shares the line, not its indentation.
pub fn indentation(source: &str, ranged: impl Ranged) -> &str {
    leading_indentation(source.full_line_str(ranged.start()))
}

pub fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

pub fn needs_parentheses(value: &Expr) -> bool {
    OperatorPrecedence::from_expr(value) < OperatorPrecedence::CallAttribute
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
}

pub fn expr_mentions(expr: &Expr, name: &str) -> bool {
    any_over_expr(expr, |expr| is_name(expr, name))
}

// Each expression `pick` maps to a value, in the order a `Visitor` walks them.
pub struct ExprCollector<F, T> {
    pick: F,
    pub found: Vec<T>,
}

impl<F, T> ExprCollector<F, T> {
    pub fn new<'a>(pick: F) -> Self
    where
        F: FnMut(&'a Expr) -> Option<T>,
    {
        Self { pick, found: Vec::new() }
    }
}

impl<'a, F: FnMut(&'a Expr) -> Option<T>, T> Visitor<'a> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(value) = (self.pick)(expr) {
            self.found.push(value);
        }
        visitor::walk_expr(self, expr);
    }
}

// The spans of the t-strings in `body`, which keep the text of their expressions, so a name changed inside one changes
// the output.
pub fn source_text_spans(body: &[Stmt]) -> Vec<TextRange> {
    let mut finder = SourceTextSpans::default();
    finder.visit_body(body);
    finder.spans
}

#[derive(Default)]
struct SourceTextSpans {
    spans: Vec<TextRange>,
}

impl<'a> Visitor<'a> for SourceTextSpans {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if matches!(expr, Expr::TString(_)) {
            self.spans.push(expr.range());
        }
        visitor::walk_expr(self, expr);
    }
}

// Rewrites the edits that change a name read in an f-string field with a `=` debug specifier, so the field prints what
// it printed before. The field prints its source text, `x=`, then the value of the expression, so the rewrite keeps that
// text as written and moves the edits into a plain field: `{x=}` becomes `x={x!r}`, and a conversion or format spec
// carries over. `None` when an edit lands where that cannot hold, such as in a t-string field, which keeps its source
// text in the template, or in a field nested in another one.
pub fn keep_debug_text(source: &str, body: &[Stmt], edits: Vec<Edit>) -> Option<Vec<Edit>> {
    let mut finder = DebugFields::default();
    finder.visit_body(body);
    let mut groups: Vec<Vec<Edit>> = vec![Vec::new(); finder.fields.len()];
    let mut rewritten = Vec::new();
    for edit in edits {
        if finder.blocked.iter().any(|range| range.contains_range(edit.range())) {
            return None;
        }
        let mut containing =
            finder.fields.iter().enumerate().filter(|(_, field)| field.range.contains_range(edit.range()));
        match (containing.next(), containing.next()) {
            (None, _) => rewritten.push(edit),
            (Some((index, _)), None) => groups[index].push(edit),
            (Some(_), Some(_)) => return None,
        }
    }
    for (field, edits) in finder.fields.iter().zip(groups) {
        if !edits.is_empty() {
            rewritten.push(Edit::range_replacement(debug_field(source, field, &edits)?, field.range));
        }
    }
    Some(rewritten)
}

// The text of a `=` debug field with its edits applied: the debug text, escaped to a literal, then a plain field.
fn debug_field(source: &str, field: &DebugField, edits: &[Edit]) -> Option<String> {
    let debug = field.debug.as_str();
    if debug.contains(['\n', '\r', '\\']) {
        return None;
    }
    let start = usize::from(field.range.start()) + 1;
    let text_end = start + debug.len();
    let equals = start + debug.rfind('=')?;
    let end = usize::from(field.range.end()) - 1;
    let (mut expression_edits, mut remainder_edits): (Vec<&Edit>, Vec<&Edit>) =
        edits.iter().partition(|edit| usize::from(edit.start()) >= start && usize::from(edit.end()) <= equals);
    let expression = splice(source, start, equals, &mut expression_edits)?;
    let expression = expression.trim_end();
    let remainder = splice(source, text_end, end, &mut remainder_edits)?;
    let conversion = if remainder.is_empty() { "!r" } else { &remainder };
    let space = if expression.starts_with('{') { " " } else { "" };
    let quote = field.quote.as_char();
    let literal = debug.replace('{', "{{").replace('}', "}}").replace(quote, &format!("\\{quote}"));
    Some(format!("{literal}{{{space}{expression}{conversion}}}"))
}

// The source between `from` and `to` with the edits inside it applied. `None` when an edit lies outside the span.
fn splice(source: &str, from: usize, to: usize, edits: &mut [&Edit]) -> Option<String> {
    edits.sort_by_key(|edit| usize::from(edit.start()));
    let mut text = String::new();
    let mut cursor = from;
    for edit in edits.iter() {
        text.push_str(source.get(cursor..usize::from(edit.start()))?);
        text.push_str(edit.content().unwrap_or_default());
        cursor = usize::from(edit.end());
    }
    text.push_str(source.get(cursor..to)?);
    Some(text)
}

// The `=` debug fields of a body, each with its quote and the text of its debug specifier.
#[derive(Default)]
struct DebugFields<'a> {
    fields: Vec<DebugField<'a>>,
    // The debug fields inside a t-string, which the rewrite cannot change.
    blocked: Vec<TextRange>,
}

struct DebugField<'a> {
    range: TextRange,
    debug: &'a ast::DebugText,
    quote: Quote,
}

impl<'a> Visitor<'a> for DebugFields<'a> {
    fn visit_f_string(&mut self, f_string: &'a ast::FString) {
        let quote = f_string.flags.quote_style();
        self.fields.extend(f_string.elements.iter().filter_map(|element| debug_field_of(element, quote)));
        visitor::walk_f_string(self, f_string);
    }

    fn visit_t_string(&mut self, t_string: &'a ast::TString) {
        self.blocked.extend(t_string.elements.iter().filter_map(|element| match element {
            ast::InterpolatedStringElement::Interpolation(interpolation) => {
                interpolation.debug_text.as_ref().map(|_| interpolation.range)
            }
            ast::InterpolatedStringElement::Literal(_) => None,
        }));
        visitor::walk_t_string(self, t_string);
    }
}

fn debug_field_of(element: &ast::InterpolatedStringElement, quote: Quote) -> Option<DebugField<'_>> {
    let ast::InterpolatedStringElement::Interpolation(interpolation) = element else {
        return None;
    };
    Some(DebugField { range: interpolation.range, debug: interpolation.debug_text.as_ref()?, quote })
}

// A module-level `if sys.version_info < ...: raise ...` or `sys.exit(...)` exists to run on interpreters older than the
// target and report them, so fixes must not add syntax or imports those
// interpreters lack.
pub fn has_version_gate(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_if_stmt).any(|gate| {
        any_over_expr(&gate.test, &|expr: &Expr| last_segment(expr) == Some("version_info"))
            && gate.body.last().is_some_and(|last| match last {
                Stmt::Raise(_) => true,
                Stmt::Expr(ast::StmtExpr { value, .. }) => {
                    value.as_call_expr().is_some_and(|call| last_segment(&call.func) == Some("exit"))
                }
                _ => false,
            })
    })
}

// The paragraphs of prose longer than `limit` words, each reported once from its first line to the line that passes
// the limit. Each item is a line's range and word count, or `None` where a line ends the paragraph.
pub(crate) fn long_paragraphs(
    lines: impl IntoIterator<Item = Option<(TextRange, usize)>>,
    limit: usize,
) -> Vec<TextRange> {
    let mut long = Vec::new();
    let mut words = 0;
    let mut first = None;
    for line in lines {
        let Some((range, count)) = line else {
            words = 0;
            first = None;
            continue;
        };
        let start = *first.get_or_insert(range.start());
        let before = words;
        words += count;
        if before <= limit && words > limit {
            long.push(TextRange::new(start, range.end()));
        }
    }
    long
}

// Ruff ranks names isort-style: constants in SCREAMING_SNAKE_CASE first, then CamelCase classes, then the rest. The two
// orders differ in the details: RUF022, sorting `__all__`, takes any name with a letter and no lowercase one for a
// constant and compares case-sensitively, while isort, sorting the names of a `from` import, wants more than one
// character for a constant and ignores case.
pub fn export_key(name: &str) -> (u8, String) {
    let rank = if name.chars().any(char::is_alphabetic) && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_owned())
}

pub fn isort_key(name: &str) -> (u8, String) {
    let rank = if name.len() > 1 && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_ascii_lowercase())
}

// `name = value` or `name: annotation = value`, binding a single plain name.
pub fn single_assignment(stmt: &Stmt) -> Option<(&ast::ExprName, Option<&Expr>, &Expr)> {
    let (target, annotation, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], None, value),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. }) => {
            (target.as_ref(), Some(annotation.as_ref()), value)
        }
        _ => return None,
    };
    Some((target.as_name_expr()?, annotation, value))
}
