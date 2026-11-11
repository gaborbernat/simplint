use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::checker::Checker;
use crate::semantic::Semantic;
use crate::syntax::single_assignment;

pub(crate) mod callable_ellipsis;
pub(crate) mod imprecise_annotation;
mod missing_final;
pub(crate) mod vars_call;

pub(crate) use missing_final::missing_final;

pub(crate) fn is_type_parameter_constructor(semantic: &Semantic, func: &Expr) -> bool {
    ["TypeVar", "TypeVarTuple", "ParamSpec"].iter().any(|member| semantic.match_typing_expr(func, member))
}

pub(crate) fn type_parameter_assignment<'a>(checker: &Checker, stmt: &'a Stmt) -> Option<&'a ast::ExprName> {
    let (target, _, value) = single_assignment(stmt)?;
    let call = value.as_call_expr()?;
    if !is_type_parameter_constructor(checker.semantic(), &call.func) {
        return None;
    }
    let name = call
        .arguments
        .args
        .first()
        .or_else(|| call.arguments.find_keyword("name").map(|keyword| &keyword.value))?
        .as_string_literal_expr()?
        .value
        .to_str();
    (target.id.as_str() == name).then_some(target)
}
