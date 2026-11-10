mod all_format;
mod all_not_last;
mod init_module_content;
mod missing_from_all;
mod module_variable_prefix;
mod test_only_code;
mod test_only_parameter;
mod unexported_public_name;
mod unused_definition;
mod unused_export;
mod unused_return_value;

pub(crate) use all_format::all_format;
pub(crate) use all_not_last::all_not_last;
pub(crate) use init_module_content::{init_module_content, is_type_checking};
pub(crate) use missing_from_all::missing_from_all;
pub(crate) use module_variable_prefix::module_variable_prefix;
pub(crate) use test_only_code::test_only_code;
pub(crate) use test_only_parameter::test_only_parameter;
pub(crate) use unexported_public_name::unexported_public_name;
pub(crate) use unused_definition::unused_definition;
pub(crate) use unused_export::unused_export;
pub(crate) use unused_return_value::unused_return_value;

use ruff_python_ast as ast;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::project::constructors::{CallContexts, CallSites, Use};
use crate::rules::call_sites::{module_candidate, module_uses};
use crate::syntax::{Exports, ReturnVisitor};

// The one-name-per-line layout `ruff format` gives a list with a trailing comma, so the fix needs no formatter pass.
fn exploded(checker: &Checker, base: &str, items: &[String]) -> String {
    let indent = checker.stylist.indentation().as_str();
    let eol = checker.stylist.line_ending().as_str();
    let lines = items.iter().fold(String::new(), |text, item| text + base + indent + item + "," + eol);
    format!("[{eol}{lines}{base}]")
}

// A star import of this module can call its functions under names no file spells out, so none is judged. A module
// only tests import serves them, so its calls are test code.
pub(super) fn judges_call_sites(checker: &Checker) -> bool {
    let project = checker.project();
    !(checker.is_test_module() || project.calls().is_test_support() || project.is_star_import_source())
}

// The module uses the name outside its own definition, so the module needs it whatever the tests do.
pub(super) fn references_outside(checker: &Checker, name: &str, definition: TextRange) -> bool {
    checker.semantic().references_to(name).iter().any(|reference| !definition.contains_range(reference.range))
}

// How this module calls its own functions: the uses of each by name, and where those calls sit.
pub(super) struct OwnCalls<'a> {
    uses: FxHashMap<&'a str, Vec<Use<'a>>>,
    contexts: CallContexts,
}

impl<'a> OwnCalls<'a> {
    pub(super) fn of(checker: &'a Checker) -> Self {
        Self {
            uses: module_uses(checker),
            contexts: CallContexts::of(checker.body(), checker.file_name() == "__main__.py"),
        }
    }

    // The calls a module-level function of this module gets: the calls other files make through the index, and the
    // calls this module makes, which are production code.
    pub(super) fn function(&self, checker: &Checker, name: &str) -> CallSites {
        let mut sites = checker.project().calls().call_sites(name).cloned().unwrap_or_default();
        self.add(&mut sites, self.uses.get(name).map_or(&[][..], Vec::as_slice));
        sites
    }

    pub(super) fn add(&self, sites: &mut CallSites, found: &[Use]) {
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

// The function's `return` statements hand back a value other than `None`, which a caller may read. A method that only
// returns `self` offers chaining, a style of its interface rather than a value, so it counts as returning none.
pub(super) fn returns_value(function: &ast::StmtFunctionDef) -> bool {
    let mut returns = ReturnVisitor::default();
    returns.visit_body(&function.body);
    let values: Vec<&ast::Expr> = returns
        .returns
        .iter()
        .filter_map(|stmt| stmt.value.as_deref())
        .filter(|value| !value.is_none_literal_expr())
        .collect();
    !values.is_empty()
        && !values.iter().all(|value| value.as_name_expr().is_some_and(|name| name.id.as_str() == "self"))
}

// A module-level class whose methods a call through `self` can reach, with no base a subclass or a framework may
// override it from.
pub(super) fn is_closed_class(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.bases().is_empty() && class.keywords().is_empty() && !checker.project().is_base_class(class.name.as_str())
}

pub(super) fn listed_exports<'a>(checker: &'a Checker) -> &'a [&'a str] {
    match checker.dunder_all() {
        Exports::Listed(names) => names.as_slice(),
        Exports::Absent | Exports::Dynamic => &[],
    }
}

// The module-level functions the call-site rules judge. Code outside the run may call a name `__all__` lists, so those
// are not judged, whatever the project kind.
pub(super) fn candidate_functions<'a>(checker: &'a Checker) -> impl Iterator<Item = &'a ast::StmtFunctionDef> {
    let listed = listed_exports(checker);
    checker
        .body()
        .iter()
        .filter_map(ast::Stmt::as_function_def_stmt)
        .filter(move |function| !listed.contains(&function.name.as_str()) && module_candidate(checker, function))
}
