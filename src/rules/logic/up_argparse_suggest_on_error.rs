use ruff_diagnostics::Applicability;
use ruff_python_ast::{self as ast, Expr};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::report_redundant_keyword;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    // Python 3.15 turns `suggest_on_error` on by default, so the keyword set to `True` changes nothing from there.
    let Some(keyword) = call
        .arguments
        .find_keyword("suggest_on_error")
        .filter(|keyword| matches!(keyword.value, Expr::BooleanLiteral(ast::ExprBooleanLiteral { value: true, .. })))
    else {
        return;
    };
    let checker = walker.checker;
    if !checker.semantic().resolves_to(&call.func, &["argparse", "ArgumentParser"]) {
        return;
    }
    report_redundant_keyword(
        checker,
        call,
        keyword,
        Rule::ArgparseSuggestOnError,
        Applicability::Safe,
        "`suggest_on_error=True` is the default from Python 3.15",
        "Drop `suggest_on_error=True`",
    );
}
