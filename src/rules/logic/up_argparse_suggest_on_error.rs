use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // Python 3.15 turns `suggest_on_error` on by default, so the keyword set to `True` changes nothing from there.
    if checker.settings.from_ruff.target_version < PythonVersion::PY315 {
        return;
    }
    let Expr::Call(call) = expr else {
        return;
    };
    if !checker.semantic().resolves_to(&call.func, &["argparse", "ArgumentParser"]) {
        return;
    }
    let Some(keyword) = call.arguments.keywords.iter().find(|keyword| {
        keyword.arg.as_ref().is_some_and(|name| name.as_str() == "suggest_on_error")
            && matches!(keyword.value, Expr::BooleanLiteral(ast::ExprBooleanLiteral { value: true, .. }))
    }) else {
        return;
    };
    let fix = removal(checker, call, keyword).map(|range| Fix::safe_edit(Edit::range_deletion(range)));
    checker.report_fix(
        Rule::ArgparseSuggestOnError,
        keyword.range(),
        "`suggest_on_error=True` is the default from Python 3.15",
        fix,
        "Drop `suggest_on_error=True`",
    );
}

// The keyword with the comma that separates it from a neighbor, so the rest of the call keeps its layout. A sole
// argument goes with its trailing comma, up to the closing parenthesis.
fn removal(checker: &Checker, call: &ast::ExprCall, keyword: &ast::Keyword) -> Option<TextRange> {
    if checker.has_comment_in(call.range()) {
        return None;
    }
    let mut arguments: Vec<TextRange> = call
        .arguments
        .args
        .iter()
        .map(Ranged::range)
        .chain(call.arguments.keywords.iter().map(Ranged::range))
        .collect();
    arguments.sort_by_key(Ranged::start);
    let index = arguments.iter().position(|range| *range == keyword.range())?;
    let range = keyword.range();
    let previous = index.checked_sub(1).map(|previous| arguments[previous]);
    Some(match (previous, arguments.get(index + 1)) {
        (_, Some(next)) => TextRange::new(range.start(), next.start()),
        (Some(previous), None) => TextRange::new(previous.end(), range.end()),
        (None, None) => TextRange::new(range.start(), call.range().end() - TextSize::from(1)),
    })
}
