use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::project::constructors::{CallSites, Use, Uses};
use crate::rules::docstrings::Need;
use crate::syntax::Visibility;

// The uses of every candidate function by its name, found in one walk of the module rather than one per function.
pub(crate) fn module_uses<'a>(checker: &'a Checker) -> FxHashMap<&'a str, Vec<Use<'a>>> {
    let owners: FxHashMap<TextRange, &str> = checker
        .body()
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|function| module_candidate(checker, function))
        .flat_map(|function| {
            let name = function.name.as_str();
            checker.semantic().references_to(name).iter().map(move |reference| (reference.range, name))
        })
        .collect();
    if owners.is_empty() {
        return FxHashMap::default();
    }
    let mut walk = Uses::new(|expr: &Expr| owners.get(&expr.as_name_expr()?.range).copied());
    walk.visit_body(checker.body());
    let mut functions: FxHashMap<&str, Vec<Use>> = FxHashMap::default();
    for (name, found) in walk.found {
        functions.entry(name).or_default().push(found);
    }
    functions
}

pub(crate) fn module_candidate(checker: &Checker, function: &ast::StmtFunctionDef) -> bool {
    eligible(checker, function) && !checker.semantic().is_rebound(function.name.as_str())
}

// Decorators may register or wrap a function, Python calls dunders itself, and code outside the run may call a public
// function without the argument, so none of these is reported.
fn eligible(checker: &Checker, function: &ast::StmtFunctionDef) -> bool {
    function.decorator_list.is_empty()
        && Visibility::of(function.name.as_str()) != Visibility::Dunder
        && !checker.state().public_api(checker).covers(function.range(), Need::Any)
}

// The uses of each candidate method of a class by its name, whether through `self`, found in one walk of the class.
pub(crate) fn method_uses<'a>(
    checker: &Checker,
    class: &'a ast::StmtClassDef,
) -> FxHashMap<&'a str, Vec<(bool, Use<'a>)>> {
    let names: FxHashSet<&str> = class
        .body
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|function| method_candidate(checker, function))
        .map(|function| function.name.as_str())
        .collect();
    if names.is_empty() {
        return FxHashMap::default();
    }
    let mut walk = Uses::new(|expr: &Expr| {
        let attribute = expr.as_attribute_expr()?;
        let name = names.get(attribute.attr.as_str())?;
        let is_self = attribute.value.as_name_expr().is_some_and(|value| value.id.as_str() == "self");
        Some((*name, is_self))
    });
    walk.visit_body(&class.body);
    let mut methods: FxHashMap<&str, Vec<(bool, Use)>> = FxHashMap::default();
    for ((name, is_self), found) in walk.found {
        methods.entry(name).or_default().push((is_self, found));
    }
    methods
}

pub(crate) fn method_candidate(checker: &Checker, function: &ast::StmtFunctionDef) -> bool {
    eligible(checker, function) && !checker.project().is_attribute_assigned(function.name.as_str())
}

pub(crate) fn add_use(sites: &mut CallSites, found: &Use) {
    match found {
        Use::Call(call) => sites.add_call(call),
        Use::Opaque => sites.opaque = true,
    }
}
