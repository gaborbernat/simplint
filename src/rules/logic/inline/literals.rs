use ruff_python_ast::{self as ast, Expr, UnaryOp};

pub(in crate::rules::logic) fn is_constant(expr: &Expr) -> bool {
    match expr {
        Expr::NumberLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        // `-` and `+` take a number, so the operand must be one.
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => is_constant(operand),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::UAdd | UnaryOp::USub, operand, .. }) => {
            operand.is_number_literal_expr()
        }
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().all(is_constant)
        }
        // A set element or dict key is hashed when the display is built, so a list or dict there is not constant.
        Expr::Set(ast::ExprSet { elts, .. }) => elts.iter().all(is_hashable),
        Expr::Dict(ast::ExprDict { items, .. }) => {
            items.iter().all(|item| item.key.as_ref().is_some_and(is_hashable) && is_constant(&item.value))
        }
        _ => false,
    }
}

pub(in crate::rules::logic) fn is_hashable(expr: &Expr) -> bool {
    match expr {
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().all(is_hashable),
        Expr::List(_) | Expr::Set(_) | Expr::Dict(_) => false,
        _ => is_constant(expr),
    }
}
