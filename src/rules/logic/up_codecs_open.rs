use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

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
    if checker.semantic().is_bound("open") || checker.has_comment_in(call.range()) {
        return None;
    }
    let argument = |index: usize| {
        arguments.args.get(index).or_else(|| {
            arguments
                .keywords
                .iter()
                .find(|keyword| keyword.arg.as_ref().is_some_and(|name| name.as_str() == PARAMETERS[index]))
                .map(|keyword| &keyword.value)
        })
    };
    let filename = argument(0)?;
    // Without an encoding `codecs.open` returns the builtin text file and ignores `errors`.
    let encoding = argument(2).filter(|encoding| !encoding.is_none_literal_expr())?;
    let mode = argument(1);
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
    parameters.extend(argument(4).map(|buffering| format!("buffering={}", checker.text(buffering.range()))));
    parameters.push(format!("encoding={}", checker.text(encoding.range())));
    parameters.extend(argument(3).map(|errors| format!("errors={}", checker.text(errors.range()))));
    parameters.push("newline=\"\"".to_owned());
    let replacement = format!("open({})", parameters.join(", "));
    Some(Fix::unsafe_edit(Edit::range_replacement(replacement, call.range())))
}
