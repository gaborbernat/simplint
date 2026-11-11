use std::borrow::Cow;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::{self as ast, Expr, Operator, PythonVersion};
use ruff_python_codegen::Generator;
use ruff_python_stdlib::logging::LoggingLevel;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::inline::{fstring_body_parts, requoted_text};
use crate::rules::logic::kinds::Kinds;

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
        || left_to_ruff(walker, expr)
    {
        return;
    }
    let checker = walker.checker;
    let range = expr.range();
    // `+` calls an operand's own `__add__` or `__radd__`, and raises `TypeError` for one that is not text, where the
    // f-string formats any value: the two agree only when every operand is surely a `str`.
    let mut kinds = Kinds::new(walker, range);
    // A class may override `+` on its own, as `markupsafe.Markup` escapes its operand, so the f-string would change
    // what the code does. An import from outside the run does not say whether its name is a class, so it counts by
    // its name.
    if operands.iter().any(|operand| kinds.is_class_instance(operand)) {
        return;
    }
    // A comment between the operands has no place in the f-string, so rewriting would drop it.
    let fix = match fstring(checker, &operands, walker.in_fstring()) {
        Err(Unwritable::Impossible) => return,
        written => written
            .ok()
            .filter(|_| !checker.has_comment_in(range) && operands.iter().all(|operand| kinds.is_str(operand)))
            .map(|text| Fix::unsafe_edit(Edit::range_replacement(text, range))),
    };
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

// Ruff's G003 reports the `+` that is a logging call's message, and its ISC003 a concatenation of two literals across
// lines that it can fix, so this rule leaves those to Ruff. A pair with a parenthesized operand gets no fix from
// ISC003, so this rule keeps it.
fn left_to_ruff(walker: &Walker, expr: &Expr) -> bool {
    is_logging_message(walker, expr)
        || (walker.checker.settings.from_ruff.allow_multiline && ruff_fixes_literal_pair_at_start(walker.checker, expr))
}

// As G003 finds the message: the `msg` keyword, or the argument at the message's position, which `log` moves after
// the level.
fn is_logging_message(walker: &Walker, expr: &Expr) -> bool {
    let Some(Expr::Call(call)) = walker.expression_parent() else {
        return false;
    };
    let semantic = walker.checker.semantic();
    let logger_objects = &walker.checker.settings.from_ruff.logger_objects;
    let position = match call.func.as_ref() {
        Expr::Attribute(ast::ExprAttribute { attr, .. })
            if semantic.is_logger_candidate(&call.func, logger_objects) =>
        {
            message_position(attr.as_str())
        }
        func @ Expr::Name(_) => semantic.qualified_name(func).and_then(|name| match name.segments() {
            ["logging", attribute] => message_position(attribute),
            _ => None,
        }),
        _ => None,
    };
    position.is_some_and(|position| {
        call.arguments.find_argument_value("msg", position).is_some_and(|message| std::ptr::eq(message, expr))
    })
}

fn message_position(attribute: &str) -> Option<usize> {
    match attribute {
        "log" => Some(1),
        _ => LoggingLevel::from_attribute(attribute).map(|_| 0),
    }
}

// Whether ISC003 reports a `+` that starts where `expr` starts, on the way down its first operands, and gives it a fix:
// the two operands of such a `+` are the same kind of literal, a line break separates them, and neither is in
// parentheses. Its report and this rule's would land on one location only then.
fn ruff_fixes_literal_pair_at_start(checker: &Checker, expr: &Expr) -> bool {
    let mut current = expr;
    loop {
        current = match current {
            Expr::BinOp(ast::ExprBinOp { left, op, right, .. }) => {
                let same_literals = matches!(
                    (left.as_ref(), right.as_ref()),
                    (Expr::StringLiteral(_) | Expr::FString(_), Expr::StringLiteral(_) | Expr::FString(_))
                        | (Expr::BytesLiteral(_), Expr::BytesLiteral(_))
                        | (Expr::TString(_), Expr::TString(_))
                );
                if matches!(op, Operator::Add)
                    && same_literals
                    && current.start() == expr.start()
                    && checker.source.contains_line_break(TextRange::new(left.end(), right.start()))
                {
                    return !is_parenthesized(checker, left, current) && !is_parenthesized(checker, right, current);
                }
                left
            }
            Expr::Attribute(ast::ExprAttribute { value, .. }) | Expr::Subscript(ast::ExprSubscript { value, .. }) => {
                value
            }
            Expr::Call(ast::ExprCall { func, .. }) => func,
            _ => return false,
        };
    }
}

fn is_parenthesized(checker: &Checker, operand: &Expr, parent: &Expr) -> bool {
    checker.parenthesized_range(operand, parent.into()) != operand.range()
}

// Why no f-string is written for a chain. Before Python 3.12 a replacement field cannot hold a backslash inside a
// string, so such a chain has no f-string for that target (`Impossible`) and is not reported. Any other chain this rule
// does not write is `Declined`, reported without a fix.
enum Unwritable {
    Declined,
    Impossible,
}

