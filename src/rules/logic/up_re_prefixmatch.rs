use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, CmpOp, Expr};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::anchored_regex_match::anchored_call;
use crate::rules::logic::modernize::{module_attribute_read, rename_attribute};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // `re.prefixmatch` is the same function as `re.match` from Python 3.15, so the rename keeps the behavior.
    let Some(attribute) = module_attribute_read(checker, expr, &["re", "match"]) else {
        return;
    };
    if left_to_ruff(walker, expr) || left_to_anchored_regex_match(walker, expr) {
        return;
    }
    checker.report_fix(
        Rule::RePrefixMatch,
        expr.range(),
        "`re.match` is soft deprecated from Python 3.15; `re.prefixmatch` names the same function",
        Some(Fix::safe_edit(rename_attribute(attribute, "prefixmatch"))),
        "Use `re.prefixmatch`",
    );
}

// Ruff's RUF055 rewrites a `re.match` call with a plain-text pattern whose result is only tested for truth into
// `str.startswith`, a fix that conflicts with this rule's, so such a call is left to it.
fn left_to_ruff(walker: &Walker, expr: &Expr) -> bool {
    let expressions = walker.expressions();
    let [outer @ .., Expr::Call(call), _] = expressions else {
        return false;
    };
    std::ptr::eq(call.func.as_ref(), expr)
        && call.arguments.len() == 2
        && call.arguments.find_argument_value("string", 1).is_some()
        && call.arguments.find_argument_value("pattern", 0).is_some_and(|pattern| is_plain_text(walker, pattern))
        && (is_compared_to_none(outer.last().copied()) || walker.is_boolean_test(1))
}

// The pattern RUF055 reads: a string or bytes literal, or a name bound once to one, with no regular expression
// metacharacter in it.
fn is_plain_text(walker: &Walker, pattern: &Expr) -> bool {
    const METACHARACTERS: [char; 12] = ['.', '^', '$', '*', '+', '?', '{', '[', '\\', '|', '(', ')'];
    let literal = match pattern {
        Expr::Name(name) => walker.checker.semantic().only_binding_value(name),
        _ => Some(pattern),
    };
    match literal {
        Some(Expr::StringLiteral(string)) => !string.value.to_str().contains(METACHARACTERS),
        Some(Expr::BytesLiteral(bytes)) => {
            !bytes.value.iter().any(|part| part.iter().any(|&byte| METACHARACTERS.contains(&char::from(byte))))
        }
        _ => false,
    }
}

// `call is None` or `call is not None`, which RUF055 rewrites along with the call.
fn is_compared_to_none(parent: Option<&Expr>) -> bool {
    parent
        .and_then(Expr::as_compare_expr)
        .and_then(ast::ExprCompare::as_single)
        .is_some_and(|(_, op, right)| matches!(op, CmpOp::Is | CmpOp::IsNot) && right.is_none_literal_expr())
}

// SMP550 turns a call whose pattern is anchored at its end into `re.fullmatch`, a fix that conflicts with the rename,
// so while it is selected it takes such a call.
fn left_to_anchored_regex_match(walker: &Walker, expr: &Expr) -> bool {
    let checker = walker.checker;
    checker.settings.rules.contains(Rule::AnchoredRegexMatch)
        && walker
            .expression_parent()
            .and_then(Expr::as_call_expr)
            .is_some_and(|call| std::ptr::eq(call.func.as_ref(), expr) && anchored_call(checker, call).is_some())
}
