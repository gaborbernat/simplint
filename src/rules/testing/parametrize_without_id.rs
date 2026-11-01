use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

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
            for element in elts.iter().filter(|element| !is_param_with_id(checker.semantic(), element)) {
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
