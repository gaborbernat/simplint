use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::{self as ast, Expr, Operator, PythonVersion, StringFlags};
use ruff_python_codegen::Generator;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::BinOp(ast::ExprBinOp { op: Operator::Add, .. }) = expr else {
        return;
    };
    // The `+` this one is the left side of reports the whole chain, unless what it adds is bytes.
    if let Some(Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. })) = walker.expression_parent()
        && std::ptr::eq(left.as_ref(), expr)
        && !right.is_bytes_literal_expr()
    {
        return;
    }
    let mut operands = Vec::new();
    flatten(expr, &mut operands);
    if !operands.iter().any(|operand| matches!(operand, Expr::StringLiteral(_) | Expr::FString(_)))
        || operands.iter().any(|operand| operand.is_bytes_literal_expr())
    {
        return;
    }
    let checker = walker.checker;
    let range = expr.range();
    // A comment between the operands has no place in the f-string, so rewriting would drop it.
    let fix = fstring(checker, &operands, walker.in_fstring())
        .filter(|_| !checker.has_comment_in(range))
        .map(|text| Fix::unsafe_edit(Edit::range_replacement(text, range)));
    checker.report_fix(
        Rule::StringConcatenation,
        range,
        "Build text with an f-string instead of `+`",
        fix,
        "Replace the concatenation with an f-string",
    );
}

fn flatten<'a>(expr: &'a Expr, operands: &mut Vec<&'a Expr>) {
    match expr {
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. }) => {
            flatten(left, operands);
            operands.push(right);
        }
        other => operands.push(other),
    }
}

fn fstring(checker: &Checker, operands: &[&Expr], nested: bool) -> Option<String> {
    // Each string part's quote and the text between its quotes.
    let mut parts: Vec<(Quote, &str)> = Vec::new();
    // The f-string's text, with each interpolated expression, its source text and conversion still to place.
    let mut pieces: Vec<Result<String, (&Expr, &str, &str)>> = Vec::new();
    let mut interpolated = false;
    for operand in operands {
        match operand {
            // An implicit concatenation, which `ruff format` may join into one literal, joins here part by part.
            Expr::StringLiteral(string) => {
                for literal in &string.value {
                    let text = literal_text(checker, literal)?;
                    parts.push((literal.flags.quote_style(), text));
                    pieces.push(Ok(text.replace('{', "{{").replace('}', "}}")));
                }
            }
            Expr::FString(ast::ExprFString { value, .. }) => {
                for part in value {
                    match part {
                        ast::FStringPartRef::Literal(literal) => {
                            let text = literal_text(checker, literal)?;
                            parts.push((literal.flags.quote_style(), text));
                            pieces.push(Ok(text.replace('{', "{{").replace('}', "}}")));
                        }
                        ast::FStringPartRef::FString(fstring) => {
                            if fstring.flags.is_triple_quoted() || fstring.flags.prefix().is_raw() {
                                return None;
                            }
                            let text = checker.text(TextRange::new(
                                fstring.start() + fstring.flags.opener_len(),
                                fstring.end() - fstring.flags.closer_len(),
                            ));
                            parts.push((fstring.flags.quote_style(), text));
                            pieces.push(Ok(text.to_owned()));
                        }
                    }
                }
                interpolated = true;
            }
            other => {
                // `str(x)`, `repr(x)` and `ascii(x)` become the conversions `!s`, `!r` and `!a`, as Ruff's RUF010 asks.
                let (inner, conversion) = match *other {
                    Expr::Call(ast::ExprCall { func, arguments, .. })
                        if arguments.keywords.is_empty()
                            && arguments.args.len() == 1
                            && !arguments.args[0].is_starred_expr() =>
                    {
                        match func.as_name_expr().map(|name| name.id.as_str()) {
                            Some(builtin @ ("str" | "repr" | "ascii")) if !checker.semantic().is_bound(builtin) => {
                                (&arguments.args[0], &builtin[..1])
                            }
                            _ => (*other, ""),
                        }
                    }
                    _ => (*other, ""),
                };
                let text = checker.text(inner.range());
                if text.contains(FORBIDDEN_IN_EXPRESSION) {
                    return None;
                }
                pieces.push(Err((inner, text, conversion)));
                interpolated = true;
            }
        }
    }
    // A part in the other quote style fits only when its text holds no delimiter, escaped or not.
    // Before Python 3.12 a string inside an f-string's replacement field cannot use that f-string's quote either, so
    // there the parts keep the one quote they share and their expressions stay as written.
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    let as_written = nested && before_312;
    let preferred = checker.stylist.quote();
    let alternatives = if as_written { vec![] } else { vec![preferred, preferred.opposite()] };
    let delimiter = parts.first().map(|(quote, _)| *quote).into_iter().chain(alternatives).find(|delimiter| {
        parts.iter().all(|(quote, text)| quote == delimiter || !as_written && !text.contains(delimiter.as_char()))
    })?;
    let mut content = String::new();
    for piece in pieces {
        match piece {
            Ok(text) => content.push_str(&text),
            Err((expr, text, conversion)) => {
                content.push('{');
                content.push_str(&expression_text(checker, expr, text, delimiter, as_written)?);
                if !conversion.is_empty() {
                    content.push('!');
                    content.push_str(conversion);
                }
                content.push('}');
            }
        }
    }
    let prefix = if interpolated { "f" } else { "" };
    let text = if interpolated { content } else { content.replace("{{", "{").replace("}}", "}") };
    let delimiter = delimiter.as_char();
    Some(format!("{prefix}{delimiter}{text}{delimiter}"))
}

// The text of an expression as it goes into a replacement field of an f-string quoted with `delimiter`.
fn expression_text(checker: &Checker, expr: &Expr, text: &str, delimiter: Quote, as_written: bool) -> Option<String> {
    // Before Python 3.12 an f-string expression cannot reuse the string's own quote, so its string literals take the
    // other one, as `ruff format` leaves them once the f-string holds them.
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    let requote = before_312 && text.contains(delimiter.as_char());
    if requote && as_written {
        return None;
    }
    // An expression the layout spreads over lines goes on one, as it would in a line long enough.
    if !requote && !text.contains('\n') {
        return Some(text.to_owned());
    }
    let mut regenerated = expr.clone();
    if requote {
        Requote(delimiter.opposite()).visit_expr(&mut regenerated);
    }
    let text = Generator::from(checker.stylist).expr(&regenerated);
    let unfit = text.contains(FORBIDDEN_IN_EXPRESSION) || before_312 && text.contains(delimiter.as_char());
    (!unfit).then_some(text)
}

// A comment, a backslash, a format spec or conversion, or nested braces in an expression would not survive the move into
// a replacement field.
const FORBIDDEN_IN_EXPRESSION: [char; 6] = ['\\', '#', ':', '!', '{', '}'];

// A named escape such as `\N{BULLET}` keeps single braces in an f-string, where every other brace doubles.
fn literal_text<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<&'a str> {
    let flags = literal.flags;
    let text = checker.text(literal.content_range());
    (!flags.is_triple_quoted() && !flags.prefix().is_raw() && !flags.prefix().is_unicode() && !text.contains("\\N{"))
        .then_some(text)
}

struct Requote(Quote);

impl Transformer for Requote {
    fn visit_expr(&self, expr: &mut Expr) {
        if let Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) = expr {
            for literal in value.iter_mut() {
                literal.flags = literal.flags.with_quote_style(self.0);
            }
        }
        transformer::walk_expr(self, expr);
    }
}
