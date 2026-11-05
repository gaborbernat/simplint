use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::is_name;

// Since Python 3.10 the `traceback` functions read the exception's type and traceback from the exception itself.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    if checker.settings.from_ruff.target_version < PythonVersion::PY310 {
        return;
    }
    let Expr::Call(call) = expr else {
        return;
    };
    let semantic = checker.semantic();
    let Some(function) = ["print_exception", "format_exception", "format_exception_only"]
        .into_iter()
        .find(|function| semantic.resolves_to(&call.func, &["traceback", function]))
    else {
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
    let range = TextRange::new(exc_type.start(), last.end());
    let fix = Fix::safe_edit(Edit::range_replacement(name.to_owned(), range));
    checker.report_fix(
        Rule::TracebackExceptionArguments,
        expr.range(),
        format!("`traceback.{function}` reads the type and traceback from `{name}`; pass `{name}` alone"),
        Some(fix),
        "Drop the type and traceback arguments",
    );
}

// The name `X` of `type(X)` or `X.__class__`.
fn exception_name<'e>(checker: &Checker, expr: &'e Expr) -> Option<&'e str> {
    match expr {
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if checker.semantic().resolves_to(func, &["", "type"])
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
