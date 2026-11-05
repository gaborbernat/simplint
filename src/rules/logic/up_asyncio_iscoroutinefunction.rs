use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::ensure_import;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return;
    };
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
    let replacement = Edit::range_replacement("inspect.iscoroutinefunction".to_owned(), func.range());
    if has_plain_inspect_import(checker.body()) {
        return Some(Fix::unsafe_edit(replacement));
    }
    let (_, edit) = ensure_import(checker, "inspect", None)?;
    Some(Fix::unsafe_edits(replacement, edit).isolate(IsolationLevel::Group(1)))
}

fn has_plain_inspect_import(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| {
        matches!(stmt, Stmt::Import(import) if import
            .names
            .iter()
            .any(|alias| alias.name.as_str() == "inspect" && alias.asname.is_none()))
    })
}
