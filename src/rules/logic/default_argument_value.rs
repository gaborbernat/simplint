use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallSites, Use};
use crate::rules::call_sites::{add_use, module_candidate, module_uses};
use crate::rules::edits::argument_removal;
use crate::syntax::{ExprCollector, Visibility};

pub(crate) fn default_argument_value(checker: &Checker) {
    let semantic = checker.semantic();
    let functions: FxHashMap<TextRange, &ast::StmtFunctionDef> = checker
        .body()
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|function| {
            let name = function.name.as_str();
            // A decorator may change the signature the call reaches, and a rebound name reaches another function.
            function.decorator_list.is_empty()
                && Visibility::of(name) != Visibility::Dunder
                && !semantic.is_rebound(name)
                && function.parameters.iter_non_variadic_params().any(|param| param.default.as_deref().is_some_and(is_literal))
        })
        .flat_map(|function| {
            semantic.references_to(function.name.as_str()).iter().map(move |reference| (reference.range, function))
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
    let mut sites: FxHashMap<&str, CallSites> = FxHashMap::default();
    for (call, function) in calls.found {
        let arguments = &call.arguments;
        if arguments.args.iter().any(Expr::is_starred_expr) || arguments.keywords.iter().any(|keyword| keyword.arg.is_none())
        {
            continue;
        }
        let name = function.name.as_str();
        let all = sites.entry(name).or_insert_with(|| function_sites(checker, function, &uses));
        for (param, argument, range) in passed_defaults(checker, function, call) {
            if always_passed(all, function, param) {
                continue;
            }
            let fix = argument_removal(checker, call, range).map(|removed| Fix::safe_edit(Edit::range_deletion(removed)));
            let param = param.parameter.name.as_str();
            checker.report_fix(
                Rule::DefaultArgumentValue,
                range,
                format!("`{}` is the default of `{param}` in `{name}`; leave it out", checker.text(argument.range())),
                fix,
                format!("Remove the argument for `{param}`"),
            );
        }
    }
}

// The calls `unused-default` judges: when each of them passes the parameter, that rule reports its default instead.
fn function_sites(
    checker: &Checker,
    function: &ast::StmtFunctionDef,
    uses: &FxHashMap<&str, Vec<Use>>,
) -> CallSites {
    let name = function.name.as_str();
    if !module_candidate(checker, function) {
        return CallSites { opaque: true, ..CallSites::default() };
    }
    let mut sites = checker.project().calls().call_sites(name).cloned().unwrap_or_default();
    for found in uses.get(name).into_iter().flatten() {
        add_use(&mut sites, found);
    }
    sites
}

fn always_passed(sites: &CallSites, function: &ast::StmtFunctionDef, param: &ast::ParameterWithDefault) -> bool {
    if sites.opaque || sites.shapes.is_empty() || sites.shapes.iter().any(|shape| shape.unpacks_mapping) {
        return false;
    }
    let parameters = &function.parameters;
    let name = param.parameter.name.as_str();
    let position = parameters.posonlyargs.iter().chain(&parameters.args).position(|other| std::ptr::eq(other, param));
    let keyword = !parameters.posonlyargs.iter().any(|other| std::ptr::eq(other, param));
    sites.shapes.iter().all(|shape| {
        position.is_some_and(|position| position < shape.positional as usize)
            || keyword && shape.keywords.iter().any(|passed| passed == name)
    })
}

// The arguments of `call` that pass a parameter its literal default: each keyword one, and the last positional one, as
// leaving out an earlier one would shift the rest onto other parameters.
fn passed_defaults<'a>(
    checker: &Checker,
    function: &'a ast::StmtFunctionDef,
    call: &'a ast::ExprCall,
) -> Vec<(&'a ast::ParameterWithDefault, &'a Expr, TextRange)> {
    let parameters = &function.parameters;
    let mut found = Vec::new();
    let positional: Vec<&ast::ParameterWithDefault> = parameters.posonlyargs.iter().chain(&parameters.args).collect();
    let arguments = &call.arguments;
    if let Some((index, last)) = arguments.args.iter().enumerate().last()
        && let Some(param) = positional.get(index)
        && is_default(param, last)
    {
        found.push((*param, last, checker.parenthesized_range(last, arguments.into())));
    }
    for keyword in &arguments.keywords {
        let Some(arg) = &keyword.arg else { continue };
        let param = parameters
            .args
            .iter()
            .chain(&parameters.kwonlyargs)
            .find(|param| param.parameter.name.as_str() == arg.as_str());
        if let Some(param) = param
            && is_default(param, &keyword.value)
        {
            found.push((param, &keyword.value, keyword.range()));
        }
    }
    found
}

fn is_default(param: &ast::ParameterWithDefault, argument: &Expr) -> bool {
    param.default.as_deref().is_some_and(|default| is_literal(default) && ComparableExpr::from(default) == ComparableExpr::from(argument))
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
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub | UnaryOp::UAdd, operand, .. }) => operand.is_number_literal_expr(),
        _ => false,
    }
}
