use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallSites, Use};
use crate::rules::api::returns_value;
use crate::rules::call_sites::{
    OwnCalls, candidate_functions, is_closed_class, judges_call_sites, method_candidate, method_uses, self_only_uses,
};

pub fn unused_return_value(checker: &Checker) {
    if !judges_call_sites(checker) {
        return;
    }
    let own = OwnCalls::of(checker);
    for function in candidate_functions(checker).filter(|function| returns_value(function)) {
        let name = function.name.as_str();
        let sites = own.function(checker, name);
        report_if_unread(checker, function.name.range(), name, &sites);
    }
    for stmt in checker.body() {
        if let Stmt::ClassDef(class) = stmt
            && is_closed_class(checker, class)
        {
            let methods = method_uses(checker, class);
            for method in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                method_value(checker, class, method, &methods, &own);
            }
        }
    }
}

// A method of a closed class, judged by its calls through `self` when no other read of its name exists in the run, as
// `unused-default` judges them.
fn method_value(
    checker: &Checker,
    class: &ast::StmtClassDef,
    function: &ast::StmtFunctionDef,
    methods: &FxHashMap<&str, Vec<(bool, Use)>>,
    own: &OwnCalls,
) {
    let name = function.name.as_str();
    if !method_candidate(checker, function) || !returns_value(function) {
        return;
    }
    let Some(uses) = self_only_uses(checker, name, methods.get(name).map_or(&[][..], Vec::as_slice)) else {
        return;
    };
    let mut sites = CallSites::default();
    for found in uses {
        own.add(&mut sites, std::slice::from_ref(found));
    }
    let qualified = format!("{}.{name}", class.name.as_str());
    report_if_unread(checker, function.name.range(), &qualified, &sites);
}

// Reports a function that production code calls when no production call reads the value it returns, whether or not
// tests read it. A function only tests call is left to `test-only-code`, and one with no call to `unused-definition`;
// a program's entry hands its value to the command line.
fn report_if_unread(checker: &Checker, range: TextRange, name: &str, sites: &CallSites) {
    if sites.opaque || sites.entry || sites.production.is_empty() || sites.production_reads {
        return;
    }
    checker.report(
        Rule::UnusedReturnValue,
        range,
        format!("No production call reads the value `{name}` returns; return nothing, or let a caller use it"),
    );
}
