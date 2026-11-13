use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;

use ruff_graph::ResolverEnvironment;

use crate::project::FileFacts;
use crate::syntax::Visibility;

use super::files::{FileId, FileIds};
use super::resolve::{ImportResolver, resolve};

type FileNames = Vec<(FileId, String)>;

// A bare module entry point is loaded whole and read by convention, so every public name it defines or imports is in
// use, such as the hooks and fixtures of a pytest plugin.
pub(super) fn interned_entry_names(
    resolver: &mut ImportResolver,
    entry_points: &[(PathBuf, String)],
    build_hooks: &[(PathBuf, String)],
    indexed: &[(FileId, &FileFacts)],
) -> (FileNames, FileNames) {
    let facts: FxHashMap<FileId, &FileFacts> = indexed.iter().copied().collect();
    let hooks: FileNames =
        build_hooks.iter().map(|(file, name)| (FileIds::intern(&mut resolver.files, file), name.clone())).collect();
    let mut names = Vec::new();
    let mut bare = Vec::new();
    for (file, attribute) in entry_point_targets(resolver, entry_points) {
        let id = FileIds::intern(&mut resolver.files, &file);
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
                    (module, Some(attribute.split_once('.').map_or(attribute, |(first, _)| first).trim().to_owned()))
                }
                None => (entry_point.as_str(), None),
            };
            let paths = resolver.entry_paths.get(root)?;
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            Some((resolve(&resolver.db, environment, None, module.trim())?, attribute))
        })
        .collect()
}

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

// Coverage calls `coverage_init` on a module it loads by dotted name, so no import reaches the function.
pub(super) fn coverage_plugin_names(
    resolver: &mut ImportResolver,
    plugins: &[(PathBuf, String)],
) -> Vec<(FileId, String)> {
    let files: Vec<PathBuf> = plugins
        .iter()
        .filter_map(|(root, module)| {
            // A project's search paths include its own directory, so each project with plugins has an entry.
            let paths = &resolver.coverage_paths[root];
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            resolve(&resolver.db, environment, None, module)
        })
        .collect();
    files.iter().map(|file| (FileIds::intern(&mut resolver.files, file), "coverage_init".to_owned())).collect()
}

pub(super) fn startup_entries(
    resolver: &mut ImportResolver,
    startup: &[String],
) -> (FxHashSet<FileId>, Vec<(FileId, String)>) {
    let mut loaded = FxHashSet::default();
    let mut reads = Vec::new();
    for (file, attribute) in startup_targets(resolver, startup) {
        let id = FileIds::intern(&mut resolver.files, &file);
        loaded.insert(id);
        reads.extend(attribute.map(|attribute| (id, attribute)));
    }
    (loaded, reads)
}

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
