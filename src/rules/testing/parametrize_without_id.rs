use ruff_python_ast::{self as ast, Expr, UnaryOp};
use ruff_text_size::Ranged;
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::Semantic;

pub fn parametrize_without_id(checker: &Checker) {
    for function in checker.function_defs() {
        for decorator in &function.decorator_list {
            let Expr::Call(ast::ExprCall { func, arguments, .. }) = &decorator.expression else {
                continue;
            };
            if !checker.semantic().resolves_to(func, &["pytest", "mark", "parametrize"]) {
                continue;
            }
            let Some(values) = arguments.find_argument_value("argvalues", 1) else {
                continue;
            };
            if arguments.find_argument_value("ids", 3).is_some() {
                continue;
            }
            let (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = values else {
                continue;
            };
            let mut id_counts = FxHashMap::default();
            for id in elts.iter().filter_map(|element| scalar_id(checker, element)) {
                *id_counts.entry(id).or_insert(0) += 1;
            }
            for element in elts.iter().filter(|element| !is_param_with_id(checker.semantic(), element)) {
                if scalar_id(checker, element).is_some_and(|id| id_counts.get(id) == Some(&1)) {
                    continue;
                }
                checker.report(
                    Rule::ParametrizeWithoutId,
                    element.range(),
                    "Parametrize row has no id; wrap it in `pytest.param(..., id=...)`",
                );
            }
        }
    }
}

fn is_param_with_id(semantic: &Semantic, element: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = element else {
        return false;
    };
    semantic.resolves_to(func, &["pytest", "param"])
        && arguments.find_keyword("id").is_some_and(|keyword| !keyword.value.is_none_literal_expr())
}

// Pytest uses a scalar's value as its id, but escapes non-printable ASCII and backslashes in strings, and suffixes
// ids that several rows share.
fn scalar_id<'a>(checker: &Checker<'a>, element: &'a Expr) -> Option<&'a str> {
    match element {
        Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
            let text = value.to_str();
            text.bytes().all(|byte| matches!(byte, b' '..=b'~') && byte != b'\\').then_some(text)
        }
        Expr::NumberLiteral(_) | Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) => Some(checker.text(element)),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub, operand, .. }) if operand.is_number_literal_expr() => {
            Some(checker.text(element))
        }
        _ => None,
    }
}
