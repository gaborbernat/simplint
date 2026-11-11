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
        || !in_coroutine(walker, expr)
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

// The innermost scope whose body holds the call decides. A class body runs as part of the function it sits in, and a
// header (decorators, defaults, annotations) runs where its `def` statement runs, so those are skipped. Lazy annotations
// are never evaluated there, so a call in one is left alone.
fn in_coroutine(walker: &Walker, expr: &Expr) -> bool {
    let lazy = walker.checker.semantic().annotations_are_lazy();
    walker
        .scopes()
        .iter()
        .rev()
        .find_map(|scope| match scope {
            Scope::Function(function) if expr.start() >= function.body[0].start() => Some(function.is_async),
            Scope::Function(function) if lazy && in_annotation(function, expr) => Some(false),
            Scope::Lambda(lambda) if expr.start() >= lambda.body.start() => Some(false),
            Scope::Function(_) | Scope::Lambda(_) | Scope::Class(_) => None,
        })
        .unwrap_or(false)
}

fn in_annotation(function: &ast::StmtFunctionDef, expr: &Expr) -> bool {
    function
        .parameters
        .iter()
        .filter_map(ast::AnyParameterRef::annotation)
        .chain(function.returns.as_deref())
        .any(|annotation| annotation.range().contains_range(expr.range()))
}
