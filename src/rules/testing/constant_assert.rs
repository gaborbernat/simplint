use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::{Truthiness, contains_effect};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{delete_statement, for_each_body, is_alone_on_lines};

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
        (Expr::Tuple(tuple @ ast::ExprTuple { elts, .. }), None) if elts.len() == 2 => {
            // Out of the tuple's brackets, an element written over several lines needs parentheses of its own.
            let [check, message] = [&elts[0], &elts[1]].map(|elt| {
                let text = checker.parenthesized_text(elt, tuple.into());
                if text.contains('\n') && text == checker.text(elt.range()) {
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
