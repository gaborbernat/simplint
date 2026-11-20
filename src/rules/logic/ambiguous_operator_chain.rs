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
    // Parentheses around the bitwise operand make the intended comparison explicit.
    if let Some(operand) =
        compare.operands[1..compare.operands.len() - 1].iter().find(|operand| is_bare_bitwise(walker, compare, operand))
    {
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
    matches!(operand, Expr::BinOp(ast::ExprBinOp { op: Operator::BitAnd | Operator::BitOr | Operator::BitXor, .. }))
        && walker.checker.parenthesized_range(operand, compare.into()) == operand.range()
}

// A single inequality can mark the endpoint of an equality chain. Two inequalities never compare the outer values.
fn is_plain_chain(ops: &[CmpOp]) -> bool {
    let all = |allowed: &[CmpOp]| ops.iter().all(|op| allowed.contains(op));
    all(&[CmpOp::Lt, CmpOp::LtE])
        || all(&[CmpOp::Gt, CmpOp::GtE])
        || all(&[CmpOp::Is])
        || (all(&[CmpOp::Eq, CmpOp::NotEq]) && ops.iter().filter(|op| **op == CmpOp::NotEq).count() <= 1)
}
