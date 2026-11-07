use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::api::exposure;
use super::resolve::{ClassBase, ClassKey, ModuleFacts, class_home};
use crate::project::facts::ImportBinding;
use crate::syntax::Visibility;

// The members a private class defines that a public class of a library inherits without overriding them, which users
// reach through the public class, as `Unix` reaches the methods of its private base `XDGMixin`. Each file maps its
// classes to their inherited members.
pub(super) fn inherited_api(
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    library_reached: &FxHashMap<FileId, FxHashSet<String>>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
) -> FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> {
    let mut classes: FxHashMap<ClassKey, (&[ClassBase], &[String])> = FxHashMap::default();
    for (id, module) in modules {
        for (name, bases, methods) in &module.classes {
            classes.insert((*id, name.clone()), (bases.as_slice(), methods.as_slice()));
        }
    }
    let home = class_home(|key| classes.contains_key(key), bindings, stars);
    let mut inherited: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> = FxHashMap::default();
    for (key, (bases, methods)) in &classes {
        if exposure(library_api.get(&key.0), library_reached.get(&key.0), &key.1).is_none() {
            continue;
        }
        let overridden: FxHashSet<&str> = methods.iter().map(String::as_str).collect();
        let mut visited: FxHashSet<ClassKey> = FxHashSet::default();
        collect(&home, &classes, bases, &overridden, &mut visited, &mut inherited);
    }
    inherited
}

// Records the public members of each base class `bases` lists, and of the bases those inherit, that no class on the way
// from the public class overrides. A base the run does not define has no members to record.
fn collect<'a>(
    home: &impl Fn(FileId, &str) -> Option<ClassKey>,
    classes: &FxHashMap<ClassKey, (&'a [ClassBase], &'a [String])>,
    bases: &[ClassBase],
    overridden: &FxHashSet<&str>,
    visited: &mut FxHashSet<ClassKey>,
    inherited: &mut FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>>,
) {
    for (file, _, name) in bases {
        let Some(key) = file.and_then(|file| home(file, name)) else {
            continue;
        };
        // `home` answers only for classes of the run.
        let (base_bases, methods) = classes[&key];
        if !visited.insert(key.clone()) {
            continue;
        }
        for method in methods.iter().filter(|method| Visibility::of(method) == Visibility::Public) {
            if !overridden.contains(method.as_str()) {
                inherited.entry(key.0).or_default().entry(key.1.clone()).or_default().insert(method.clone());
            }
        }
        let mut deeper = overridden.clone();
        deeper.extend(methods.iter().map(String::as_str));
        collect(home, classes, base_bases, &deeper, visited, inherited);
    }
}
