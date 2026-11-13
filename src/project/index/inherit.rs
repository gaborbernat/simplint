use std::path::PathBuf;

use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::api::exposure;
use super::classes::{ClassGraph, Reexports};
use super::resolve::{ClassBase, ClassKey};
use crate::syntax::Visibility;

// Users reach a private base's members through the public class that inherits them, as `Unix` reaches the methods of
// `XDGMixin`.
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
        if !visited.insert(key.clone()) {
            continue;
        }
        let (base_bases, methods) = classes[&key];
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

// The methods each class defines again after a class of the run up its bases defined them, by class and file.
pub(super) fn overridden_methods(
    graph: &ClassGraph,
    reexports: &Reexports,
) -> FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> {
    let home = graph.home(reexports);
    let mut overridden: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>> = FxHashMap::default();
    for (key, (bases, methods)) in &graph.classes {
        let base_key = |(file, _, name): &ClassBase| file.and_then(|file| home(file, name));
        let mut pending: Vec<ClassKey> = bases.iter().filter_map(base_key).collect();
        let mut visited: FxHashSet<ClassKey> = FxHashSet::default();
        let mut ancestors: FxHashSet<&str> = FxHashSet::default();
        while let Some(base) = pending.pop() {
            if !visited.insert(base.clone()) {
                continue;
            }
            let (base_bases, base_methods) = graph.classes[&base];
            ancestors.extend(base_methods.iter().map(String::as_str));
            pending.extend(base_bases.iter().filter_map(base_key));
        }
        let found: FxHashSet<String> =
            methods.iter().filter(|method| ancestors.contains(method.as_str())).cloned().collect();
        if !found.is_empty() {
            overridden.entry(key.0).or_default().insert(key.1.clone(), found);
        }
    }
    overridden
}
