use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

// Since Python 3.11 `sys.exception()` returns the exception being handled, the value `sys.exc_info()` holds in its
// second slot.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    match expr {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) if checker.text(slice.as_ref()) == "1" => {
            let Some(module) = exc_info_module(checker, value) else {
                return;
            };
            let fix = (!checker.has_comment_in(expr.range()))
                .then(|| Fix::safe_edit(Edit::range_replacement(format!("{module}.exception()"), expr.range())));
            checker.report_fix(
                Rule::ExcInfoException,
                expr.range(),
                "`sys.exc_info()[1]` is the exception being handled; `sys.exception()` returns it directly",
                fix,
                "Use `sys.exception()`",
            );
        }
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if checker.semantic().resolves_to(func, &["traceback", "print_exception"]) =>
        {
            let [starred @ Expr::Starred(ast::ExprStarred { value, .. })] = arguments.args.as_ref() else {
                return;
            };
            let Some(module) = exc_info_module(checker, value) else {
                return;
            };
            let fix = (!checker.has_comment_in(starred.range()))
                .then(|| Fix::safe_edit(Edit::range_replacement(format!("{module}.exception()"), starred.range())));
            checker.report_fix(
                Rule::ExcInfoException,
                starred.range(),
                "Unpacking `sys.exc_info()` passes its three values; `sys.exception()` passes the exception alone",
                fix,
                "Pass `sys.exception()`",
            );
        }
        _ => {}
    }
}

// The module text `X` of a call `X.exc_info()` that resolves to `sys.exc_info`.
fn exc_info_module<'a>(checker: &Checker<'a>, expr: &Expr) -> Option<&'a str> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    let attribute = func.as_attribute_expr()?;
    let no_arguments = arguments.args.is_empty() && arguments.keywords.is_empty();
    (no_arguments && checker.semantic().resolves_to(func, &["sys", "exc_info"]))
        .then(|| checker.text(attribute.value.as_ref()))
}
