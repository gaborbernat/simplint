use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallShape, CallSites, Use};
use crate::rules::call_sites::{add_use, method_candidate, method_uses, module_candidate, module_uses};

pub(crate) fn unused_default(checker: &Checker) {
    let project = checker.project();
    // A star import of this file can call its functions under names no file spells out, so none of them is known.
    if project.is_star_import_source() {
        return;
    }
    let functions = module_uses(checker);
    for stmt in checker.body() {
        match stmt {
            Stmt::FunctionDef(function) => module_function(checker, function, &functions),
            Stmt::ClassDef(class)
                if class.bases().is_empty()
                    && class.keywords().is_empty()
                    && !project.is_base_class(class.name.as_str()) =>
            {
                let methods = method_uses(checker, class);
                for method in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    method_of(checker, method, &methods);
                }
            }
            _ => {}
        }
    }
}

// A module-level function, called by the other files of the run through its module and in this file by its name.
fn module_function(checker: &Checker, function: &ast::StmtFunctionDef, functions: &FxHashMap<&str, Vec<Use>>) {
    let name = function.name.as_str();
    if !module_candidate(checker, function) {
        return;
    }
    let mut sites = checker.project().calls().call_sites(name).cloned().unwrap_or_default();
    for found in functions.get(name).into_iter().flatten() {
        add_use(&mut sites, found);
    }
    report(checker, function, name, &sites, false);
}

// A method of a module-level class without bases, called through `self` in its class. Another file's `obj.name` read
// would count as a load the class does not account for, so the method is judged only when every load of its name is
// one of the class's `self.name(...)` calls.
fn method_of(checker: &Checker, function: &ast::StmtFunctionDef, methods: &FxHashMap<&str, Vec<(bool, Use)>>) {
    let name = function.name.as_str();
    let project = checker.project();
    if !method_candidate(checker, function) {
        return;
    }
    let found = methods.get(name).map_or(&[][..], Vec::as_slice);
    if found.len() != project.calls().attribute_loads(name) as usize {
        return;
    }
    let mut sites = CallSites::default();
    for (is_self, found) in found {
        if !is_self {
            return;
        }
        add_use(&mut sites, found);
    }
    report(checker, function, name, &sites, true);
}

// Reports each default that every call passes. The receiver of a method is not a parameter of its calls, so `receiver`
// skips the first positional parameter.
fn report(checker: &Checker, function: &ast::StmtFunctionDef, name: &str, sites: &CallSites, receiver: bool) {
    if sites.opaque || sites.shapes.is_empty() || sites.shapes.iter().any(|shape| shape.unpacks_mapping) {
        return;
    }
    let parameters = &function.parameters;
    let positional: Vec<_> = parameters.posonlyargs.iter().chain(parameters.args.iter()).collect();
    let skip = usize::from(receiver);
    let mut defaulted_before = false;
    for (index, param) in positional.iter().enumerate().skip(skip) {
        let position = index - skip;
        let positional_only = index < parameters.posonlyargs.len();
        let passed = |shape: &CallShape| {
            position < shape.positional as usize
                || (!positional_only && shape.keywords.iter().any(|keyword| keyword == param_name(param)))
        };
        if let Some(default) = param.default.as_deref()
            && sites.shapes.iter().all(passed)
        {
            // Dropping a default after another defaulted parameter leaves a parameter without one after it, which breaks
            // the signature, so only defaults with no defaulted parameter before them are offered a fix.
            report_default(checker, name, param, default, !defaulted_before);
        }
        defaulted_before |= param.default.is_some();
    }
    for param in &parameters.kwonlyargs {
        let passed = |shape: &CallShape| shape.keywords.iter().any(|keyword| keyword == param_name(param));
        if let Some(default) = param.default.as_deref()
            && sites.shapes.iter().all(passed)
        {
            report_default(checker, name, param, default, true);
        }
    }
}

fn param_name(param: &ast::ParameterWithDefault) -> &str {
    param.parameter.name.as_str()
}

fn report_default(
    checker: &Checker,
    function: &str,
    param: &ast::ParameterWithDefault,
    default: &Expr,
    removable: bool,
) {
    let parameter = &param.parameter;
    let range = checker.parenthesized_range(default, param.into());
    let message = format!(
        "Every call to `{function}` passes `{}`; drop its default `{}`",
        param_name(param),
        checker.text(range)
    );
    let fix = removable.then(|| {
        let start = match parameter.annotation.as_deref() {
            Some(annotation) => checker.parenthesized_range(annotation, parameter.into()).end(),
            None => parameter.name.range.end(),
        };
        Fix::unsafe_edit(Edit::range_deletion(TextRange::new(start, range.end())))
    });
    checker.report_fix(
        Rule::UnusedDefault,
        range,
        message,
        fix,
        format!("Remove the default of `{}`", param_name(param)),
    );
}
