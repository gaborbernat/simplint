use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::{self as ast, CmpOp, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::needs_parentheses;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Compare(compare) = expr else {
        return;
    };
    let Some((left, &op, right)) = compare.as_single() else {
        return;
    };
    let (CmpOp::Eq | CmpOp::NotEq) = op else {
        return;
    };
    let (value, empty) = match (empty_literal(left), empty_literal(right)) {
        (None, Some(empty)) => (left, empty),
        (Some(empty), None) => (right, empty),
        _ => return,
    };
    // An `assert` that a result equals `[]` also checks its type, which a truth test would not; so does a comparison
    // next to one that tells the value from `None` or another literal.
    if matches!(walker.statement(), Some(Stmt::Assert(_))) || tells_types_apart(walker, expr, value) {
        return;
    }
    let checker = walker.checker;
    let text = checker.parenthesized_text(value, compare.into());
    let fix = is_condition(walker, expr).then(|| {
        let replacement = match op {
            CmpOp::Eq if needs_parentheses(value) && !text.starts_with('(') => format!("not ({text})"),
            CmpOp::Eq => format!("not {text}"),
            _ => text.to_owned(),
        };
        Fix::unsafe_edit(Edit::range_replacement(replacement, expr.range()))
    });
    let truth = if op == CmpOp::Eq { "falsy" } else { "truthy" };
    checker.report_fix(
        Rule::EmptyCollectionComparison,
        expr.range(),
        format!("Comparing with an empty `{empty}` builds a new object; test whether the value is {truth} instead"),
        fix,
        "Test the value's truth",
    );
}

fn empty_literal(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::List(ast::ExprList { elts, .. }) if elts.is_empty() => Some("[]"),
        Expr::Tuple(ast::ExprTuple { elts, .. }) if elts.is_empty() => Some("()"),
        Expr::Dict(ast::ExprDict { items, .. }) if items.is_empty() => Some("{}"),
        _ => None,
    }
}

// The test of an `if`, `elif`, `while` or conditional expression, where only the truth of the comparison counts.
fn is_condition(walker: &Walker, expr: &Expr) -> bool {
    let is_expr = |candidate: &Expr| std::ptr::eq(candidate, expr);
    if let Some(Expr::If(ternary)) = walker.expression_parent() {
        return is_expr(&ternary.test);
    }
    if walker.expression_parent().is_some() {
        return false;
    }
    match walker.statement() {
        Some(Stmt::If(ast::StmtIf { test, elif_else_clauses, .. })) => {
            is_expr(test) || elif_else_clauses.iter().any(|clause| clause.test.as_ref().is_some_and(is_expr))
        }
        Some(Stmt::While(ast::StmtWhile { test, .. })) => is_expr(test),
        _ => false,
    }
}

// A `bool` operation around the comparison that also compares the value with `None` or another literal, as
// `value == [] or value is None` does to tell falsy values apart.
fn tells_types_apart(walker: &Walker, expr: &Expr, value: &Expr) -> bool {
    let value = ComparableExpr::from(value);
    walker.expressions().iter().filter_map(|outer| outer.as_bool_op_expr()).any(|bool_op| {
        bool_op.values.iter().filter(|operand| !std::ptr::eq(*operand, expr)).any(|operand| {
            operand.as_compare_expr().and_then(ast::ExprCompare::as_single).is_some_and(|(left, _, right)| {
                (ComparableExpr::from(left) == value && is_literal(right))
                    || (ComparableExpr::from(right) == value && is_literal(left))
            })
        })
    })
}

fn is_literal(expr: &Expr) -> bool {
    expr.is_literal_expr() || empty_literal(expr).is_some()
}
