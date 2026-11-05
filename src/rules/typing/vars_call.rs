use ruff_python_ast::Expr;
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
    {
        walker.checker.report(
            Rule::VarsCall,
            expr.range(),
            "`vars()` erases types; read attributes from a typed object",
        );
    }
}
