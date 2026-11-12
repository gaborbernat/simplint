use ruff_python_ast::comparable::{ComparableExpr, ComparablePattern, ComparableStmt};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

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
    // `read() or read()` calls twice and can differ; a walrus rebinds what a later operand reads.
    if values.iter().any(|value| any_over_expr(value, &has_effect)) {
        return;
    }
    let mut seen: Vec<ComparableExpr> = Vec::new();
    for value in values {
        let comparable = ComparableExpr::from(value);
        if seen.contains(&comparable) {
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
        } else {
            seen.push(comparable);
        }
    }
}

// A test the chain already tried is false whenever its branch is reached, so the branch never runs. Ruff's SIM114
// reports a branch whose body repeats the one right before it, and offers to merge the two, so that pair is left to it.
fn repeated_tests(walker: &Walker, if_stmt: &ast::StmtIf) {
    let mut branches = vec![(&*if_stmt.test, if_stmt.body.as_slice())];
    branches.extend(
        if_stmt.elif_else_clauses.iter().filter_map(|clause| clause.test.as_ref().map(|test| (test, clause.body.as_slice()))),
    );
    if branches.iter().any(|(test, _)| any_over_expr(test, &has_effect)) {
        return;
    }
    for (index, (test, body)) in branches.iter().enumerate().skip(1) {
        let comparable = ComparableExpr::from(*test);
        let Some(earlier) = branches[..index].iter().position(|(seen, _)| ComparableExpr::from(*seen) == comparable)
        else {
            continue;
        };
        if earlier + 1 == index && same_body(branches[earlier].1, body) {
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

// A case matches whenever an earlier unguarded case with the same pattern would, so it never runs. An earlier case with
// the same guard hides it too, unless the guard calls something that could answer differently the second time.
fn repeated_patterns(walker: &Walker, match_stmt: &ast::StmtMatch) {
    for (index, case) in match_stmt.cases.iter().enumerate() {
        let pattern = ComparablePattern::from(&case.pattern);
        let hidden = match_stmt.cases[..index].iter().any(|earlier| {
            ComparablePattern::from(&earlier.pattern) == pattern
                && match (&earlier.guard, &case.guard) {
                    (None, _) => true,
                    (Some(first), Some(second)) => {
                        ComparableExpr::from(first.as_ref()) == ComparableExpr::from(second.as_ref())
                            && !any_over_expr(first, &has_effect)
                    }
                    (Some(_), None) => false,
                }
        });
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
