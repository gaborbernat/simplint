use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

use crate::project::FileFacts;
use crate::project::facts::ImportBinding;
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::resolve::Unreached;
use super::{FileId, FileIds};

// The files `import *` reaches: those it reads every public name of, those a package's `__init__.py` star-imports, and
// every file a star import names.
pub(super) struct StarTargets {
    pub(super) star_imported: FxHashSet<FileId>,
    pub(super) package_star_imported: FxHashSet<FileId>,
    pub(super) star_sources: FxHashSet<FileId>,
}

impl StarTargets {
    pub(super) fn of(
        stars: &[(FileId, FileId)],
        sources: &FxHashMap<FileId, &FileFacts>,
        unresolved: &FxHashSet<&str>,
        files: &FileIds,
    ) -> Self {
        let star_imported = stars
            .iter()
            .map(|(_, source)| *source)
            .chain(unresolved_candidates(files, sources.keys().copied(), unresolved))
            .filter(|file| sources.get(file).is_some_and(|facts| facts.exports.is_none()))
            .collect();
        let package_star_imported = stars
            .iter()
            .filter(|(importer, source)| {
                files.path(*source).parent().is_some_and(|parent| files.path(*importer) == parent.join("__init__.py"))
            })
            .map(|(_, source)| *source)
            .collect();
        let star_sources = stars.iter().map(|(_, source)| *source).collect();
        Self { star_imported, package_star_imported, star_sources }
    }
}

// The classes and aliases each file defines that another exported name's signature reaches, keyed by that file.
pub(super) fn api_reached(indexed: &[(FileId, &FileFacts)]) -> FxHashMap<FileId, FxHashSet<String>> {
    indexed
        .iter()
        .filter(|(_, facts)| !facts.api_reached.is_empty())
        .map(|(id, facts)| (*id, facts.api_reached.iter().cloned().collect()))
        .collect()
}

pub(super) fn unqualified(class: &str) -> &str {
    class.rsplit('.').next().unwrap_or(class)
}

// The names a library's users reach. A public module offers what its `__all__` lists, or, without one, every name it
// defines or re-exports without a leading underscore, as the typing spec reads a module; a name another module imports
// from it counts too, since `missing-from-all` lists it there. A module that re-exports a public name, through
// `__all__`, a redundant alias or `import *`, makes it public where it is defined, through any chain of re-exports.
// Names only spread, so this settles.
pub(super) fn user_facing(
    sources: &FxHashMap<FileId, &FileFacts>,
    roles: &FxHashMap<FileId, ModuleRole>,
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
    files: &FileIds,
) -> FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>> {
    // What `import *` of a module takes: its `__all__`, or every name it defines without a leading underscore.
    let starred = |name: &&str| Visibility::of(name) == Visibility::Public;
    let star_exports: FxHashMap<FileId, Vec<&str>> = stars
        .iter()
        .map(|(_, source)| {
            let exported = sources.get(source).map_or_else(Vec::new, |facts| match &facts.exports {
                Some(listed) => listed.iter().map(String::as_str).collect(),
                None => facts.defined.iter().map(String::as_str).filter(starred).collect(),
            });
            (*source, exported)
        })
        .collect();
    let mut names: FxHashMap<FileId, FxHashMap<String, Option<FileId>>> = FxHashMap::default();
    for (path, facts) in sources {
        if !roles.get(path).is_some_and(|role| role.public) {
            continue;
        }
        let own = names.entry(*path).or_default();
        let mut add = |name: &str| {
            own.entry(name.to_owned()).or_insert(None);
        };
        match &facts.exports {
            Some(listed) => listed.iter().for_each(|name| add(name)),
            None if facts.exports_unknown => {}
            None => {
                let redundant = facts.bindings.iter().filter(|binding| binding.redundant).map(|binding| &binding.bound);
                facts
                    .defined
                    .iter()
                    .map(String::as_str)
                    .chain(redundant.map(String::as_str))
                    .filter(starred)
                    .for_each(&mut add);
                for (_, source) in stars.iter().filter(|(importer, _)| importer == path) {
                    star_exports[source].iter().copied().for_each(&mut add);
                }
            }
        }
        imported.get(path).into_iter().flatten().map(String::as_str).filter(starred).for_each(&mut add);
    }
    loop {
        let mut reached: Vec<(FileId, String, FileId)> = Vec::new();
        for (importer, source, binding) in bindings {
            if names.get(importer).is_some_and(|known| known.contains_key(&binding.bound)) {
                reached.push((*source, binding.name.clone(), *importer));
            }
        }
        for (importer, source) in stars {
            let Some(known) = names.get(importer) else {
                continue;
            };
            reached.extend(
                star_exports[source]
                    .iter()
                    .filter(|name| known.contains_key(**name))
                    .map(|name| (*source, (*name).to_owned(), *importer)),
            );
        }
        let mut spread = false;
        for (source, name, importer) in reached {
            if let std::collections::hash_map::Entry::Vacant(entry) = names.entry(source).or_default().entry(name) {
                entry.insert(Some(importer));
                spread = true;
            }
        }
        if !spread {
            return names
                .into_iter()
                .map(|(file, names)| {
                    let names = names
                        .into_iter()
                        .map(|(name, importer)| (name, importer.map(|importer| files.path(importer).to_path_buf())))
                        .collect();
                    (file, names)
                })
                .collect();
        }
    }
}

