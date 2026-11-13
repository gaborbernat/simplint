use ruff_python_ast::{self as ast, Expr, Operator};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Some(call) = expr.as_call_expr() else {
        return;
    };
    let semantic = walker.checker.semantic();
    if semantic.is_builtin(&call.func, "vars")
        && semantic.references_to("vars").iter().any(|reference| reference.range == call.func.range())
        && !builds_mapping(walker, expr)
        && !call.arguments.args.first().is_some_and(|argument| semantic.is_class_object(argument))
    {
        walker.checker.report(
            Rule::VarsCall,
            expr.range(),
            "`vars()` erases types; read attributes from a typed object",
        );
    }
}

// The attributes of an open-ended object have no key type to name, so a mapping whose keys come from its `vars()` is
// left alone; a `vars()` that is only a value in a mapping does not supply keys and stays reported.
fn builds_mapping(walker: &Walker, expr: &Expr) -> bool {
    let Some(parent) = walker.expression_parent() else {
        return false;
    };
    match parent {
        Expr::Call(call) => {
            walker.checker.semantic().is_builtin(&call.func, "dict")
                && call.arguments.args.first().is_some_and(|arg| arg.range() == expr.range())
        }
        Expr::Dict(ast::ExprDict { items, .. }) => {
            items.iter().any(|item| item.key.is_none() && item.value.range() == expr.range())
        }
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::BitOr, right, .. }) => {
            left.range() == expr.range() || right.range() == expr.range()
        }
        // `vars(obj).items()` is the iterable of a dict comprehension, which sits two expressions above the call.
        Expr::Attribute(ast::ExprAttribute { attr, .. }) if attr.as_str() == "items" => {
            let [.., Expr::DictComp(comprehension), Expr::Call(items), _, _] = walker.expressions() else {
                return false;
            };
            items.func.range() == parent.range()
                && comprehension.generators.iter().any(|generator| generator.iter.range() == items.range())
        }
        _ => false,
    }
}
