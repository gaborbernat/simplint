use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::callee::Callee;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
        return;
    };
    let Expr::Call(call) = value.as_ref() else {
        return;
    };
    let Some(callee) = Callee::of(walker, &call.func) else {
        return;
    };
    if !callee.makes_coroutine() || callee.is_decorated() {
        return;
    }
    let name = callee.name();
    let in_coroutine = in_coroutine(walker);
    let (advice, fix) = if in_coroutine {
        ("`await` it", Some(Fix::unsafe_edit(Edit::insertion("await ".to_owned(), call.start()))))
    } else {
        ("`await` it from a coroutine, or run it with `asyncio.run`", None)
    };
    walker.checker.report_fix(
        Rule::UnawaitedCoroutine,
        call.range(),
        format!("`{name}()` makes a coroutine that nothing awaits, so its body never runs; {advice}"),
        fix,
        format!("Await `{name}()`"),
    );
}

// Only the body of an `async def` itself can await: neither a lambda nor a class body inside one can.
fn in_coroutine(walker: &Walker) -> bool {
    matches!(walker.scopes().last(), Some(Scope::Function(function)) if function.is_async)
}
