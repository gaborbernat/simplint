use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::classes::ClassGraph;

// The hook interfaces hatchling loads a custom hook's class from, with the module and name each one is imported as.
const HOOK_INTERFACES: [(&str, &str); 2] = [
    ("hatchling.builders.hooks.plugin.interface", "BuildHookInterface"),
    ("hatchling.metadata.plugin.interface", "MetadataHookInterface"),
];

// The classes each file defines that subclass a hook interface, directly or through the classes of the run they inherit
// from. hatchling calls the class of a custom hook file that subclasses one, which no import reaches.
pub(super) fn hatch_hook_classes(graph: &ClassGraph) -> FxHashMap<FileId, FxHashSet<String>> {
    let mut found: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for key in graph.descending_from(is_hook_interface) {
        found.entry(key.0).or_default().insert(key.1.clone());
    }
    found
}

fn is_hook_interface(module: &str, name: &str) -> bool {
    HOOK_INTERFACES
        .iter()
        .any(|&(interface_module, interface_name)| interface_module == module && interface_name == name)
}
