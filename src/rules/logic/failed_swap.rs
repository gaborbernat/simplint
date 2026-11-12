use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_body};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn failed_swap(checker: &Checker) {
    let mut visitor = Visitor { checker };
    visitor.visit_body(checker.body());
}

struct Visitor<'c, 'a> {
    checker: &'c Checker<'a>,
}

impl<'a> StatementVisitor<'a> for Visitor<'_, 'a> {
    fn visit_body(&mut self, body: &'a [Stmt]) {
        for pair in body.windows(2) {
            self.swap(&pair[0], &pair[1]);
        }
        for stmt in body {
            if let Stmt::AugAssign(augmented) = stmt {
                self.repeated_target(augmented);
            }
        }
        walk_body(self, body);
    }
}

impl Visitor<'_, '_> {
    // After `a = b`, `b = a` stores `b` back into itself, so both names hold the old `b`.
    fn swap(&self, first: &Stmt, second: &Stmt) {
        let (Some((first_target, first_value)), Some((second_target, second_value))) = (plain(first), plain(second))
        else {
            return;
        };
        let same = |left: &Expr, right: &Expr| ComparableExpr::from(left) == ComparableExpr::from(right);
        if !same(first_target, second_value) || !same(first_value, second_target) || same(first_target, first_value) {
            return;
        }
        let checker = self.checker;
        let range = first.range().cover(second.range());
        let fix = (!checker.has_comment_in(range)).then(|| {
            Fix::unsafe_edit(Edit::range_replacement(
                format!(
                    "{first}, {second} = {second}, {first}",
                    first = checker.text(first_target),
                    second = checker.text(first_value)
                ),
                range,
            ))
        });
        checker.report_fix(
            Rule::FailedSwap,
            range,
            format!(
                "`{}` is assigned back from the value it just received, so both hold the old `{}`",
                checker.text(second_target),
                checker.text(first_value)
            ),
            fix,
            "Swap both in one tuple assignment",
        );
    }

    // `x += x + y` adds `x` twice, where `x += y` was meant.
    fn repeated_target(&self, augmented: &ast::StmtAugAssign) {
        let op = augmented.op;
        // `x + y + z` parses as `(x + y) + z`, so the target sits at the bottom of the left operands.
        let mut leftmost = augmented.value.as_ref();
        let mut nested = false;
        while let Expr::BinOp(ast::ExprBinOp { left, op: inner, .. }) = leftmost
            && *inner == op
        {
            leftmost = left;
            nested = true;
        }
        if !nested || !is_reference(&augmented.target) {
            return;
        }
        if ComparableExpr::from(leftmost) != ComparableExpr::from(augmented.target.as_ref()) {
            return;
        }
        let checker = self.checker;
        checker.report(
            Rule::FailedSwap,
            augmented.range(),
            format!(
                "`{target} {op}= {target} {op} ...` applies `{target}` twice; `{target} {op}= ...` applies it once",
                target = checker.text(augmented.target.as_ref()),
                op = op.as_str()
            ),
        );
    }
}

// A single target and a value that are both plain names or attributes, which reading has no effect on.
fn plain(stmt: &Stmt) -> Option<(&Expr, &Expr)> {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return None;
    };
    let [target] = targets.as_slice() else {
        return None;
    };
    (is_reference(target) && is_reference(value)).then_some((target, value.as_ref()))
}

fn is_reference(expr: &Expr) -> bool {
    match expr {
        Expr::Name(_) => true,
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_reference(value),
        _ => false,
    }
}
