use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Some(name) = expr.as_call_expr().and_then(|call| call.func.as_name_expr()) else {
        return;
    };
    let semantic = walker.checker.semantic();
    if name.id.as_str() == "vars"
        && !semantic.is_bound("vars")
        && semantic.references_to("vars").iter().any(|reference| reference.range == name.range)
    {
        walker.checker.report(
            Rule::VarsCall,
            expr.range(),
            "`vars()` erases types; read attributes from a typed object",
        );
    }
}
