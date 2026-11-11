use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallShape, CallSites};
use crate::rules::call_sites::{OwnCalls, candidate_functions, judges_call_sites};

pub fn test_only_parameter(checker: &Checker) {
    if !judges_call_sites(checker) {
        return;
    }
    let own = OwnCalls::of(checker);
    for function in candidate_functions(checker) {
        let sites = own.function(checker, function.name.as_str());
        // A call that unpacks `**kwargs` may pass any parameter, and a call the index cannot see hides what it passes.
        // A program's entry gets its arguments from the command line, so leaving one out there says nothing.
        if sites.opaque
            || sites.entry
            || sites.production.is_empty()
            || sites.production.iter().any(|shape| shape.unpacks_mapping)
        {
            continue;
        }
        report_parameters(checker, function, &sites);
    }
}

// Reports each default that no production call passes while a test call does. Only a default can be left out, so a
// required parameter is never reported.
fn report_parameters(checker: &Checker, function: &ast::StmtFunctionDef, sites: &CallSites) {
    let parameters = &function.parameters;
    let positional_only = parameters.posonlyargs.len();
    for (index, param) in parameters.posonlyargs.iter().chain(&parameters.args).enumerate() {
        let name = param.parameter.name.as_str();
        let passed = |shape: &CallShape| {
            index < shape.positional as usize
                || (index >= positional_only && shape.keywords.iter().any(|keyword| keyword == name))
        };
        if param.default.is_some() {
            report_if_test_only(checker, function, param, sites, passed);
        }
    }
    for param in &parameters.kwonlyargs {
        let name = param.parameter.name.as_str();
        let passed = |shape: &CallShape| shape.keywords.iter().any(|keyword| keyword == name);
        if param.default.is_some() {
            report_if_test_only(checker, function, param, sites, passed);
        }
    }
}

fn report_if_test_only(
    checker: &Checker,
    function: &ast::StmtFunctionDef,
    param: &ast::ParameterWithDefault,
    sites: &CallSites,
    passed: impl Fn(&CallShape) -> bool,
) {
    if sites.production.iter().any(&passed) || !sites.shapes.iter().any(&passed) {
        return;
    }
    let name = param.parameter.name.as_str();
    checker.report(
        Rule::TestOnlyParameter,
        param.parameter.name.range(),
        format!("Only tests pass `{name}` to `{}`; production calls leave it at its default", function.name.as_str()),
    );
}
