use crate::project::docs::Module;
use crate::project::receivers::module_parts;
use crate::project::{DocReferences, FileFacts};

use super::files::FileId;
use super::resolve::ImportResolver;

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
                // A rendered class documents its members too.
                for class in facts.classes.iter().filter(|class| members.contains(&class.name.as_str())) {
                    expanded.add_scope(format!("{dotted}.{}.", class.name));
                }
                expanded.add_names(members.into_iter().map(|name| format!("{dotted}.{name}")));
            }
        }
    }
    expanded
}

// Sphinx renders underscore and dunder names only when the directive's flags take them in.
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
