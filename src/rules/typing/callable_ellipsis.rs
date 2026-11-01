use ruff_python_ast::{self as ast, Expr};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Expr::Subscript(ast::ExprSubscript { value, slice, range, .. }) = expr else {
        return;
    };
    let Expr::Tuple(ast::ExprTuple { elts, .. }) = slice.as_ref() else {
        return;
    };
    let semantic = walker.checker.semantic();
    if (semantic.match_typing_expr(value, "Callable")
        || semantic.resolves_to(value, &["collections", "abc", "Callable"]))
        && elts.first().is_some_and(Expr::is_ellipsis_literal_expr)
    {
        walker.checker.report(
            Rule::CallableEllipsis,
            *range,
            "`Callable[..., T]` hides the signature; describe it with a `Protocol`",
        );
    }
}
