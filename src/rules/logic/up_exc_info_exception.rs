use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::replacement_unless_commented;

// Since Python 3.11 `sys.exception()` returns the exception being handled, the value `sys.exc_info()` holds in its
// second slot.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    match expr {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) if checker.text(slice.as_ref()) == "1" => {
            let Some(module) = exc_info_module(checker, value) else {
                return;
            };
            report(
                checker,
                expr.range(),
                module,
                "`sys.exc_info()[1]` is the exception being handled; `sys.exception()` returns it directly",
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
            report(
                checker,
                starred.range(),
                module,
                "Unpacking `sys.exc_info()` passes its three values; `sys.exception()` passes the exception alone",
                "Pass `sys.exception()`",
            );
        }
        _ => {}
    }
}

fn exc_info_module<'a>(checker: &Checker<'a>, expr: &Expr) -> Option<&'a str> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    let attribute = func.as_attribute_expr()?;
    (arguments.is_empty() && checker.semantic().resolves_to(func, &["sys", "exc_info"]))
        .then(|| checker.text(attribute.value.as_ref()))
}

fn report(checker: &Checker, range: TextRange, module: &str, message: &str, title: &str) {
    checker.report_fix(
        Rule::ExcInfoException,
        range,
        message,
        replacement_unless_commented(checker, format!("{module}.exception()"), range).map(Fix::safe_edit),
        title,
    );
}