// A name another module takes from a module that star-imports it, as `from pkg import Name` where `pkg/__init__.py` has
// `from ._types import *`, comes from the star-imported module, so it counts there too, through any chain of star
// re-exports, unless that module's `__all__` leaves it out of the `import *`. Names only spread, so this settles.
pub(super) fn reexport(
    names: &mut FxHashMap<FileId, FxHashSet<String>>,
    stars: &[(FileId, FileId)],
    sources: &FxHashMap<FileId, &FileFacts>,
) {
    loop {
        let mut spread = false;
        for (importer, source) in stars {
            let listed = sources.get(source).and_then(|facts| facts.exports.as_deref());
            let reached: Vec<String> = names
                .get(importer)
                .into_iter()
                .flatten()
                .filter(|name| listed.is_none_or(|listed| listed.contains(name)))
                .filter(|name| !names.get(source).is_some_and(|known| known.contains(*name)))
                .cloned()
                .collect();
            spread |= !reached.is_empty();
            names.entry(*source).or_default().extend(reached);
        }
        if !spread {
            return;
        }
    }
}

// An `import *` of a module that does not resolve, as one found on a `sys.path` entry a `conftest.py` adds, may read any
// file of the run Python could import under that module's last name: `name.py` or `name/__init__.py`. A module no file
// of the run is named after is outside the run, so it reads none of them.
fn unresolved_candidates(
    paths: &FileIds,
    files: impl Iterator<Item = FileId>,
    unresolved: &FxHashSet<&str>,
) -> Vec<FileId> {
    files.filter(|file| last_name(paths.path(*file)).is_some_and(|name| unresolved.contains(name))).collect()
}

// A dotted path through a package that names no module, as `nx.reportviews.Name` where `networkx` re-exports its
// `networkx.classes.reportviews` submodule, may reach any module of that package with the path's last name. Those
// names count as referenced rather than imported: they are kept, yet not made part of the module's API.
pub(super) fn reach_by_last_name(
    referenced: &mut FxHashMap<FileId, FxHashSet<String>>,
    unreached: &[Unreached],
    paths: &FileIds,
    files: impl Iterator<Item = FileId>,
) {
    if unreached.is_empty() {
        return;
    }
    let mut by_name: FxHashMap<&str, Vec<FileId>> = FxHashMap::default();
    for file in files {
        if let Some(name) = last_name(paths.path(file)) {
            by_name.entry(name).or_default().push(file);
        }
    }
    for (package, name, names) in unreached {
        for file in by_name.get(name).into_iter().flatten().filter(|file| paths.path(**file).starts_with(package)) {
            referenced.entry(*file).or_default().extend(names.iter().cloned());
        }
    }
}

// The last name of the module a file defines: `name` for `name.py` and `name/__init__.py`.
fn last_name(file: &Path) -> Option<&str> {
    let stem = file.file_stem().and_then(|stem| stem.to_str());
    if stem == Some("__init__") { file.parent().and_then(Path::file_name).and_then(|name| name.to_str()) } else { stem }
}
