use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::resolve::{ClassBase, ClassKey, ModuleFacts, class_home};
use crate::project::facts::ImportBinding;

// The hook interfaces hatchling loads a custom hook's class from, with the module and name each one is imported as.
const HOOK_INTERFACES: [(&str, &str); 2] = [
    ("hatchling.builders.hooks.plugin.interface", "BuildHookInterface"),
    ("hatchling.metadata.plugin.interface", "MetadataHookInterface"),
];

// The classes each file defines that subclass a hook interface, directly or through the classes of the run they inherit
// from. hatchling calls the class of a custom hook file that subclasses one, which no import reaches.
pub(super) fn hatch_hook_classes(
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
) -> FxHashMap<FileId, FxHashSet<String>> {
    let mut classes: FxHashMap<ClassKey, Vec<ClassBase>> = FxHashMap::default();
    for (id, module) in modules {
        classes.extend(module.classes.iter().map(|(name, bases, _)| ((*id, name.clone()), bases.clone())));
    }
    let home = class_home(|key| classes.contains_key(key), bindings, stars);
    let mut parents: FxHashMap<&ClassKey, Vec<ClassKey>> = FxHashMap::default();
    let mut outside: FxHashMap<&ClassKey, Vec<(&str, &str)>> = FxHashMap::default();
    for (key, bases) in &classes {
        for (file, module, name) in bases {
            match file.and_then(|file| home(file, name)) {
                Some(parent) => parents.entry(key).or_default().push(parent),
                None => outside.entry(key).or_default().push((module, name)),
            }
        }
    }
    let mut hooks: FxHashSet<&ClassKey> = FxHashSet::default();
    loop {
        let mut grew = false;
        for key in classes.keys() {
            let interface = outside
                .get(key)
                .is_some_and(|bases| bases.iter().any(|&(module, name)| is_hook_interface(module, name)));
            let from_parents = parents.get(key).is_some_and(|found| found.iter().any(|parent| hooks.contains(parent)));
            if (interface || from_parents) && hooks.insert(key) {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let mut found: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for key in hooks {
        found.entry(key.0).or_default().insert(key.1.clone());
    }
    found
}

fn is_hook_interface(module: &str, name: &str) -> bool {
    HOOK_INTERFACES
        .iter()
        .any(|&(interface_module, interface_name)| interface_module == module && interface_name == name)
}
