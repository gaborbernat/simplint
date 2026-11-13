use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

use crate::project::FileFacts;
use crate::project::facts::{Closure, ExportsUnknown, ImportBinding};
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::resolve::UnreachedPath;
use super::{FileId, FileIds};

pub(super) struct StarTargets {
    pub(super) star_imported: FxHashSet<FileId>,
    pub(super) package_star_imported: FxHashSet<FileId>,
    pub(super) star_sources: FxHashSet<FileId>,
    pub(super) package_bound: FxHashSet<FileId>,
}

impl StarTargets {
    pub(super) fn of(
        stars: &[(FileId, FileId)],
        sources: &FxHashMap<FileId, &FileFacts>,
        unresolved: &FxHashSet<&str>,
        files: &FileIds,
        production: &FxHashSet<FileId>,
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
        let package_bound = stars
            .iter()
            .filter(|(importer, _)| production.contains(importer))
            .map(|(_, source)| *source)
            .flat_map(|package| {
                let path = files.path(package);
                let listed = sources.get(&package).and_then(|facts| facts.exports.as_deref());
                let directory = path.parent().filter(|_| path.ends_with("__init__.py"));
                directory.zip(listed).into_iter().flat_map(|(directory, listed)| {
                    listed.iter().flat_map(move |name| {
                        [directory.join(format!("{name}.py")), directory.join(name).join("__init__.py")]
                    })
                })
            })
            .filter_map(|path| files.get(&path))
            .collect();
        Self { star_imported, package_star_imported, star_sources, package_bound }
    }
}

// One pass per file, so both root sets share its references.
pub(super) fn signature_reached(
    indexed: &[(FileId, &FileFacts)],
    library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
) -> (FxHashMap<FileId, FxHashSet<String>>, FxHashMap<FileId, FxHashSet<String>>) {
    let mut listed = FxHashMap::default();
    let mut exposed = FxHashMap::default();
    for (id, facts) in indexed {
        let closure = Closure::new(&facts.interface);
        let from_all = closure.reached(facts.exports.iter().flatten().map(String::as_str));
        if !from_all.is_empty() {
            listed.insert(*id, from_all.into_iter().collect());
        }
        let from_exposed =
            closure.reached(library_api.get(id).into_iter().flat_map(|names| names.keys().map(String::as_str)));
        if !from_exposed.is_empty() {
            exposed.insert(*id, from_exposed.into_iter().collect());
        }
    }
    (listed, exposed)
}

