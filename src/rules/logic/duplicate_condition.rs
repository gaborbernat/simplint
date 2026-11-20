use ruff_python_ast::comparable::{ComparableExpr, ComparablePattern, ComparableStmt};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::If(if_stmt) => repeated_tests(walker, if_stmt),
        Stmt::Match(match_stmt) => repeated_patterns(walker, match_stmt),
        _ => {}
    }
}

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::BoolOp(ast::ExprBoolOp { op, values, .. }) = expr else {
        return;
    };
    // Repeated calls can return different values, and an assignment expression can change a later operand.
    if values.iter().any(|value| any_over_expr(value, &has_effect)) {
        return;
    }
    let mut seen: FxHashSet<ComparableExpr> = FxHashSet::default();
    for value in values {
        if !seen.insert(ComparableExpr::from(value)) {
            walker.checker.report(
                Rule::DuplicateCondition,
                value.range(),
                format!(
                    "`{}` repeats an earlier operand of this `{}`, so it never changes the result",
                    walker.checker.text(value),
                    match op {
                        ast::BoolOp::And => "and",
                        ast::BoolOp::Or => "or",
                    }
                ),
            );
        }
    }
}

// Ruff's SIM114 can merge adjacent branches with identical bodies.
fn repeated_tests(walker: &Walker, if_stmt: &ast::StmtIf) {
    let mut branches = vec![(&*if_stmt.test, if_stmt.body.as_slice())];
    branches.extend(
        if_stmt
            .elif_else_clauses
            .iter()
            .filter_map(|clause| clause.test.as_ref().map(|test| (test, clause.body.as_slice()))),
    );
    if branches.iter().any(|(test, _)| any_over_expr(test, &has_effect)) {
        return;
    }
    let mut first: FxHashMap<ComparableExpr, usize> = FxHashMap::default();
    for (index, (test, body)) in branches.iter().enumerate() {
        let earlier = *first.entry(ComparableExpr::from(*test)).or_insert(index);
        if earlier == index || earlier + 1 == index && same_body(branches[earlier].1, body) {
            continue;
        }
        walker.checker.report(
            Rule::DuplicateCondition,
            test.range(),
            format!(
                "`{}` repeats an earlier test of this `if` chain, so this branch never runs",
                walker.checker.text(*test)
            ),
        );
    }
}

fn same_body(first: &[Stmt], second: &[Stmt]) -> bool {
    first.len() == second.len()
        && first.iter().zip(second).all(|(left, right)| ComparableStmt::from(left) == ComparableStmt::from(right))
}

// A guarded case leaves its pattern available to a later case when the guard fails.
fn repeated_patterns(walker: &Walker, match_stmt: &ast::StmtMatch) {
    let mut earlier_cases: FxHashMap<ComparablePattern, Vec<&ast::MatchCase>> = FxHashMap::default();
    for case in &match_stmt.cases {
        let earlier = earlier_cases.entry(ComparablePattern::from(&case.pattern)).or_default();
        let hidden = earlier.iter().any(|earlier| match (&earlier.guard, &case.guard) {
            (None, _) => true,
            (Some(first), Some(second)) => {
                ComparableExpr::from(first.as_ref()) == ComparableExpr::from(second.as_ref())
                    && !any_over_expr(first, &has_effect)
            }
            (Some(_), None) => false,
        });
        earlier.push(case);
        if hidden {
            walker.checker.report(
                Rule::DuplicateCondition,
                case.pattern.range(),
                format!(
                    "`case {}` repeats an earlier case, so this case never runs",
                    walker.checker.text(&case.pattern)
                ),
            );
        }
    }
}

fn has_effect(expr: &Expr) -> bool {
    matches!(expr, Expr::Call(_) | Expr::Await(_) | Expr::Named(_) | Expr::Yield(_) | Expr::YieldFrom(_))
}
