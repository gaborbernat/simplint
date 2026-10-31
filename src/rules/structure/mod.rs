mod definition_before_use;
mod forwarding_function;
mod single_user_constant;
mod walrus_candidate;

pub(crate) use definition_before_use::definition_before_use;
pub(crate) use forwarding_function::forwarding_function;
pub(crate) use single_user_constant::single_user_constant;
pub(crate) use walrus_candidate::walrus_candidate;

use ruff_python_ast::{Expr, OperatorPrecedence};

fn needs_parentheses(value: &Expr) -> bool {
    OperatorPrecedence::from_expr(value) < OperatorPrecedence::CallAttribute
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
}
