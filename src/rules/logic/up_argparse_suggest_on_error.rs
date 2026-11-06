use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::keyword_removal;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // Python 3.15 turns `suggest_on_error` on by default, so the keyword set to `True` changes nothing from there.
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(keyword) = call
        .arguments
        .find_keyword("suggest_on_error")
        .filter(|keyword| matches!(keyword.value, Expr::BooleanLiteral(ast::ExprBooleanLiteral { value: true, .. })))
    else {
        return;
    };
    if !checker.semantic().resolves_to(&call.func, &["argparse", "ArgumentParser"]) {
        return;
    }
    let fix = keyword_removal(checker, call, keyword).map(|range| Fix::safe_edit(Edit::range_deletion(range)));
    checker.report_fix(
        Rule::ArgparseSuggestOnError,
        keyword.range(),
        "`suggest_on_error=True` is the default from Python 3.15",
        fix,
        "Drop `suggest_on_error=True`",
    );
}
