use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::project::constructors::{CallContexts, CallSites, Use, Uses};
use crate::rules::public_api::Need;
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
    let mut walk =
        Uses::new(|expr: &Expr| owners.get(&expr.as_name_expr()?.range).copied()).with_source(checker.source);
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

pub(crate) fn method_uses<'a>(
    checker: &Checker<'a>,
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
    })
    .with_source(checker.source);
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

// A star import of this module can call its functions under names no file spells out, so none is judged. A module
// only tests import serves them, so its calls are test code.
pub(crate) fn judges_call_sites(checker: &Checker) -> bool {
    let project = checker.project();
    !(checker.is_test_module() || project.calls().is_test_support() || project.is_star_import_source())
}

pub(crate) struct OwnCalls<'a> {
    uses: FxHashMap<&'a str, Vec<Use<'a>>>,
    contexts: CallContexts,
}

impl<'a> OwnCalls<'a> {
    pub(crate) fn of(checker: &'a Checker) -> Self {
        Self {
            uses: module_uses(checker),
            contexts: CallContexts::of(checker.body(), checker.file_name() == "__main__.py"),
        }
    }

    pub(crate) fn function(&self, checker: &Checker, name: &str) -> CallSites {
        let mut sites = checker.project().calls().call_sites(name).cloned().unwrap_or_default();
        self.add(&mut sites, self.uses.get(name).map_or(&[][..], Vec::as_slice));
        sites
    }

    pub(crate) fn add(&self, sites: &mut CallSites, found: &[Use]) {
        for found in found {
            match found {
                Use::Call(call) => {
                    let (discarded, entry) = self.contexts.classify(call);
                    sites.add_production_call(call, discarded, entry);
                }
                Use::Opaque => sites.opaque = true,
            }
        }
    }
}

// The module-level functions the call-site rules judge. Code outside the run may call a name `__all__` lists, so those
// are not judged, whatever the project kind.
pub(crate) fn candidate_functions<'a>(checker: &'a Checker) -> impl Iterator<Item = &'a ast::StmtFunctionDef> {
    checker.body().iter().filter_map(Stmt::as_function_def_stmt).filter(move |function| {
        !checker.dunder_all().lists(function.name.as_str()) && module_candidate(checker, function)
    })
}

// A module-level class whose methods a call through `self` can reach, with no base a subclass or a framework may
// override it from.
pub(crate) fn is_closed_class(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.bases().is_empty() && class.keywords().is_empty() && !checker.project().is_base_class(class.name.as_str())
}

// A method whose every load is a call through `self` of its class, so the uses found are all of its calls. The rules
// that judge methods add these uses to their own counts, which differ.
pub(crate) fn self_only_uses<'u, 'a>(
    checker: &Checker,
    name: &str,
    found: &'u [(bool, Use<'a>)],
) -> Option<impl Iterator<Item = &'u Use<'a>>> {
    let every_load_is_self = found.len() == checker.project().calls().attribute_loads(name) as usize
        && found.iter().all(|(is_self, _)| *is_self);
    every_load_is_self.then(|| found.iter().map(|(_, found)| found))
}
