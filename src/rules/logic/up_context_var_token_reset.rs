use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::scoped_block::{finally_call, is_exported_module_name, wrap_edits};
use crate::rules::logic::sole_name_target;
use crate::syntax::{for_each_body, is_name, sole_argument};

// Since Python 3.14 the token `ContextVar.set` returns is a context manager that resets the variable on exit.
pub fn up_context_var_token_reset(checker: &Checker) {
    for_each_body(checker.body(), &mut |body, _| {
        for pair in body.windows(2) {
            if let [Stmt::Assign(assign), Stmt::Try(try_stmt)] = pair {
                report_reset(checker, assign, try_stmt);
            }
        }
    });
}

fn report_reset(checker: &Checker, assign: &ast::StmtAssign, try_stmt: &ast::StmtTry) {
    let Some(token) = sole_name_target(assign) else {
        return;
    };
    let Expr::Call(set) = assign.value.as_ref() else {
        return;
    };
    let Some(set_attribute) = set.func.as_attribute_expr() else {
        return;
    };
    if set_attribute.attr.as_str() != "set"
        || sole_argument(set).is_none()
        || !is_context_var(checker, &set_attribute.value)
    {
        return;
    }
    let Some(reset) = finally_call(try_stmt) else {
        return;
    };
    let Some(reset_attribute) = reset.func.as_attribute_expr() else {
        return;
    };
    if reset_attribute.attr.as_str() != "reset"
        || checker.text(reset_attribute.value.as_ref()) != checker.text(set_attribute.value.as_ref())
        || !sole_argument(reset).is_some_and(|argument| is_name(argument, token.id.as_str()))
    {
        return;
    }
    // The assignment and the `reset` argument are the token's only spellings; any other use reads it after the block.
    if checker.identifier_count_in(checker.enclosing_scope(assign.range()), token.id.as_str()) != 2 {
        return;
    }
    let fix = wrap_edits(checker, assign.range(), try_stmt, &format!("with {}:", checker.text(set)))
        .filter(|_| !is_exported_module_name(checker, token))
        .map(|[head, tail]| Fix::safe_edits(head, [tail]));
    checker.report_fix(
        Rule::ContextVarTokenReset,
        assign.range(),
        format!("`{}` resets the variable in a `finally`; a `with` block does that on exit", token.id),
        fix,
        "Enter the `set` call as a `with` block",
    );
}

// Whether every binding of the receiver gives it a `ContextVar`: an assignment of a `contextvars.ContextVar(...)`
// call, or a parameter annotated as one. Any other object's `set` returns something that need not enter as a context
// manager.
fn is_context_var(checker: &Checker, receiver: &Expr) -> bool {
    let Expr::Name(name) = receiver else {
        return false;
    };
    let semantic = checker.semantic();
    let names_context_var = |expr: &Expr| {
        let class = match expr {
            Expr::Subscript(ast::ExprSubscript { value, .. }) => value.as_ref(),
            _ => expr,
        };
        semantic.resolves_to(class, &["contextvars", "ContextVar"])
    };
    let bindings = semantic.resolved_bindings(name);
    !bindings.is_empty()
        && bindings.iter().all(|binding| match semantic.binding_values(binding) {
            Some(values) => {
                values.iter().all(|value| value.as_call_expr().is_some_and(|call| names_context_var(&call.func)))
            }
            None => semantic.parameter_annotation(binding).is_some_and(names_context_var),
        })
}
