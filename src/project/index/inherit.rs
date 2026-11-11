use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::api::exposure;
use super::classes::{ClassGraph, Reexports};
use super::resolve::{ClassBase, ClassKey};
use crate::syntax::Visibility;

pub(super) fn inherited_api(
    library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    library_reached: &FxHashMap<FileId, FxHashSet<String>>,
    graph: &ClassGraph,
    reexports: &Reexports,
) -> FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> {
    let home = graph.home(reexports);
    let mut inherited: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> = FxHashMap::default();
    for (key, (bases, methods)) in &graph.classes {
        if exposure(library_api.get(&key.0), library_reached.get(&key.0), &key.1).is_none() {
            continue;
        }
        let overridden: FxHashSet<&str> = methods.iter().map(String::as_str).collect();
        let mut visited: FxHashSet<ClassKey> = FxHashSet::default();
        collect(&home, &graph.classes, bases, &overridden, &mut visited, &mut inherited);
    }
    inherited
}

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
