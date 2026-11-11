use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::builtins::is_exception;

use crate::project::FileFacts;
use crate::project::facts::ImportBinding;
use crate::syntax::is_stdlib_root;

use super::FileId;
use super::resolve::{ClassBase, ClassKey, ModuleFacts, acyclic_order, class_home};

// A handler that names a tuple of classes catches each class in it, and a tuple may name other tuples.
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

// The exception classes of the run that no handler catches, by name or through a subclass, which is how a handler of a
// base class reads, and the names each file gives to an exception class, defined or imported. A class is an exception
// when a base is an exception: a builtin one, a standard-library one, or an exception class of the run. A warning is
// not reported, since a warning is raised to be seen, not caught.
pub(super) fn exception_classes(
    files: &[(FileId, &FileFacts)],
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    python_version: PythonVersion,
    test_modules: &FxHashSet<FileId>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
) -> (FxHashMap<FileId, FxHashSet<String>>, FxHashMap<FileId, FxHashSet<String>>) {
    let minor = python_version.minor;
    let (mut caught, mut classes): (Vec<ClassKey>, FxHashMap<ClassKey, Vec<ClassBase>>) =
        (Vec::new(), FxHashMap::default());
    let mut tuples: FxHashMap<ClassKey, Vec<ClassKey>> = FxHashMap::default();
    for (id, _) in files {
        let module = &modules[id];
        if !test_modules.contains(id) {
            caught.extend(module.caught.iter().map(|(file, name)| (*file, name.clone())));
        }
        for (name, elements) in &module.class_tuples {
            tuples.entry((*id, name.clone())).or_default().extend(elements.iter().cloned());
        }
        classes.extend(module.classes.iter().map(|(name, bases, _)| ((*id, name.clone()), bases.clone())));
    }
    expand_tuples(&mut caught, &tuples);
    let home = class_home(|key| classes.contains_key(key), bindings, stars);
    let mut parents: FxHashMap<&ClassKey, Vec<ClassKey>> = FxHashMap::default();
    let mut children: FxHashMap<ClassKey, Vec<&ClassKey>> = FxHashMap::default();
    let mut outside: FxHashMap<&ClassKey, Vec<(&str, &str)>> = FxHashMap::default();
    for (key, bases) in &classes {
        for (file, module, name) in bases {
            match file.and_then(|file| home(file, name)) {
                Some(parent) => {
                    children.entry(parent.clone()).or_default().push(key);
                    parents.entry(key).or_default().push(parent);
                }
                None => outside.entry(key).or_default().push((module, name)),
            }
        }
    }
    let (mut raised, mut warned): (Vec<&ClassKey>, Vec<&ClassKey>) = (Vec::new(), Vec::new());
    for key in classes.keys() {
        let bases = outside.get(key).map_or(&[][..], Vec::as_slice);
        if bases.iter().any(|(module, name)| is_exception_name(module, name, minor)) {
            raised.push(key);
        }
        if bases.iter().any(|(module, name)| name.ends_with("Warning") && is_exception_name(module, name, minor)) {
            warned.push(key);
        }
    }
    let exceptions = with_subclasses(raised, &children);
    let warnings = with_subclasses(warned, &children);
    // A handler catches its class and every base of it, so each base of a caught class is covered too.
    let mut covered: FxHashSet<ClassKey> = FxHashSet::default();
    let mut pending: Vec<ClassKey> = caught.iter().filter_map(|(file, name)| home(*file, name)).collect();
    while let Some(key) = pending.pop() {
        if covered.insert(key.clone()) {
            pending.extend(parents.get(&key).into_iter().flatten().cloned());
        }
    }
    let mut uncaught: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in classes.keys() {
        let key = (*file, name.clone());
        if exceptions.contains(&key) && !warnings.contains(&key) && !covered.contains(&key) {
            uncaught.entry(*file).or_default().insert(name.clone());
        }
    }
    let mut types: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    let exception_names: FxHashSet<&str> = exceptions.iter().map(|key| key.1.as_str()).collect();
    let mut declared: FxHashMap<FileId, Vec<&str>> = FxHashMap::default();
    for (file, name) in classes.keys() {
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
            star_exception_names(stars, &declared)
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

// A class that derives from one of the roots, or is one, through the classes that derive from each class.
fn with_subclasses<'a>(
    roots: Vec<&'a ClassKey>,
    children: &FxHashMap<ClassKey, Vec<&'a ClassKey>>,
) -> FxHashSet<&'a ClassKey> {
    let mut found = FxHashSet::default();
    let mut pending = roots;
    while let Some(key) = pending.pop() {
        if found.insert(key) {
            pending.extend(children.get(key).into_iter().flatten().copied());
        }
    }
    found
}

// The exception names each file of a star import brings in, by the file that star-imports it. A file that reaches no
// star-import cycle takes the names of the files it star-imports, once; a file that reaches one is walked, since a cycle
// leaves the file out of its own names.
fn star_exception_names<'a>(
    stars: &[(FileId, FileId)],
    declared: &FxHashMap<FileId, Vec<&'a str>>,
) -> FxHashMap<FileId, FxHashSet<&'a str>> {
    let mut starred: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in stars {
        starred.entry(*importer).or_default().push(*source);
    }
    let mut memo: FxHashMap<FileId, FxHashSet<&'a str>> = FxHashMap::default();
    for file in acyclic_order(stars.iter().copied()) {
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
            let names = memo.remove(&importer).unwrap_or_else(|| starred_exception_names(importer, &starred, declared));
            (importer, names)
        })
        .collect()
}

// The exception names a file's star imports bring in: those each star-imported file defines or binds, and those of the
// files it star-imports in turn.
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

// Whether a base class a run does not define is an exception: a builtin one, or a standard-library one, which by its
// name ends in `Error`, `Exception` or `Warning`.
fn is_exception_name(module: &str, name: &str, minor: u8) -> bool {
    let standard = is_stdlib_root(minor, module);
    is_exception(name, minor)
        || standard && ["Error", "Exception", "Warning"].iter().any(|suffix| name.ends_with(suffix))
}
