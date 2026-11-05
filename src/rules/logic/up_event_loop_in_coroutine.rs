use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return;
    };
    if !arguments.is_empty()
        || !checker.semantic().resolves_to(func, &["asyncio", "get_event_loop"])
        || !in_coroutine(walker)
    {
        return;
    }
    let fix = func.as_attribute_expr().map(|attribute| {
        Fix::safe_edit(Edit::range_replacement("get_running_loop".to_owned(), attribute.attr.range()))
    });
    checker.report_fix(
        Rule::EventLoopInCoroutine,
        expr.range(),
        "`asyncio.get_event_loop()` in a coroutine; a loop is always running there",
        fix,
        "Call `asyncio.get_running_loop()`",
    );
}

// The innermost function decides. A class body runs as part of the function it sits in, so it is skipped.
fn in_coroutine(walker: &Walker) -> bool {
    walker
        .scopes()
        .iter()
        .rev()
        .find_map(|scope| match scope {
            Scope::Function(function) => Some(function.is_async),
            Scope::Lambda(_) => Some(false),
            Scope::Class(_) => None,
        })
        .unwrap_or(false)
}
