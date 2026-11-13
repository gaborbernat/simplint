use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::callee::Callee;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return;
    };
    let args = &arguments.args;
    if args.len() < 2 || args.iter().any(Expr::is_starred_expr) {
        return;
    }
    let Some(callee) = Callee::of(walker, func) else {
        return;
    };
    if callee.is_decorated() {
        return;
    }
    let parameters = callee.positional();
    // An argument named after another parameter that this call also fills by position went to the wrong slot.
    let filled = &parameters[..args.len().min(parameters.len())];
    let misplaced: Vec<(&str, &str)> = args
        .iter()
        .zip(filled)
        .filter_map(|(arg, parameter)| {
            let name = arg.as_name_expr()?.id.as_str();
            (name != *parameter && filled.contains(&name)).then_some((name, *parameter))
        })
        .collect();
    // Arguments that only trade places among themselves are a deliberate swap, as a test of a symmetric function or a
    // call that flips roles does.
    let given: FxHashSet<&str> = misplaced.iter().map(|(argument, _)| *argument).collect();
    if misplaced.len() < 2 || misplaced.iter().all(|(_, parameter)| given.contains(parameter)) {
        return;
    }
    let pairs = misplaced
        .iter()
        .map(|(argument, parameter)| format!("`{argument}` goes to `{parameter}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let name = callee.name();
    walker.checker.report(
        Rule::MisplacedArguments,
        arguments.range(),
        format!(
            "arguments of `{name}` are out of order: {pairs}; pass them in the order of the parameters, or by keyword"
        ),
    );
}