fn fstring(checker: &Checker, operands: &[&Expr], nested: bool) -> Result<String, Unwritable> {
    // Each string part's quote and the text between its quotes.
    let mut parts: Vec<(Quote, Cow<str>)> = Vec::new();
    // The f-string's text, with each interpolated expression, its source text and conversion still to place.
    let mut pieces = Vec::new();
    let mut interpolated = false;
    // A part or an expression the rule does not write leaves the chain reported without a fix, but a later string with
    // a backslash still makes it impossible, so the loop goes on.
    let mut declined = false;
    let before_312 = checker.target_version() < PythonVersion::PY312;
    for operand in operands {
        match operand {
            // An implicit concatenation, which `ruff format` may join into one literal, joins here part by part.
            Expr::StringLiteral(_) | Expr::FString(_) => {
                let Some(body) = fstring_body_parts(checker, operand) else {
                    declined = true;
                    continue;
                };
                for (quote, text) in body {
                    parts.push((quote, text.clone()));
                    pieces.push(Ok(text));
                }
                interpolated |= operand.is_f_string_expr();
            }
            other => {
                // Before Python 3.12 a string inside a replacement field cannot hold a backslash, so a string with one
                // in an expression leaves no f-string for the chain. A line continuation outside any string is not one.
                if before_312
                    && any_over_expr(other, &|sub: &Expr| {
                        matches!(sub, Expr::StringLiteral(_) | Expr::BytesLiteral(_) | Expr::FString(_))
                            && checker.text(sub.range()).contains('\\')
                    })
                {
                    return Err(Unwritable::Impossible);
                }
                // Ruff's RUF010 asks for the conversions `!s`, `!r` and `!a` in place of `str(x)`, `repr(x)` and
                // `ascii(x)`.
                let (inner, conversion) = match *other {
                    Expr::Call(ast::ExprCall { func, arguments, .. })
                        if arguments.keywords.is_empty()
                            && arguments.args.len() == 1
                            && !arguments.args[0].is_starred_expr() =>
                    {
                        match ["str", "repr", "ascii"]
                            .into_iter()
                            .find(|builtin| checker.semantic().is_builtin(func, builtin))
                        {
                            Some(builtin) => (&arguments.args[0], &builtin[..1]),
                            _ => (*other, ""),
                        }
                    }
                    _ => (*other, ""),
                };
                let text = checker.text(inner.range());
                if text.contains(FORBIDDEN_IN_EXPRESSION) {
                    declined = true;
                    continue;
                }
                pieces.push(Err((inner, text, conversion)));
                interpolated = true;
            }
        }
    }
    if declined {
        return Err(Unwritable::Declined);
    }
    // A part in the other quote style fits only when its text holds no delimiter, escaped or not.
    // Before Python 3.12 a string inside an f-string's replacement field cannot use that f-string's quote either, so
    // there the parts keep the one quote they share and their expressions stay as written.
    let as_written = nested && before_312;
    let preferred = checker.stylist.quote();
    let delimiter = parts
        .first()
        .map(|(quote, _)| *quote)
        .into_iter()
        .chain(if as_written { vec![] } else { vec![preferred, preferred.opposite()] })
        .find(|delimiter| {
            parts.iter().all(|(quote, text)| quote == delimiter || !as_written && !text.contains(delimiter.as_char()))
        })
        .ok_or(Unwritable::Declined)?;
    let mut content = String::new();
    for piece in pieces {
        match piece {
            Ok(text) => content.push_str(&text),
            Err((expr, text, conversion)) => {
                content.push('{');
                content.push_str(
                    &expression_text(checker, expr, text, delimiter, as_written).ok_or(Unwritable::Declined)?,
                );
                if !conversion.is_empty() {
                    content.push('!');
                    content.push_str(conversion);
                }
                content.push('}');
            }
        }
    }
    let delimiter = delimiter.as_char();
    let (prefix, text) =
        if interpolated { ("f", content) } else { ("", content.replace("{{", "{").replace("}}", "}")) };
    Ok(format!("{prefix}{delimiter}{text}{delimiter}"))
}

// The text of an expression as it goes into a replacement field of an f-string quoted with `delimiter`.
fn expression_text(checker: &Checker, expr: &Expr, text: &str, delimiter: Quote, as_written: bool) -> Option<String> {
    // Before Python 3.12 an f-string expression cannot reuse the string's own quote, so its string literals take the
    // other one, as `ruff format` leaves them once the f-string holds them.
    let before_312 = checker.target_version() < PythonVersion::PY312;
    let requote = before_312 && text.contains(delimiter.as_char());
    if requote && as_written {
        return None;
    }
    // An expression the layout spreads over lines goes on one, as it would in a line long enough.
    if !requote && !text.contains('\n') {
        // A replacement field takes only a yield or star expression unparenthesized, so a generator the call took bare
        // gets the parentheses its own syntax would give it.
        return Some(if matches!(expr, Expr::Generator(ast::ExprGenerator { parenthesized: false, .. })) {
            format!("({text})")
        } else {
            text.to_owned()
        });
    }
    let text =
        if requote { requoted_text(checker, expr, delimiter)? } else { Generator::from(checker.stylist).expr(expr) };
    (!(text.contains(FORBIDDEN_IN_EXPRESSION) || before_312 && text.contains(delimiter.as_char()))).then_some(text)
}

// A comment, a backslash, a format spec or conversion, or nested braces in an expression would not survive the move
// into a replacement field.
const FORBIDDEN_IN_EXPRESSION: [char; 6] = ['\\', '#', ':', '!', '{', '}'];
