use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::classes::ClassGraph;

// hatchling calls the hook class a custom hook file defines, which no import reaches.
pub(super) fn hatch_hook_classes(graph: &ClassGraph) -> FxHashMap<FileId, FxHashSet<String>> {
    let mut found: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for key in graph.descending_from(is_hook_interface) {
        found.entry(key.0).or_default().insert(key.1.clone());
    }
    found
}

fn is_hook_interface(module: &str, name: &str) -> bool {
    matches!(
        (module, name),
        ("hatchling.builders.hooks.plugin.interface", "BuildHookInterface")
            | ("hatchling.metadata.plugin.interface", "MetadataHookInterface")
    )
}
