use ruff_python_ast::{self as ast, CmpOp, Expr, Operator};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Compare(compare) = expr else {
        return;
    };
    if compare.ops.len() < 2 {
        return;
    }
    let checker = walker.checker;
    // A bitwise operator binds tighter than a comparison, so `a > 1 & b < 2` is `a > (1 & b) < 2`, not the
    // `(a > 1) & (b < 2)` a mask written without parentheses means.
    let middle = &compare.operands[1..compare.operands.len() - 1];
    if let Some(operand) = middle.iter().find(|operand| is_bare_bitwise(walker, compare, operand)) {
        checker.report(
            Rule::AmbiguousOperatorChain,
            expr.range(),
            format!(
                "`{}` binds tighter than the comparisons around it, so this compares against it as one value",
                checker.text(operand)
            ),
        );
        return;
    }
    if is_plain_chain(&compare.ops) {
        return;
    }
    let operators = compare.ops.iter().map(CmpOp::as_str).collect::<Vec<_>>().join("`, `");
    checker.report(
        Rule::AmbiguousOperatorChain,
        expr.range(),
        format!("This chain mixes `{operators}`, and Python joins each pair with `and`, which reads as one comparison"),
    );
}

fn is_bare_bitwise(walker: &Walker, compare: &ast::ExprCompare, operand: &Expr) -> bool {
    matches!(
        operand,
        Expr::BinOp(ast::ExprBinOp { op: Operator::BitAnd | Operator::BitOr | Operator::BitXor, .. })
    ) && walker.checker.parenthesized_range(operand, compare.into()) == operand.range()
}

// `a < b <= c`, `a == b == c`, `a == b != c` and `a is b is None` read as written. A chain that turns around, such as
// `a < b > c`, that holds `in`, or that mixes kinds, such as `0 < x is None`, does not, and neither does `a != b != c`,
// which does not compare `a` with `c`.
fn is_plain_chain(ops: &[CmpOp]) -> bool {
    let all = |allowed: &[CmpOp]| ops.iter().all(|op| allowed.contains(op));
    all(&[CmpOp::Lt, CmpOp::LtE])
        || all(&[CmpOp::Gt, CmpOp::GtE])
        || all(&[CmpOp::Is])
        || (all(&[CmpOp::Eq, CmpOp::NotEq]) && ops.iter().filter(|op| **op == CmpOp::NotEq).count() <= 1)
}
