use ruff_diagnostics::{Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::str::Quote;
use ruff_python_ast::{self as ast, Expr, Operator, PythonVersion, StringFlags};
use ruff_python_codegen::Generator;
use ruff_python_stdlib::logging::LoggingLevel;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::inline::{literal_text, requoted_text};
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
    let texts = operands.iter().all(|operand| kinds.is_str(operand));
    // A comment between the operands has no place in the f-string, so rewriting would drop it.
    let fix = fstring(checker, &operands, walker.in_fstring())
        .filter(|_| texts && !checker.has_comment_in(range))
        .map(|text| Fix::unsafe_edit(Edit::range_replacement(text, range)));
    checker.report_fix(
        Rule::StringConcatenation,
        range,
        "Build text with an f-string instead of `+`",
        fix,
        "Replace the concatenation with an f-string",
    );
}

// Ruff's G003 reports the `+` that is a logging call's message, and its ISC003 a concatenation of two literals across
// lines, so this rule leaves those to Ruff while it runs.
fn left_to_ruff(walker: &Walker, expr: &Expr) -> bool {
    let settings = &walker.checker.settings.from_ruff;
    (settings.ruff_runs(RuffRule::LoggingStringConcat) && is_logging_message(walker, expr))
        || (settings.allow_multiline
            && settings.ruff_runs(RuffRule::ExplicitStringConcatenation)
            && starts_with_literal_pair_across_lines(walker.checker, expr))
}

// Whether `expr` is the message argument of a logging call, as G003 finds it: the `msg` keyword or the argument at
// the message's position, which `log` moves after the level.
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

// Whether ISC003 reports a `+` that starts where `expr` starts, on the way down its first operands: the two operands
// of such a `+` are the same kind of literal and a line break separates them. Its report and this rule's would land on
// one location only then.
fn starts_with_literal_pair_across_lines(checker: &Checker, expr: &Expr) -> bool {
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
                    return true;
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
    // A replacement field takes only a yield or star expression unparenthesized, so a generator the call took bare
    // gets the parentheses its own syntax would give it.
    let bare_generator = matches!(expr, Expr::Generator(ast::ExprGenerator { parenthesized: false, .. }));
    // An expression the layout spreads over lines goes on one, as it would in a line long enough.
    if !requote && !text.contains('\n') {
        return Some(if bare_generator { format!("({text})") } else { text.to_owned() });
    }
    let text =
        if requote { requoted_text(checker, expr, delimiter)? } else { Generator::from(checker.stylist).expr(expr) };
    let unfit = text.contains(FORBIDDEN_IN_EXPRESSION) || before_312 && text.contains(delimiter.as_char());
    (!unfit).then_some(text)
}

// A comment, a backslash, a format spec or conversion, or nested braces in an expression would not survive the move into
// a replacement field.
const FORBIDDEN_IN_EXPRESSION: [char; 6] = ['\\', '#', ':', '!', '{', '}'];
