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

use crate::lint::walker::{Scope, Walker};
use crate::syntax::last_segment;

pub(crate) use catch_all_module::catch_all_module;
pub(crate) use entrypoint_module_name::entrypoint_module_name;
pub(crate) use module_variable_case::module_variable_case;
pub(crate) use unguarded_entrypoint_call::unguarded_entrypoint_call;

// The names the naming rules reported in each def, class or lambda, by its range, the module's an empty one: each name
// is reported once per scope, at its first binding.
#[derive(Default)]
pub(crate) struct ReportedNames<'a>(RefCell<FxHashSet<(TextRange, &'a str)>>);

impl<'a> ReportedNames<'a> {
    // Whether `name` is new in `scope`, which records it as reported.
    fn first_in_scope(&self, scope: TextRange, name: &'a str) -> bool {
        self.0.borrow_mut().insert((scope, name))
    }
}

// The check a naming rule runs on each name a binding site introduces, with the scope it binds in and its range.
type NameCheck<'a> = fn(&Walker<'_, 'a>, TextRange, &'a str, TextRange);

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
        Scope::Function(function)
            if !exempt(function)
                && !function
                    .decorator_list
                    .iter()
                    .any(|decorator| last_segment(&decorator.expression) == Some("override")) =>
        {
            &*function.parameters
        }
        Scope::Lambda(ast::ExprLambda { parameters: Some(parameters), .. }) => parameters,
        _ => return,
    };
    for parameter in parameters {
        check(walker, scope.range(), parameter.name().as_str(), parameter.name().range);
    }
}

fn except_name<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler, check: NameCheck<'a>) {
    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), .. }) = handler else {
        return;
    };
    check(walker, current(walker), name.as_str(), name.range);
}

fn pattern_name<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern, check: NameCheck<'a>) {
    if let Pattern::MatchAs(ast::PatternMatchAs { name: Some(name), .. })
    | Pattern::MatchStar(ast::PatternMatchStar { name: Some(name), .. })
    | Pattern::MatchMapping(ast::PatternMatchMapping { rest: Some(name), .. }) = pattern
    {
        check(walker, current(walker), name.as_str(), name.range);
    }
}
