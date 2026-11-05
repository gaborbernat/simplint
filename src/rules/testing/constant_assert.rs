use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::{Truthiness, contains_effect};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{delete_statement, is_alone_on_lines};
use crate::syntax::for_each_body;

pub fn constant_assert(checker: &Checker) {
    let is_builtin = |name: &str| !checker.semantic().is_bound(name);
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Assert(assert) = stmt
                && matches!(Truthiness::from_expr(&assert.test, is_builtin), Truthiness::True | Truthiness::Truthy)
            {
                report_constant_assert(checker, stmt, assert, statements);
            }
        }
    });
}

fn report_constant_assert(checker: &Checker, stmt: &Stmt, assert: &ast::StmtAssert, statements: &[Stmt]) {
    let ast::StmtAssert { test, msg, .. } = assert;
    let fix = match (test.as_ref(), msg) {
        // `assert (check, "message")` meant `assert check, "message"`; deleting it would drop the check.
        // A starred element has no meaning outside the tuple, so there is no single-argument form to restore.
        (Expr::Tuple(tuple @ ast::ExprTuple { elts, .. }), None)
            if elts.len() == 2 && !elts.iter().any(Expr::is_starred_expr) =>
        {
            // Out of the tuple's brackets, a walrus or a multi-line element needs parentheses of its own.
            let [check, message] = [&elts[0], &elts[1]].map(|elt| {
                let text = checker.parenthesized_text(elt, tuple.into());
                if (text.contains('\n') || matches!(elt, Expr::Named(_))) && text == checker.text(elt.range()) {
                    format!("({text})")
                } else {
                    text.to_owned()
                }
            });
            let replacement = format!("assert {check}, {message}");
            Some(Fix::unsafe_edit(Edit::range_replacement(replacement, stmt.range())))
        }
        (Expr::Tuple(_), _) => None,
        // A call inside the test, as in `assert [compute()]`, runs even though the assertion cannot fail.
        _ => (is_alone_on_lines(checker.source, stmt.range())
            && !contains_effect(test, |name| !checker.semantic().is_bound(name)))
        .then(|| Fix::safe_edit(delete_statement(checker.source, stmt, statements))),
    };
    let title = if test.is_tuple_expr() { "Split the tuple into a test and a message" } else { "Remove the assertion" };
    checker.report_fix(
        Rule::ConstantAssert,
        stmt.range(),
        "Assertion on a constant always passes; assert on the result under test",
        fix,
        title,
    );
}
