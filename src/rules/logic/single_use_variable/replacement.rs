use ruff_python_ast::str::Quote;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, PythonVersion, Stmt, StringFlags};
use ruff_text_size::{Ranged, TextRange};

use super::reading::ReadSite;
use crate::layout::joined;
use crate::lint::checker::Checker;
use crate::rules::logic::inline::{fstring_body_parts, inlined_text, requoted_text};
use crate::syntax::needs_parentheses;

pub(super) fn inlined_replacement(
    checker: &Checker,
    assign: &ast::StmtAssign,
    next: &Stmt,
    (use_range, reading): (TextRange, &ReadSite<'_>),
) -> Option<(TextRange, String)> {
    let value = assign.value.as_ref();
    let before_312 = checker.target_version() < PythonVersion::PY312;
    if let Some(field) = reading.bare_field
        && let Some(text) = spliced_text(
            checker,
            value,
            reading.field_fstring,
            &reading.fstring_quotes,
            reading.in_format_spec,
            before_312,
        )
    {
        return Some((field, text));
    }
    // The formatter drops the parentheses around an assigned value, so the use does not get them either, and
    // `inlined_text` puts them back where the value needs them. A value over several lines keeps them outside brackets,
    // where its line breaks would end the statement.
    let value_range = if checker.text(value.range()).contains(['\n', '\r']) && !in_brackets(checker, next, use_range) {
        checker.parenthesized_range(value, assign.into())
    } else {
        value.range()
    };
    let value_text = checker.text(value_range);
    // A replacement field cannot hold a backslash or a line break before Python 3.12 (PEP 701), so a value the source
    // spreads over lines goes in on one line, as `ruff format` joins it once the assignment is gone.
    let field_before_312 = before_312 && !reading.fstring_quotes.is_empty();
    if field_before_312 && value_text.contains('\\') {
        return None;
    }
    let joined =
        if field_before_312 && value_text.contains(['\n', '\r']) { Some(joined(checker, value_text)?) } else { None };
    // Before Python 3.9 a decorator is a dotted name, optionally called, so the value must be one.
    if checker.target_version() < PythonVersion::PY39 && reading.decorator_head && !is_dotted_name(value) {
        return None;
    }
    // A whole argument, return or assigned value needs no parentheses, though a tuple still does.
    let parenthesize = needs_parentheses_at(value, reading.slot) && (!reading.whole || value.is_tuple_expr());
    let mut replacement = field_value(
        checker,
        (value, value_range, joined.as_deref()),
        parenthesize,
        &reading.fstring_quotes,
        before_312,
    )?;
    // A value that starts with `{` would make the field's `{` an escaped `{{`, so the value gets a space.
    if reading.field_start
        && replacement.starts_with('{')
        && checker.source[..use_range.start().to_usize()].ends_with('{')
    {
        replacement = format!(" {replacement} ");
    }
    Some((use_range, replacement))
}

// `None` where the field's f-string cannot hold the text.
fn spliced_text(
    checker: &Checker,
    value: &Expr,
    outer: Option<ast::FStringFlags>,
    quotes: &[Quote],
    in_format_spec: bool,
    before_312: bool,
) -> Option<String> {
    let outer = outer?;
    if outer.prefix().is_raw() {
        return None;
    }
    let text: String = fstring_body_parts(checker, value)?.into_iter().map(|(_, text)| text).collect();
    let splices_wrongly = text.contains(outer.quote_style().as_char())
        || (before_312 && (text.contains('\\') || quotes.iter().any(|quote| text.contains(quote.as_char()))))
        || (in_format_spec && text.contains(['{', '}']));
    (!splices_wrongly).then_some(text)
}

fn in_brackets(checker: &Checker, next: &Stmt, use_range: TextRange) -> bool {
    let tokens = checker.parsed.tokens().in_range(TextRange::new(next.start(), use_range.start()));
    let opened =
        tokens.iter().filter(|token| matches!(token.kind(), TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace));
    let closed =
        tokens.iter().filter(|token| matches!(token.kind(), TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace));
    opened.count() > closed.count()
}

fn is_dotted_name(expr: &Expr) -> bool {
    match expr {
        Expr::Name(_) => true,
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_dotted_name(value),
        _ => false,
    }
}

// Where the read is in no operator, `needs_parentheses` decides.
fn needs_parentheses_at(value: &Expr, slot: Option<OperatorPrecedence>) -> bool {
    match slot {
        Some(required) => {
            OperatorPrecedence::from_expr(value) < required
                || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
        }
        None => needs_parentheses(value),
    }
}

// The value as it goes into the statement. Before Python 3.12 a replacement field cannot hold the quote of the f-string
// around it, so a value that holds it takes the other quote in its strings, which needs every such f-string to share
// one quote.
fn field_value(
    checker: &Checker,
    (value, value_range, joined): (&Expr, TextRange, Option<&str>),
    parenthesize: bool,
    quotes: &[Quote],
    before_312: bool,
) -> Option<String> {
    let value_text = joined.unwrap_or_else(|| checker.text(value_range));
    if !(before_312 && quotes.iter().any(|quote| value_text.contains(quote.as_char()))) {
        return Some(match joined {
            Some(text) if parenthesize => format!("({text})"),
            Some(text) => text.to_owned(),
            None => inlined_text(checker, value, value_range, parenthesize),
        });
    }
    let quote = quotes.first().filter(|first| quotes.iter().all(|quote| quote == *first))?;
    let text = requoted_text(checker, value, *quote)?;
    Some(if parenthesize { format!("({text})") } else { text })
}
