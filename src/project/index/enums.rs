use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::classes::{ClassGraph, Reexports};
use crate::project::facts::ImportBinding;

// The enum classes a file names, its own or imported from another module of the run, so an annotation or default that
// names one is a flag rather than a collaborator.
pub(super) fn enum_classes(
    graph: &ClassGraph,
    reexports: &Reexports,
    bindings: &[(FileId, FileId, &ImportBinding)],
) -> FxHashMap<FileId, FxHashSet<String>> {
    let enums = graph.descending_from(is_enum_base);
    let home = graph.home(reexports);
    let defined = graph.classes.keys().map(|(file, name)| (*file, name.as_str()));
    let imported = bindings.iter().map(|(importer, _, binding)| (*importer, binding.bound.as_str()));
    let mut found: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in defined.chain(imported) {
        if home(file, name).is_some_and(|key| enums.contains(&key)) {
            found.entry(file).or_default().insert(name.to_owned());
        }
    }
    found
}

fn is_enum_base(module: &str, name: &str) -> bool {
    module == "enum" && matches!(name, "Enum" | "IntEnum" | "StrEnum" | "Flag" | "IntFlag")
}
