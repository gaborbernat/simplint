use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::builtins::is_exception;

use crate::project::facts::ImportBinding;
use crate::syntax::is_stdlib_root;

use super::FileId;
use super::classes::{ClassGraph, Reexports, acyclic_order};
use super::resolve::{ClassKey, ModuleFacts};

fn expand_tuples(caught: &mut Vec<ClassKey>, tuples: &FxHashMap<ClassKey, Vec<ClassKey>>) {
    let mut pending = std::mem::take(caught);
    let mut expanded: FxHashSet<ClassKey> = FxHashSet::default();
    while let Some(key) = pending.pop() {
        match tuples.get(&key) {
            Some(elements) => {
                if expanded.insert(key.clone()) {
                    pending.extend(elements.iter().cloned());
                }
            }
            None => caught.push(key),
        }
    }
}

// A warning is not reported, since a warning is raised to be seen, not caught.
pub(super) fn exception_classes(
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    python_version: PythonVersion,
    test_modules: &FxHashSet<FileId>,
    graph: &ClassGraph,
    reexports: &Reexports,
    bindings: &[(FileId, FileId, &ImportBinding)],
) -> (FxHashMap<FileId, FxHashSet<String>>, FxHashMap<FileId, FxHashSet<String>>) {
    let minor = python_version.minor;
    let (mut caught, mut tuples): (Vec<ClassKey>, FxHashMap<ClassKey, Vec<ClassKey>>) =
        (Vec::new(), FxHashMap::default());
    for (id, module) in modules {
        if !test_modules.contains(id) {
            caught.extend(module.caught.iter().map(|(file, name)| (*file, name.clone())));
        }
        for (name, elements) in &module.class_tuples {
            tuples.entry((*id, name.clone())).or_default().extend(elements.iter().cloned());
        }
    }
    expand_tuples(&mut caught, &tuples);
    let home = graph.home(reexports);
    let exceptions = graph.descending_from(|module, name| is_exception_name(module, name, minor));
    let warnings =
        graph.descending_from(|module, name| name.ends_with("Warning") && is_exception_name(module, name, minor));
    // A handler catches its class and every base of it, so each base of a caught class is covered too.
    let mut covered: FxHashSet<ClassKey> = FxHashSet::default();
    let mut pending: Vec<ClassKey> = caught.iter().filter_map(|(file, name)| home(*file, name)).collect();
    while let Some(key) = pending.pop() {
        if covered.insert(key.clone()) {
            pending.extend(graph.parents.get(&key).into_iter().flatten().cloned());
        }
    }
    let mut uncaught: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in graph.classes.keys() {
        let key = (*file, name.clone());
        if exceptions.contains(&key) && !warnings.contains(&key) && !covered.contains(&key) {
            uncaught.entry(*file).or_default().insert(name.clone());
        }
    }
    let mut types: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    let exception_names: FxHashSet<&str> = exceptions.iter().map(|key| key.1.as_str()).collect();
    let mut declared: FxHashMap<FileId, Vec<&str>> = FxHashMap::default();
    for (file, name) in graph.classes.keys() {
        if exception_names.contains(name.as_str()) {
            declared.entry(*file).or_default().push(name.as_str());
        }
    }
    for (importer, _, binding) in bindings {
        if exception_names.contains(binding.bound.as_str()) {
            declared.entry(*importer).or_default().push(binding.bound.as_str());
        }
    }
    let candidates = exceptions
        .iter()
        .map(|key| (key.0, key.1.as_str()))
        .chain(bindings.iter().map(|(importer, _, binding)| (*importer, binding.bound.as_str())))
        .chain(
            star_exception_names(&reexports.starred, &declared)
                .into_iter()
                .flat_map(|(importer, names)| names.into_iter().map(move |name| (importer, name))),
        );
    for (file, name) in candidates {
        if home(file, name).is_some_and(|key| exceptions.contains(&key)) {
            types.entry(file).or_default().insert(name.to_owned());
        }
    }
    (uncaught, types)
}

// A file that reaches a star-import cycle is walked, since a cycle leaves the file out of its own names.
fn star_exception_names<'a>(
    starred: &FxHashMap<FileId, Vec<FileId>>,
    declared: &FxHashMap<FileId, Vec<&'a str>>,
) -> FxHashMap<FileId, FxHashSet<&'a str>> {
    let edges = starred.iter().flat_map(|(importer, sources)| sources.iter().map(|source| (*importer, *source)));
    let mut memo: FxHashMap<FileId, FxHashSet<&'a str>> = FxHashMap::default();
    for file in acyclic_order(edges) {
        let mut names = FxHashSet::default();
        for source in starred.get(&file).into_iter().flatten() {
            names.extend(declared.get(source).into_iter().flatten().copied());
            names.extend(memo[source].iter().copied());
        }
        memo.insert(file, names);
    }
    starred
        .keys()
        .map(|&importer| {
            let names = memo.remove(&importer).unwrap_or_else(|| starred_exception_names(importer, starred, declared));
            (importer, names)
        })
        .collect()
}

fn starred_exception_names<'a>(
    file: FileId,
    starred: &FxHashMap<FileId, Vec<FileId>>,
    declared: &FxHashMap<FileId, Vec<&'a str>>,
) -> FxHashSet<&'a str> {
    let mut names = FxHashSet::default();
    let mut seen = FxHashSet::from_iter([file]);
    let mut pending: Vec<FileId> = starred.get(&file).into_iter().flatten().copied().collect();
    while let Some(source) = pending.pop() {
        if seen.insert(source) {
            names.extend(declared.get(&source).into_iter().flatten().copied());
            pending.extend(starred.get(&source).into_iter().flatten().copied());
        }
    }
    names
}

fn is_exception_name(module: &str, name: &str, minor: u8) -> bool {
    let standard = is_stdlib_root(minor, module);
    is_exception(name, minor)
        || standard && ["Error", "Exception", "Warning"].iter().any(|suffix| name.ends_with(suffix))
}
