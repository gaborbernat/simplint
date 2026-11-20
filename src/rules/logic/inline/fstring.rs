use std::borrow::Cow;

use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::{self as ast, Expr, StringFlags};
use ruff_python_codegen::Generator;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::edits::escape_braces;

// Before Python 3.12 a quote can terminate the enclosing f-string inside a replacement field.
pub(in crate::rules::logic) fn requoted_text(checker: &Checker, expr: &Expr, delimiter: Quote) -> Option<String> {
    if any_over_expr(expr, &|sub: &Expr| unquotable(checker, sub)) {
        return None;
    }
    let mut regenerated = expr.clone();
    Requote(delimiter.opposite()).visit_expr(&mut regenerated);
    let text = Generator::from(checker.stylist).expr(&regenerated);
    (!text.contains(delimiter.as_char()) && !text.contains('\\')).then_some(text)
}

// Keep forms the generator cannot safely requote. Giving nested strings a shared delimiter can close an f-string
// early on Python versions before 3.12.
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

// Preserve each part's quote style when moving adjacent literals into an f-string.
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

fn literal_part<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<(Quote, Cow<'a, str>)> {
    Some((literal.flags.quote_style(), Cow::Owned(escape_braces(literal_text(checker, literal)?))))
}

// Doubling the braces would break named escapes such as `\N{BULLET}`.
fn literal_text<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<&'a str> {
    let flags = literal.flags;
    let text = checker.text(literal.content_range());
    (!flags.is_triple_quoted() && !flags.prefix().is_raw() && !flags.prefix().is_unicode() && !text.contains("\\N{"))
        .then_some(text)
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
