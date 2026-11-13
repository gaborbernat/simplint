use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, BoolOp, Expr, Stmt, UnaryOp};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::callee::Callee;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::If(ast::StmtIf { test, elif_else_clauses, .. }) => {
            check(walker, test);
            for clause in elif_else_clauses {
                if let Some(test) = &clause.test {
                    check(walker, test);
                }
            }
        }
        Stmt::While(ast::StmtWhile { test, .. }) | Stmt::Assert(ast::StmtAssert { test, .. }) => check(walker, test),
        _ => {}
    }
}

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    if let Expr::If(ast::ExprIf { test, .. }) = expr {
        check(walker, test);
    }
}

// A function object is always true, so a test of one, alone, negated or as an operand of `and` or `or`, never
// depends on what the function would return.
fn check<'a>(walker: &Walker<'_, 'a>, test: &'a Expr) {
    match test {
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => check(walker, operand),
        Expr::BoolOp(ast::ExprBoolOp { op: BoolOp::And | BoolOp::Or, values, .. }) => {
            for value in values {
                check(walker, value);
            }
        }
        Expr::Name(_) | Expr::Attribute(_) => report(walker, test),
        _ => {}
    }
}

fn report<'a>(walker: &Walker<'_, 'a>, test: &'a Expr) {
    let Some(callee) = Callee::of(walker, test) else {
        return;
    };
    // A decorator such as `property` turns the attribute into the value it computes.
    if callee.is_decorated() {
        return;
    }
    let name = callee.name();
    walker.checker.report_fix(
        Rule::FunctionAsCondition,
        test.range(),
        format!("`{name}` is a function, which is always true; call it to test what it returns"),
        takes_no_arguments(&callee).then(|| Fix::unsafe_edit(Edit::insertion("()".to_owned(), test.end()))),
        format!("Call `{name}()`"),
    );
}

fn takes_no_arguments(callee: &Callee) -> bool {
    let Some(function) = callee.function else {
        return false;
    };
    let parameters = &function.parameters;
    parameters
        .posonlyargs
        .iter()
        .chain(&parameters.args)
        .skip(usize::from(callee.bound))
        .chain(&parameters.kwonlyargs)
        .all(|parameter| parameter.default.is_some())
}
