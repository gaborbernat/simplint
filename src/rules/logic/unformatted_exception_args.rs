use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::builtins::is_exception;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

// The builtin exceptions whose second positional argument is not part of the message: a syntax error's location and
// an exception group's exceptions. Every other builtin exception shows its positional arguments as a tuple.
const STRUCTURED: [&str; 5] = ["SyntaxError", "IndentationError", "TabError", "BaseExceptionGroup", "ExceptionGroup"];

#[derive(Clone, Copy)]
enum Style {
    Percent,
    Braces,
}

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) = stmt else {
        return;
    };
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = exc.as_ref() else {
        return;
    };
    let checker = walker.checker;
    let is_builtin_exception = checker.semantic().qualified_name(func).is_some_and(|name| match name.segments() {
        ["", builtin] => is_exception(builtin, checker.target_version().minor) && !STRUCTURED.contains(builtin),
        _ => false,
    });
    if !is_builtin_exception {
        return;
    }
    let [Expr::StringLiteral(message), rest @ ..] = arguments.args.as_ref() else {
        return;
    };
    if rest.is_empty() || rest.iter().any(Expr::is_starred_expr) {
        return;
    }
    let text = message.value.to_str();
    let Some((style, count)) = placeholders(text) else {
        return;
    };
    let shown = match style {
        Style::Percent => "`%` placeholders",
        Style::Braces => "`{}` fields",
    };
    let fix = (count == rest.len() && !message.value.is_implicit_concatenated())
        .then(|| format_fix(checker, style, message.range(), rest))
        .flatten();
    checker.report_fix(
        Rule::UnformattedExceptionArgs,
        exc.range(),
        format!(
            "The message has {shown}, but the exception keeps its arguments apart and never fills them, so it shows a tuple"
        ),
        fix,
        "Format the message before passing it",
    );
}

// The placeholders of one style the message holds, `None` when it has none or mixes named and positional ones the fix
// cannot fill from a tuple.
fn placeholders(text: &str) -> Option<(Style, usize)> {
    let percent = percent_placeholders(text);
    if let Some(count) = percent
        && count > 0
    {
        return Some((Style::Percent, count));
    }
    let braces = brace_fields(text)?;
    (braces > 0).then_some((Style::Braces, braces))
}

// `%s`, `%d`, `%5.2f` and the other conversions, without `%%`. A `%(name)s` mapping key makes the count unusable.
fn percent_placeholders(text: &str) -> Option<usize> {
    let mut count = 0;
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        if char != '%' {
            continue;
        }
        if chars.peek() == Some(&'%') {
            chars.next();
            continue;
        }
        if chars.peek() == Some(&'(') {
            return None;
        }
        while chars.peek().is_some_and(|next| "#0-+*.123456789hlL".contains(*next)) {
            chars.next();
        }
        if chars.next().is_some_and(|conversion| "diouxXeEfFgGcrsa".contains(conversion)) {
            count += 1;
        } else {
            return Some(0);
        }
    }
    Some(count)
}

// Auto-numbered `{}` fields, with an optional `!r` conversion or `:` spec; `{{` and `}}` are literal braces. A numbered
// or named field makes the count unusable.
fn brace_fields(text: &str) -> Option<usize> {
    let mut count = 0;
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        match char {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '{' => {
                if !matches!(chars.peek(), Some('}' | '!' | ':')) {
                    return None;
                }
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                }
                count += 1;
            }
            _ => {}
        }
    }
    Some(count)
}

fn format_fix(checker: &Checker, style: Style, message: TextRange, rest: &[Expr]) -> Option<Fix> {
    let range = message.cover(rest.last()?.range());
    if checker.has_comment_in(range) {
        return None;
    }
    let values = rest.iter().map(|value| checker.text(value)).collect::<Vec<_>>();
    let literal = checker.text(message);
    let replacement = match style {
        Style::Percent if values.len() == 1 => format!("{literal} % ({},)", values[0]),
        Style::Percent => format!("{literal} % ({})", values.join(", ")),
        Style::Braces => format!("{literal}.format({})", values.join(", ")),
    };
    Some(Fix::unsafe_edit(Edit::range_replacement(replacement, range)))
}
