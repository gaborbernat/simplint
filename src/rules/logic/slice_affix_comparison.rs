use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, CmpOp, Expr, Number, UnaryOp};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::kinds::Kinds;
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
    let (subscript, literal) = match (left, right) {
        (Expr::Subscript(subscript), literal) | (literal, Expr::Subscript(subscript)) => (subscript, literal),
        _ => return,
    };
    let Some(length) = literal_length(literal) else {
        return;
    };
    let Some(method) = affix(&subscript.slice, length) else {
        return;
    };
    let value = subscript.value.as_ref();
    // Any class may answer `==` with a `str`, as pyasn1's character types do, without having `startswith`.
    let text = Kinds::new(walker, expr.range()).is_text(value);
    let checker = walker.checker;
    let receiver = if needs_parentheses(value) {
        format!("({})", checker.text(value))
    } else {
        checker.text(value).to_owned()
    };
    let call = format!("{receiver}.{method}({})", checker.text(literal));
    let replacement = if op == CmpOp::Eq { call } else { format!("not {call}") };
    checker.report_fix(
        Rule::SliceAffixComparison,
        expr.range(),
        format!("Comparing a slice with a literal of its length is `{method}`"),
        text.then(|| Fix::unsafe_edit(Edit::range_replacement(replacement, expr.range()))),
        format!("Use `{method}`"),
    );
}

// The length of a string or bytes literal: characters for a string, bytes for bytes.
fn literal_length(expr: &Expr) -> Option<u64> {
    let length = match expr {
        Expr::StringLiteral(string) => string.value.to_str().chars().count(),
        Expr::BytesLiteral(bytes) => bytes.value.bytes().count(),
        _ => return None,
    };
    u64::try_from(length).ok().filter(|length| *length > 0)
}

// `[:n]` is the prefix of `n` items and `[-n:]` the suffix of `n` items.
fn affix(slice: &Expr, length: u64) -> Option<&'static str> {
    let Expr::Slice(ast::ExprSlice { lower, upper, step: None, .. }) = slice else {
        return None;
    };
    match (lower.as_deref(), upper.as_deref()) {
        (None, Some(upper)) if int(upper) == Some(length) => Some("startswith"),
        (Some(Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub, operand, .. })), None)
            if int(operand) == Some(length) =>
        {
            Some("endswith")
        }
        _ => None,
    }
}

fn int(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: Number::Int(int), .. }) => int.as_u64(),
        _ => None,
    }
}
