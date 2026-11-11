use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::is_bound_around;
use crate::rules::needs_parentheses;
use crate::syntax::argument;

// `codecs.open(filename, mode, encoding, errors, buffering)`, in the order of its parameters.
const PARAMETERS: [&str; 5] = ["filename", "mode", "encoding", "errors", "buffering"];

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(call) = expr else {
        return;
    };
    if !checker.semantic().resolves_to(&call.func, &["codecs", "open"]) {
        return;
    }
    checker.report_fix(
        Rule::CodecsOpen,
        expr.range(),
        "`codecs.open()` is deprecated since Python 3.14; `open()` takes the same arguments",
        fix(checker, call),
        "Call `open()` with `newline=\"\"`",
    );
}

// `codecs.open` splits lines the way `str.splitlines` does, at form feeds and other separators that `open` leaves
// alone, so the rewrite is unsafe. Reads and writes agree on every text the rewrite was probed with.
fn fix(checker: &Checker, call: &ast::ExprCall) -> Option<Fix> {
    let arguments = &call.arguments;
    if arguments.args.iter().any(Expr::is_starred_expr)
        || arguments.keywords.iter().any(|keyword| keyword.arg.is_none())
    {
        return None;
    }
    // A name the rewrite would call `open` could be something else.
    if checker.semantic().is_bound("open")
        || is_bound_around(checker, "open", call.range())
        || checker.has_comment_in(call.range())
    {
        return None;
    }
    let parameter = |index: usize| argument(call, index, PARAMETERS[index]);
    let filename = parameter(0)?;
    // Without an encoding `codecs.open` returns the builtin text file and ignores `errors`.
    let encoding = parameter(2).filter(|encoding| !encoding.is_none_literal_expr())?;
    let mode = parameter(1);
    // A binary mode with an encoding is a `codecs` stream, and `open()` rejects the encoding for it.
    if let Some(mode) = mode {
        let Expr::StringLiteral(literal) = mode else {
            return None;
        };
        if literal.value.to_str().contains('b') {
            return None;
        }
    }
    let mut parameters = vec![checker.text(filename.range()).to_owned()];
    parameters.extend(mode.map(|mode| checker.text(mode.range()).to_owned()));
    parameters.extend(parameter(4).map(|buffering| keyword_argument(checker, call, "buffering", buffering)));
    parameters.push(keyword_argument(checker, call, "encoding", encoding));
    parameters.extend(parameter(3).map(|errors| keyword_argument(checker, call, "errors", errors)));
    parameters.push("newline=\"\"".to_owned());
    let replacement = format!("open({})", parameters.join(", "));
    Some(Fix::unsafe_edit(Edit::range_replacement(replacement, call.range())))
}

// A value that needs parentheses keeps the ones it has in the source, and gets them when it has none, since a walrus,
// a `yield` or a lambda cannot be a keyword value bare where its precedence would end.
fn keyword_argument(checker: &Checker, call: &ast::ExprCall, name: &str, value: &Expr) -> String {
    let range = checker.parenthesized_range(value, (&call.arguments).into());
    let text = checker.text(range);
    if needs_parentheses(value) && range == value.range() {
        format!("{name}=({text})")
    } else {
        format!("{name}={text}")
    }
}
