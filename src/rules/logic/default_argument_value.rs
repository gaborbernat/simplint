use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::CallSites;
use crate::rules::call_sites::{always_passes, module_sites, module_uses, resolvable};
use crate::rules::edits::argument_removal;
use crate::syntax::ExprCollector;

pub(crate) fn default_argument_value(checker: &Checker) {
    let functions: FxHashMap<TextRange, &ast::StmtFunctionDef> = checker
        .body()
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|function| {
            resolvable(checker, function)
                && function
                    .parameters
                    .iter_non_variadic_params()
                    .any(|param| param.default.as_deref().is_some_and(is_literal))
        })
        .flat_map(|function| {
            checker
                .semantic()
                .references_to(function.name.as_str())
                .iter()
                .map(move |reference| (reference.range, function))
        })
        .collect();
    if functions.is_empty() {
        return;
    }
    let mut calls = ExprCollector::new(|expr: &Expr| {
        let call = expr.as_call_expr()?;
        Some((call, *functions.get(&call.func.as_name_expr()?.range)?))
    });
    calls.visit_body(checker.body());
    let uses = module_uses(checker);
    let mut sites: FxHashMap<&str, Option<CallSites>> = FxHashMap::default();
    for (call, function) in calls.found {
        let arguments = &call.arguments;
        if arguments.args.iter().any(Expr::is_starred_expr)
            || arguments.keywords.iter().any(|keyword| keyword.arg.is_none())
        {
            continue;
        }
        let name = function.name.as_str();
        let all = sites.entry(name).or_insert_with(|| module_sites(checker, function, &uses));
        for (param, argument, range) in passed_defaults(checker, function, call) {
            // When every call passes the parameter, `unused-default` reports its default instead.
            if all.as_ref().is_some_and(|all| always_passes(all, &function.parameters, param, 0)) {
                continue;
            }
            let param = param.parameter.name.as_str();
            checker.report_fix(
                Rule::DefaultArgumentValue,
                range,
                format!("`{}` is the default of `{param}` in `{name}`; leave it out", checker.text(argument.range())),
                argument_removal(checker, call, range).map(|removed| Fix::safe_edit(Edit::range_deletion(removed))),
                format!("Remove the argument for `{param}`"),
            );
        }
    }
}

// A literal evaluates to an equal value at each call, so passing it again matches the default. A name or a call may
// not, as its value can change between the definition and the call.
fn is_literal(expr: &Expr) -> bool {
    match expr {
        Expr::NoneLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub | UnaryOp::UAdd, operand, .. }) => {
            operand.is_number_literal_expr()
        }
        _ => false,
    }
}

// The arguments of `call` that pass a parameter its literal default: each keyword one, and the last positional one, as
// leaving out an earlier one would shift the rest onto other parameters.
fn passed_defaults<'a>(
    checker: &Checker,
    function: &'a ast::StmtFunctionDef,
    call: &'a ast::ExprCall,
) -> Vec<(&'a ast::ParameterWithDefault, &'a Expr, TextRange)> {
    let parameters = &function.parameters;
    let positional: Vec<&ast::ParameterWithDefault> = parameters.posonlyargs.iter().chain(&parameters.args).collect();
    let arguments = &call.arguments;
    let mut found = Vec::new();
    if let Some((index, last)) = arguments.args.iter().enumerate().last()
        && let Some(param) = positional.get(index)
        && is_default(param, last)
    {
        found.push((*param, last, checker.parenthesized_range(last, arguments.into())));
    }
    for keyword in &arguments.keywords {
        let arg = keyword.arg.as_ref().expect("calls that unpack `**` are skipped");
        if let Some(param) = parameters
            .args
            .iter()
            .chain(&parameters.kwonlyargs)
            .find(|param| param.parameter.name.as_str() == arg.as_str())
            && is_default(param, &keyword.value)
        {
            found.push((param, &keyword.value, keyword.range()));
        }
    }
    found
}

fn is_default(param: &ast::ParameterWithDefault, argument: &Expr) -> bool {
    param
        .default
        .as_deref()
        .is_some_and(|default| is_literal(default) && ComparableExpr::from(default) == ComparableExpr::from(argument))
}
