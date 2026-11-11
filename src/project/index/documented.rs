use crate::project::docs::Module;
use crate::project::receivers::module_parts;
use crate::project::{DocReferences, FileFacts};

use super::files::FileId;
use super::resolve::ImportResolver;

// An entry point runs without any import, so its name counts as used by code outside the tests.
// The names the members of each module `automodule` documents with `:members:` add to the references, as Sphinx renders
// them: the module's `__all__` if it has one, else its public names. The directive's `:exclude-members:` leaves names out,
// and its `:private-members:` and `:special-members:` take in the underscore and dunder names.
pub(super) fn documented_members(
    docs: &DocReferences,
    indexed: &[(FileId, &FileFacts)],
    resolver: &ImportResolver,
) -> DocReferences {
    let mut expanded = docs.clone();
    let named: Vec<(&FileFacts, Vec<String>)> = indexed
        .iter()
        .map(|(file, facts)| {
            let dotted = resolver
                .search
                .module_paths(resolver.files.path(*file))
                .iter()
                .map(|parts| module_parts(parts).join("."))
                .collect();
            (*facts, dotted)
        })
        .collect();
    for module in docs.modules() {
        for (facts, dotted_names) in &named {
            for dotted in dotted_names.iter().filter(|dotted| {
                dotted.as_str() == module.target
                    || dotted.strip_suffix(module.target.as_str()).is_some_and(|rest| rest.ends_with('.'))
            }) {
                let members: Vec<&str> = match &facts.exports {
                    Some(exports) => exports.iter().map(String::as_str).collect(),
                    None => facts.defined.iter().map(String::as_str).filter(|name| is_rendered(name, module)).collect(),
                };
                let members: Vec<&str> = members
                    .into_iter()
                    .filter(|name| !module.exclude.iter().any(|excluded| excluded == name))
                    .collect();
                // A class the directive renders documents its members too, as the class's own name does.
                for class in facts.classes.iter().filter(|class| members.contains(&class.name.as_str())) {
                    expanded.add_scope(format!("{dotted}.{}.", class.name));
                }
                expanded.add_names(members.into_iter().map(|name| format!("{dotted}.{name}")));
            }
        }
    }
    expanded
}

// Whether Sphinx renders a module's name with `:members:`: its public names, and with the directive's flags its
// underscore names and its special names.
fn is_rendered(name: &str, module: &Module) -> bool {
    let special = name.len() > 4 && name.starts_with("__") && name.ends_with("__");
    if special {
        module.special
    } else if name.starts_with('_') {
        module.private
    } else {
        true
    }
}
