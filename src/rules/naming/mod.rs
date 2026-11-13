mod catch_all_module;
mod entrypoint_module_name;
pub(crate) mod meaningless_name;
mod module_variable_case;
pub(crate) mod single_letter_name;
mod unguarded_entrypoint_call;

use std::cell::RefCell;

use ruff_python_ast::{self as ast, Pattern};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::{has_decorator, pattern_capture};

pub(crate) use catch_all_module::catch_all_module;
pub(crate) use entrypoint_module_name::entrypoint_module_name;
pub(crate) use module_variable_case::module_variable_case;
pub(crate) use unguarded_entrypoint_call::unguarded_entrypoint_call;

fn script_targets<'a>(checker: &'a Checker<'_>) -> Vec<&'a str> {
    if checker.settings.project.metadata.scripts.is_empty() {
        return Vec::new();
    }
    let Some(module) = checker.module_path().map(|path| path.join(".")).or_else(|| {
        let path = checker.path.strip_prefix(&checker.settings.project.root).ok()?;
        let path = path.strip_prefix("src").unwrap_or(path);
        Some(path.with_extension("").iter().map(|part| part.to_str()).collect::<Option<Vec<_>>>()?.join("."))
    }) else {
        return Vec::new();
    };
    let module = module.strip_suffix(".__init__").unwrap_or(&module);
    checker
        .settings
        .project
        .metadata
        .scripts
        .iter()
        .filter_map(|entry| entry.split_once(':'))
        .filter(|(entry_module, _)| entry_module.trim() == module)
        .filter_map(|(_, target)| target.split_whitespace().next())
        .collect()
}

// Each def, class and lambda, keyed by its range (the module's is empty), reports a name once, at its first binding.
#[derive(Default)]
pub(crate) struct ReportedNames<'a>(RefCell<FxHashSet<(TextRange, &'a str)>>);

impl<'a> ReportedNames<'a> {
    fn first_in_scope(&self, scope: TextRange, name: &'a str) -> bool {
        self.0.borrow_mut().insert((scope, name))
    }
}

// A name as a binding site introduces it, with the range of the name itself.
#[derive(Clone, Copy)]
struct NameSite<'a> {
    text: &'a str,
    range: TextRange,
}

// The check a naming rule runs on each name a binding site introduces, with the scope it binds in.
type NameCheck<'a> = fn(&Walker<'_, 'a>, TextRange, NameSite<'a>);

fn current(walker: &Walker) -> TextRange {
    walker.scopes().last().map_or_else(TextRange::default, Ranged::range)
}

// `exempt` skips the functions whose parameter names a rule leaves alone. An override must keep the names of the method
// it replaces.
fn parameters<'a>(
    walker: &mut Walker<'_, 'a>,
    scope: &Scope<'a>,
    exempt: fn(&ast::StmtFunctionDef) -> bool,
    check: NameCheck<'a>,
) {
    let parameters = match scope {
        Scope::Function(function) if !exempt(function) && !has_decorator(&function.decorator_list, &["override"]) => {
            &*function.parameters
        }
        Scope::Lambda(ast::ExprLambda { parameters: Some(parameters), .. }) => parameters,
        _ => return,
    };
    for parameter in parameters {
        let site = NameSite { text: parameter.name().as_str(), range: parameter.name().range };
        check(walker, scope.range(), site);
    }
}

fn except_name<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler, check: NameCheck<'a>) {
    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), .. }) = handler else {
        return;
    };
    check(walker, current(walker), NameSite { text: name.as_str(), range: name.range });
}

fn pattern_name<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern, check: NameCheck<'a>) {
    if let Some(name) = pattern_capture(pattern) {
        check(walker, current(walker), NameSite { text: name.as_str(), range: name.range });
    }
}