// A module whose `__getattr__` supplies names lazily offers only what it defines, since the names it re-exports are
// not all known. A public name an entry point or build hook loads is reached whatever `__all__` says, since the tool
// that loads it is outside the run.
pub(super) fn library_api(
    sources: &FxHashMap<FileId, &FileFacts>,
    roles: &FxHashMap<FileId, ModuleRole>,
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    entries: &[(FileId, String)],
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
    files: &FileIds,
) -> FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>> {
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
    let by_importer = starred_by_importer(stars);
    let mut loaded: FxHashMap<FileId, FxHashSet<&str>> = FxHashMap::default();
    for (file, name) in entries {
        loaded.entry(*file).or_default().insert(name.as_str());
    }
    let mut names: FxHashMap<FileId, FxHashMap<String, Option<FileId>>> = FxHashMap::default();
    for (path, facts) in sources {
        if !roles.get(path).is_some_and(|role| role.public) {
            continue;
        }
        let own = names.entry(*path).or_default();
        let mut add = |name: &str| {
            own.entry(name.to_owned()).or_insert(None);
        };
        loaded.get(path).into_iter().flatten().copied().filter(starred).for_each(&mut add);
        if let Some(listed) = &facts.exports {
            for name in listed {
                add(name);
            }
        } else {
            let unknown = facts.exports_unknown;
            if unknown != ExportsUnknown::Dynamic {
                facts.defined.iter().map(String::as_str).filter(starred).for_each(&mut add);
            }
            if unknown == ExportsUnknown::No {
                facts
                    .bindings
                    .iter()
                    .filter(|binding| binding.redundant)
                    .map(|binding| binding.bound.as_str())
                    .filter(starred)
                    .for_each(&mut add);
                for source in by_importer.get(path).into_iter().flatten() {
                    star_exports[source].iter().copied().for_each(&mut add);
                }
            }
            if unknown != ExportsUnknown::Lazy {
                imported.get(path).into_iter().flatten().map(String::as_str).filter(starred).for_each(&mut add);
            }
        }
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

pub(super) fn starred_by_importer(stars: &[(FileId, FileId)]) -> FxHashMap<FileId, Vec<FileId>> {
    let mut by_importer: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in stars {
        by_importer.entry(*importer).or_default().push(*source);
    }
    by_importer
}

// Names only spread, so this settles.
pub(super) fn reexport(
    names: &mut FxHashMap<FileId, FxHashSet<String>>,
    stars: &[(FileId, FileId)],
    sources: &FxHashMap<FileId, &FileFacts>,
) {
    let listed: FxHashMap<FileId, FxHashSet<&str>> = stars
        .iter()
        .filter_map(|(_, source)| {
            let exports = sources.get(source)?.exports.as_ref()?;
            Some((*source, exports.iter().map(String::as_str).collect()))
        })
        .collect();
    let by_importer = starred_by_importer(stars);
    let mut pending: Vec<FileId> = by_importer.keys().copied().collect();
    while let Some(importer) = pending.pop() {
        for source in by_importer.get(&importer).into_iter().flatten() {
            let reached: Vec<String> = names
                .get(&importer)
                .into_iter()
                .flatten()
                .filter(|name| listed.get(source).is_none_or(|listed| listed.contains(name.as_str())))
                .filter(|name| !names.get(source).is_some_and(|known| known.contains(*name)))
                .cloned()
                .collect();
            if !reached.is_empty() {
                names.entry(*source).or_default().extend(reached);
                pending.push(*source);
            }
        }
    }
}

// A `conftest.py` may add a `sys.path` entry the resolver does not see, so an unresolved `import *` may read any file
// under that module's last name.
fn unresolved_candidates(
    paths: &FileIds,
    files: impl Iterator<Item = FileId>,
    unresolved: &FxHashSet<&str>,
) -> Vec<FileId> {
    files.filter(|file| last_name(paths.path(*file)).is_some_and(|name| unresolved.contains(name))).collect()
}

pub(super) struct LastNames<'p> {
    by_name: FxHashMap<&'p str, Vec<FileId>>,
    paths: &'p FileIds,
}

impl<'p> LastNames<'p> {
    pub(super) fn new(paths: &'p FileIds, files: impl Iterator<Item = FileId>) -> Self {
        let mut by_name: FxHashMap<&str, Vec<FileId>> = FxHashMap::default();
        for file in files {
            if let Some(name) = last_name(paths.path(file)) {
                by_name.entry(name).or_default().push(file);
            }
        }
        Self { by_name, paths }
    }

    // A dotted path through a package may reach any module of that package with the path's last name.
    pub(super) fn reached<'s>(
        &'s self,
        unreached: impl IntoIterator<Item = &'s UnreachedPath> + 's,
    ) -> impl Iterator<Item = (FileId, &'s [String])> + 's {
        unreached.into_iter().flat_map(move |path| {
            self.by_name
                .get(path.last.as_str())
                .into_iter()
                .flatten()
                .filter(move |file| self.paths.path(**file).starts_with(&path.package))
                .map(move |file| (*file, path.names.as_slice()))
        })
    }
}

pub(super) fn last_name(file: &Path) -> Option<&str> {
    let stem = file.file_stem().and_then(|stem| stem.to_str());
    if stem == Some("__init__") { file.parent().and_then(Path::file_name).and_then(|name| name.to_str()) } else { stem }
}
