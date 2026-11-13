// The module-scope variables the case and prefix rules check. Names a framework or tool reads by their exact spelling,
// names a loop, `with`, `except` or `match` binds, and annotations alone under `if TYPE_CHECKING:` are left out.

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::TextRange;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::rules::frameworks::framework_names;
use crate::rules::typing::type_parameter_assignment;
use crate::rules::{is_class_like, is_sphinx_conf};
use crate::semantic::{Binding, BindingKind};
use crate::syntax::{Exports, Visibility, single_assignment, statement_index};

pub(crate) struct ModuleVariable<'a> {
    pub(crate) name: &'a str,
    pub(crate) range: TextRange,
    // Whether the name should have a leading underscore. An imported name keeps the underscore it has, and any other
    // name is private unless `__all__` lists it.
    pub(crate) private: bool,
}

// Module names a tool reads by name, which also decides the type their value must have: pytest takes `pytestmark` only
// as a list or a single mark.
pub(crate) const TOOL_READ_NAMES: &[&str] =
    &["pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob", "TYPE_CHECKING"];

// Sorted by position, so the rules report them in the order they appear.
pub(crate) fn module_variables<'a>(checker: &Checker<'a>) -> Vec<ModuleVariable<'a>> {
    if is_sphinx_conf(checker) {
        return Vec::new();
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => None,
        Exports::Listed(names) => Some(names),
        Exports::Dynamic => return Vec::new(),
    };
    let read_by_name = framework_names(checker);
    let aliases = function_aliases(checker);
    let type_parameters: FxHashSet<&str> = checker
        .body()
        .iter()
        .filter_map(|stmt| type_parameter_assignment(checker, stmt))
        .map(|target| target.id.as_str())
        .collect();
    let stub = |binding: &Binding| checker.semantic().is_type_checking_stub(binding.range);
    let mut variables = Vec::new();
    for (name, bindings) in checker.semantic().all_bindings() {
        let Some(first) = bindings.iter().find(|binding| !stub(binding)) else {
            continue;
        };
        if is_dunder(name)
            || is_class_like(name)
            || TOOL_READ_NAMES.contains(&name)
            || read_by_name.contains(name)
            || aliases.contains(name)
            || (type_parameters.contains(name)
                && bindings.iter().filter(|binding| !stub(binding)).all(|binding| binding.range == first.range))
            || checker.settings.project.knobs.keep_names.iter().any(|kept| kept == name)
            || bindings.iter().any(|binding| {
                matches!(
                    binding.kind,
                    BindingKind::Import | BindingKind::Function | BindingKind::Class | BindingKind::TypeAlias
                )
            })
            || bindings.iter().filter(|binding| !stub(binding)).all(|binding| binding.kind == BindingKind::Target)
            || is_block_local(checker, name, bindings)
        {
            continue;
        }
        let private = if checker.project().is_used_elsewhere(name) {
            Visibility::of(name) != Visibility::Public
        } else {
            !exported.is_some_and(|names| names.contains(name))
        };
        variables.push(ModuleVariable { name, range: first.range, private });
    }
    variables.sort_by_key(|variable| variable.range.start());
    variables
}

// The module-scope names bound to a callable, as `_build_cli = build_cli`, `to_json = dumps` or `encode =
// _codec.encode`. Such an alias reads as the callable and is called like it, so it follows the callable's name as a
// class alias does: a function the module defines, a lowercase imported name, a lowercase attribute of an imported
// module, and a method of an instance of a class the module defines. An uppercase name or attribute reads as a constant.
fn function_aliases<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    checker
        .body()
        .iter()
        .filter_map(single_assignment)
        .filter(|(_, _, value)| is_callable_alias(checker, value))
        .map(|(target, _, _)| target.id.as_str())
        .collect()
}

fn is_callable_alias(checker: &Checker, value: &Expr) -> bool {
    let semantic = checker.semantic();
    let bound_as = |name: &str, kind: BindingKind| semantic.bindings(name).iter().any(|binding| binding.kind == kind);
    match value {
        Expr::Name(name) => {
            let name = name.id.as_str();
            bound_as(name, BindingKind::Function) || (is_lowercase(name) && bound_as(name, BindingKind::Import))
        }
        Expr::Attribute(attribute) if is_lowercase(attribute.attr.as_str()) => {
            let Expr::Name(receiver) = attribute.value.as_ref() else {
                return false;
            };
            bound_as(receiver.id.as_str(), BindingKind::Import)
                || instance_class(checker, receiver).is_some_and(|class| {
                    class
                        .body
                        .iter()
                        .filter_map(Stmt::as_function_def_stmt)
                        .any(|method| method.name.as_str() == attribute.attr.as_str())
                })
        }
        _ => false,
    }
}

// The class the module defines whose call gives `name` its only value, as `_codec = _Codec()` does.
fn instance_class<'a>(checker: &Checker<'a>, name: &ast::ExprName) -> Option<&'a ast::StmtClassDef> {
    let name = name.id.as_str();
    if checker.semantic().is_rebound(name) {
        return None;
    }
    let body = checker.body();
    let class = body.iter().filter_map(single_assignment).find_map(|(target, _, value)| match value {
        Expr::Call(ast::ExprCall { func, .. }) if target.id == name => func.as_name_expr(),
        _ => None,
    })?;
    body.iter().filter_map(Stmt::as_class_def_stmt).find(|candidate| candidate.name.id == class.id)
}

fn is_lowercase(name: &str) -> bool {
    name.chars().any(char::is_lowercase)
}

// A name that one module-level `if`, `try` or `with` block binds and reads, and nothing outside the block reads,
// works as a local of that block, as `msg` does before a `raise` in a version check. A name read by another file
// stays a module variable.
fn is_block_local(checker: &Checker, name: &str, bindings: &[Binding]) -> bool {
    let body = checker.body();
    let references = checker.semantic().references_to(name);
    let Some(block) = references.first().map(|reference| reference.statement) else {
        return false;
    };
    matches!(body.get(block), Some(Stmt::If(_) | Stmt::Try(_) | Stmt::With(_)))
        && references.iter().all(|reference| reference.statement == block)
        && bindings.iter().all(|binding| statement_index(body, binding.range.start()) == block)
        && !checker.project().is_used_elsewhere(name)
}
