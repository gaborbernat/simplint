use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::replacement_unless_commented;
use crate::syntax::is_name;

// Since Python 3.10 the `traceback` functions read the exception's type and traceback from the exception itself.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(name) = checker.semantic().qualified_name(&call.func) else {
        return;
    };
    let Some(function) = (match name.segments() {
        ["traceback", function] => ["print_exception", "format_exception", "format_exception_only"]
            .into_iter()
            .find(|candidate| candidate == function),
        _ => None,
    }) else {
        return;
    };
    let [exc_type, exc, rest @ ..] = call.arguments.args.as_ref() else {
        return;
    };
    let Some(name) = exception_name(checker, exc_type).filter(|name| is_name(exc, name)) else {
        return;
    };
    // `format_exception_only` takes the exception alone, and the other functions also take its traceback, which the
    // exception carries, so only the exception's own traceback can go.
    let last = match rest {
        [] if function == "format_exception_only" => exc,
        [traceback] if function != "format_exception_only" && is_traceback_of(traceback, name) => traceback,
        _ => return,
    };
    let range = TextRange::new(
        checker.parenthesized_range(exc_type, (&call.arguments).into()).start(),
        checker.parenthesized_range(last, (&call.arguments).into()).end(),
    );
    let fix = replacement_unless_commented(checker, name.to_owned(), range).map(Fix::safe_edit);
    checker.report_fix(
        Rule::TracebackExceptionArguments,
        expr.range(),
        format!("`traceback.{function}` reads the type and traceback from `{name}`; pass `{name}` alone"),
        fix,
        "Drop the type and traceback arguments",
    );
}

// The name `X` of `type(X)` or `X.__class__`.
fn exception_name<'e>(checker: &Checker, expr: &'e Expr) -> Option<&'e str> {
    match expr {
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if checker.semantic().is_builtin(func, "type")
                && arguments.args.len() == 1
                && arguments.keywords.is_empty() =>
        {
            arguments.args[0].as_name_expr().map(|name| name.id.as_str())
        }
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "__class__" => {
            value.as_name_expr().map(|name| name.id.as_str())
        }
        _ => None,
    }
}

fn is_traceback_of(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Attribute(ast::ExprAttribute { value, attr, .. })
        if attr.as_str() == "__traceback__" && is_name(value, name))
}
