use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::{ensure_import, with_import};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return;
    };
    let checker = walker.checker;
    if !checker.semantic().resolves_to(func, &["asyncio", "iscoroutinefunction"]) {
        return;
    }
    checker.report_fix(
        Rule::AsyncioIsCoroutineFunction,
        expr.range(),
        "`asyncio.iscoroutinefunction()` is deprecated since Python 3.14 and also accepts the legacy coroutine marker",
        fix(checker, func),
        "Call `inspect.iscoroutinefunction()`",
    );
}

// The legacy marker of `asyncio.coroutine` counts for `asyncio` only, so the rewrite can change the result.
fn fix(checker: &Checker, func: &Expr) -> Option<Fix> {
    let import = ensure_import(checker, "inspect", None, func.range())?;
    let replacement = Edit::range_replacement("inspect.iscoroutinefunction".to_owned(), func.range());
    Some(with_import(replacement, import, Applicability::Unsafe))
}
