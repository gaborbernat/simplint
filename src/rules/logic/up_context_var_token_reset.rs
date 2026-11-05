use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::scoped_block::{finally_call, identifier_uses, scope_of, sole_argument, wrap_edits};
use crate::syntax::{for_each_body, is_name};

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
    let [Expr::Name(token)] = assign.targets.as_slice() else {
        return;
    };
    let Expr::Call(set) = assign.value.as_ref() else {
        return;
    };
    let Some(set_attribute) = set.func.as_attribute_expr() else {
        return;
    };
    if set_attribute.attr.as_str() != "set" || sole_argument(set).is_none() {
        return;
    }
    let Some(reset) = finally_call(try_stmt) else {
        return;
    };
    let Some(reset_attribute) = reset.func.as_attribute_expr() else {
        return;
    };
    let same_variable = checker.text(reset_attribute.value.as_ref()) == checker.text(set_attribute.value.as_ref());
    if reset_attribute.attr.as_str() != "reset"
        || !same_variable
        || !sole_argument(reset).is_some_and(|argument| is_name(argument, token.id.as_str()))
    {
        return;
    }
    // The assignment and the `reset` argument are the token's only spellings; any other use reads it after the block.
    let uses = identifier_uses(checker, scope_of(checker, assign.range()), token.id.as_str());
    if uses != 2 {
        return;
    }
    let header = format!("with {}:\n", checker.text(set));
    let fix = wrap_edits(checker, assign.range(), try_stmt, &header).map(|[head, tail]| Fix::safe_edits(head, [tail]));
    checker.report_fix(
        Rule::ContextVarTokenReset,
        assign.range(),
        format!("`{}` resets the variable in a `finally`; a `with` block does that on exit", token.id),
        fix,
        "Enter the `set` call as a `with` block",
    );
}
