use std::borrow::Cow;

use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::{self as ast, Expr, StringFlags};
use ruff_python_codegen::Generator;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::edits::escape_braces;

// The expression's text with every string in it in the other quote than `delimiter`, for a replacement field before
// Python 3.12 that cannot hold `delimiter`. `None` when a string cannot change quote, or the text still holds
// `delimiter` or a backslash.
pub(in crate::rules::logic) fn requoted_text(checker: &Checker, expr: &Expr, delimiter: Quote) -> Option<String> {
    if any_over_expr(expr, &|sub: &Expr| unquotable(checker, sub)) {
        return None;
    }
    let mut regenerated = expr.clone();
    Requote(delimiter.opposite()).visit_expr(&mut regenerated);
    let text = Generator::from(checker.stylist).expr(&regenerated);
    (!text.contains(delimiter.as_char()) && !text.contains('\\')).then_some(text)
}

// A triple-quoted, raw or `\N{` string keeps its form only in its own quotes, so it cannot change quote. Neither can an
// f-string whose replacement field holds a string: every string of the value takes the other quote, so the field would
// hold the quote its f-string is written in, which Python before 3.12 does not allow.
fn unquotable(checker: &Checker, expr: &Expr) -> bool {
    match expr {
        Expr::StringLiteral(_) => fstring_body_parts(checker, expr).is_none(),
        Expr::FString(ast::ExprFString { value, .. }) => {
            fstring_body_parts(checker, expr).is_none()
                || value.elements().any(|element| match element {
                    ast::InterpolatedStringElement::Interpolation(interpolation) => {
                        any_over_expr(&interpolation.expression, &|sub: &Expr| {
                            matches!(sub, Expr::StringLiteral(_) | Expr::FString(_))
                        })
                    }
                    ast::InterpolatedStringElement::Literal(_) => false,
                })
        }
        _ => false,
    }
}

fn literal_part<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<(Quote, Cow<'a, str>)> {
    Some((literal.flags.quote_style(), Cow::Owned(escape_braces(literal_text(checker, literal)?))))
}

// A named escape such as `\N{BULLET}` keeps single braces in an f-string, where every other brace doubles.
fn literal_text<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<&'a str> {
    let flags = literal.flags;
    let text = checker.text(literal.content_range());
    (!flags.is_triple_quoted() && !flags.prefix().is_raw() && !flags.prefix().is_unicode() && !text.contains("\\N{"))
        .then_some(text)
}

// The parts of a string or f-string value as an f-string body takes them, each with its quote: a literal's text with its
// braces doubled, and an f-string's text between its quotes as it stands. `None` when a part cannot move into an
// f-string body, which is a literal `literal_text` refuses or a raw or triple-quoted f-string.
pub(in crate::rules::logic) fn fstring_body_parts<'a>(
    checker: &Checker<'a>,
    value: &Expr,
) -> Option<Vec<(Quote, Cow<'a, str>)>> {
    match value {
        Expr::StringLiteral(string) => string.value.iter().map(|literal| literal_part(checker, literal)).collect(),
        Expr::FString(ast::ExprFString { value, .. }) => value
            .iter()
            .map(|part| match part {
                ast::FStringPartRef::Literal(literal) => literal_part(checker, literal),
                ast::FStringPartRef::FString(fstring) => {
                    if fstring.flags.is_triple_quoted() || fstring.flags.prefix().is_raw() {
                        return None;
                    }
                    let content = TextRange::new(
                        fstring.start() + fstring.flags.opener_len(),
                        fstring.end() - fstring.flags.closer_len(),
                    );
                    Some((fstring.flags.quote_style(), Cow::Borrowed(checker.text(content))))
                }
            })
            .collect(),
        _ => None,
    }
}

struct Requote(Quote);

impl Transformer for Requote {
    fn visit_expr(&self, expr: &mut Expr) {
        match expr {
            Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
                for literal in value.iter_mut() {
                    literal.flags = literal.flags.with_quote_style(self.0);
                }
            }
            Expr::FString(ast::ExprFString { value, .. }) => {
                for part in value.iter_mut() {
                    match part {
                        ast::FStringPartMut::Literal(literal) => literal.flags = literal.flags.with_quote_style(self.0),
                        ast::FStringPartMut::FString(fstring) => fstring.flags = fstring.flags.with_quote_style(self.0),
                    }
                }
            }
            _ => {}
        }
        transformer::walk_expr(self, expr);
    }
}
