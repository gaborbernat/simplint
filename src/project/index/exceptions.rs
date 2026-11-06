use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::builtins::is_exception;
use ruff_python_stdlib::sys::is_known_standard_library;

use crate::project::FileFacts;
use crate::project::facts::ImportBinding;

use super::FileId;
use super::resolve::{ClassBase, ClassKey, ModuleFacts};

// The exception classes of the run that no handler catches, by name or through a subclass, which is how a handler of a
// base class reads. A class is an exception when a base is an exception: a builtin one, a standard-library one, or an
// exception class of the run. A warning is not reported, since a warning is raised to be seen, not caught.
pub(super) fn uncaught_exceptions(
    files: &[(FileId, &FileFacts)],
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    python_version: PythonVersion,
    test_modules: &FxHashSet<FileId>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
) -> FxHashMap<FileId, FxHashSet<String>> {
    let minor = python_version.minor;
    let (mut caught, mut classes): (Vec<ClassKey>, FxHashMap<ClassKey, Vec<ClassBase>>) =
        (Vec::new(), FxHashMap::default());
    for (id, _) in files {
        let module = &modules[id];
        if !test_modules.contains(id) {
            caught.extend(module.caught.iter().map(|(file, name)| (*file, name.clone())));
        }
        classes.extend(module.classes.iter().map(|(name, bases)| ((*id, name.clone()), bases.clone())));
    }
    let mut reexports: FxHashMap<FileId, Vec<(&str, FileId, &str)>> = FxHashMap::default();
    for (importer, source, binding) in bindings {
        reexports.entry(*importer).or_default().push((&binding.bound, *source, &binding.name));
    }
    let mut starred: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in stars {
        starred.entry(*importer).or_default().push(*source);
    }
    // The class a name of a file is, following the imports that re-export it from the file that defines it.
    let home = |file: FileId, name: &str| -> Option<ClassKey> {
        let mut pending: Vec<(FileId, &str)> = vec![(file, name)];
        let mut seen: FxHashSet<(FileId, &str)> = FxHashSet::default();
        while let Some((file, name)) = pending.pop() {
            if !seen.insert((file, name)) {
                continue;
            }
            let key = (file, name.to_owned());
            if classes.contains_key(&key) {
                return Some(key);
            }
            for (bound, source, original) in reexports.get(&file).into_iter().flatten() {
                if *bound == name {
                    pending.push((*source, original));
                }
            }
            pending.extend(starred.get(&file).into_iter().flatten().map(|source| (*source, name)));
        }
        None
    };
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
    let (mut exceptions, mut warnings): (FxHashSet<&ClassKey>, FxHashSet<&ClassKey>) =
        (FxHashSet::default(), FxHashSet::default());
    loop {
        let mut grew = false;
        for key in classes.keys() {
            let bases = outside.get(key).map_or(&[][..], Vec::as_slice);
            let from_parents = |set: &FxHashSet<&ClassKey>| {
                parents.get(key).is_some_and(|found| found.iter().any(|parent| set.contains(parent)))
            };
            let raised = bases.iter().any(|(module, name)| is_exception_name(module, name, minor));
            let warns =
                bases.iter().any(|(module, name)| name.ends_with("Warning") && is_exception_name(module, name, minor));
            if (raised || from_parents(&exceptions)) && exceptions.insert(key) {
                grew = true;
            }
            if (warns || from_parents(&warnings)) && warnings.insert(key) {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
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
    uncaught
}

// Whether a base class a run does not define is an exception: a builtin one, or a standard-library one, which by its
// name ends in `Error`, `Exception` or `Warning`.
fn is_exception_name(module: &str, name: &str, minor: u8) -> bool {
    let standard = is_known_standard_library(minor, module.split('.').next().unwrap_or_default());
    is_exception(name, minor)
        || standard && ["Error", "Exception", "Warning"].iter().any(|suffix| name.ends_with(suffix))
}
