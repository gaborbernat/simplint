use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;

use ruff_graph::ResolverEnvironment;

use crate::project::FileFacts;
use crate::syntax::Visibility;

use super::files::FileId;
use super::resolve::{ImportResolver, resolve};

type FileNames = Vec<(FileId, String)>;

// The entry points' and build hooks' names, with each file interned, so the index keys them by file id, and the names
// the bare module entry points load among them. A bare module entry point, which the host loads whole and reads by
// convention, names every public name it defines or imports, such as the hooks and fixtures of a pytest plugin, so none
// of them is internal.
pub(super) fn interned_entry_names(
    resolver: &mut ImportResolver,
    entry_points: &[(PathBuf, String)],
    build_hooks: &[(PathBuf, String)],
    indexed: &[(FileId, &FileFacts)],
) -> (FileNames, FileNames) {
    let facts: FxHashMap<FileId, &FileFacts> = indexed.iter().copied().collect();
    let hooks: FileNames = build_hooks.iter().map(|(file, name)| (resolver.intern(file), name.clone())).collect();
    let mut names = Vec::new();
    let mut bare = Vec::new();
    for (file, attribute) in entry_point_targets(resolver, entry_points) {
        let id = resolver.intern(&file);
        match attribute {
            Some(attribute) => names.push((id, attribute)),
            None => bare.extend(
                facts.get(&id).map_or_else(Vec::new, |facts| public_names(facts)).into_iter().map(|name| (id, name)),
            ),
        }
    }
    names.extend(bare.iter().cloned());
    names.extend(hooks);
    (names, bare)
}

// The modules the interpreter loads at startup through the `.pth` names, which no import reaches, with each attribute
// one of those names reads from its module. The longest prefix of a name that is a module is the one loaded, and the
// part after it is the attribute read.
pub(super) fn startup_entries(
    resolver: &mut ImportResolver,
    startup: &[String],
) -> (FxHashSet<FileId>, Vec<(FileId, String)>) {
    let mut loaded = FxHashSet::default();
    let mut reads = Vec::new();
    for (file, attribute) in startup_targets(resolver, startup) {
        let id = resolver.intern(&file);
        loaded.insert(id);
        reads.extend(attribute.map(|attribute| (id, attribute)));
    }
    (loaded, reads)
}

// The file each `.pth` name resolves to, with the attribute it reads from that file, if any.
fn startup_targets(resolver: &ImportResolver, startup: &[String]) -> Vec<(PathBuf, Option<String>)> {
    let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, resolver.paths.clone());
    startup
        .iter()
        .filter_map(|name| {
            let parts: Vec<&str> = name.split('.').collect();
            (1..=parts.len()).rev().find_map(|length| {
                let file = resolve(&resolver.db, environment, None, &parts[..length].join("."))?;
                Some((file, parts.get(length).map(|attribute| (*attribute).to_owned())))
            })
        })
        .collect()
}

// The `coverage_init` of each coverage plugin module that resolves to a file of the run. Coverage loads the module by
// its dotted name and calls `coverage_init` on it, so no import reaches the function, and the name is read as a string.
pub(super) fn coverage_plugin_names(
    resolver: &mut ImportResolver,
    plugins: &[(PathBuf, String)],
) -> Vec<(FileId, String)> {
    let files: Vec<PathBuf> = plugins
        .iter()
        .filter_map(|(root, module)| {
            let paths = resolver.coverage_paths.get(root)?;
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            resolve(&resolver.db, environment, None, module)
        })
        .collect();
    files.iter().map(|file| (resolver.intern(file), "coverage_init".to_owned())).collect()
}

// The file each entry point of the project names, with the top-level attribute of a `module:attribute` entry point.
// Each resolves with its project's directory on the search path, since a flat layout's modules sit there.
fn entry_point_targets(
    resolver: &ImportResolver,
    entry_points: &[(PathBuf, String)],
) -> Vec<(PathBuf, Option<String>)> {
    entry_points
        .iter()
        .filter_map(|(root, entry_point)| {
            let (module, attribute) = match entry_point.split_once(':') {
                Some((module, attribute)) => {
                    (module, Some(attribute.split('.').next().unwrap_or_default().trim().to_owned()))
                }
                None => (entry_point.as_str(), None),
            };
            let paths = resolver.entry_paths.get(root)?;
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            Some((resolve(&resolver.db, environment, None, module.trim())?, attribute))
        })
        .collect()
}

// The names a module defines or imports at its top level that are not private, which its users may reach.
fn public_names(facts: &FileFacts) -> Vec<String> {
    facts
        .defined
        .iter()
        .map(String::as_str)
        .chain(facts.bindings.iter().map(|binding| binding.bound.as_str()))
        .filter(|name| Visibility::of(name) == Visibility::Public)
        .map(str::to_owned)
        .collect()
}
