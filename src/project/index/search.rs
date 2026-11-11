use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::fs;
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::package::PackageRoot;
use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::identifiers::is_identifier;

use crate::project::cache::{Stat, stat};

use super::resolve::normalized;
use super::startup;

#[derive(Clone)]
pub struct SearchRoots {
    pub(super) roots: Vec<PathBuf>,
    pub(super) python_version: PythonVersion,
}

impl CacheKey for SearchRoots {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.roots.cache_key(state);
        std::hash::Hash::hash(&self.python_version, state);
    }
}

impl SearchRoots {
    // A cached run that skips the resolver must not miss what it sees beyond the run's files. A directory's time moves
    // when an entry is added, removed or renamed, so the directories stand in for their entries.
    pub fn layout_key<'a>(&self, files: impl Iterator<Item = &'a Path>, hasher: &mut CacheKeyHasher) {
        Self::hash_directories(&Self::walk(&self.roots, files), hasher);
    }

    fn hash_directories(directories: &[PathBuf], hasher: &mut CacheKeyHasher) {
        let stats: Vec<Option<Stat>> = directories.par_iter().map(|directory| stat(directory)).collect();
        for (directory, stat) in directories.iter().zip(stats) {
            directory.cache_key(hasher);
            stat.cache_key(hasher);
        }
    }

    fn walk<'a>(roots: &[PathBuf], files: impl Iterator<Item = &'a Path>) -> Vec<PathBuf> {
        let mut level: Vec<PathBuf> = roots.to_vec();
        level.extend(files.filter_map(Path::parent).flat_map(|directory| {
            let root = roots.iter().find(|root| directory.starts_with(root));
            directory
                .ancestors()
                .take_while(move |ancestor| root.is_some_and(|root| ancestor.starts_with(root)))
                .map(Path::to_path_buf)
        }));
        let mut walked: FxHashSet<PathBuf> = FxHashSet::default();
        // Only a link can lead back to a directory already walked, so only a link's target is resolved to stop a loop.
        // Each level is listed on all threads and its links are taken in order, so the same tree gives the same key.
        let mut linked: FxHashSet<PathBuf> = FxHashSet::default();
        while !level.is_empty() {
            level.retain(|directory| walked.insert(directory.clone()));
            let below: Vec<(PathBuf, Option<PathBuf>)> =
                level.par_iter().flat_map_iter(|directory| importable_directories(directory)).collect();
            level = below
                .into_iter()
                .filter_map(|(path, target)| target.is_none_or(|target| linked.insert(target)).then_some(path))
                .collect();
        }
        let mut walked: Vec<PathBuf> = walked.into_iter().collect();
        walked.sort_unstable();
        walked
    }

    // The sources `private-import-path` looks for may sit outside the search roots, so the cache keys those trees too.
    pub fn probed_layout(root: &Path, hasher: &mut CacheKeyHasher) {
        let directories = Self::walk(&[root.to_path_buf(), root.join("src")], std::iter::empty());
        Self::hash_directories(&directories, hasher);
        let mut links: Vec<PathBuf> =
            directories.par_iter().flat_map_iter(|directory| module_links(directory)).collect();
        links.sort_unstable();
        for link in links {
            link.cache_key(hasher);
            link.is_file().cache_key(hasher);
        }
    }

    pub fn new<'a>(settings: &ruff_workspace::Settings, packages: impl IntoIterator<Item = PackageRoot<'a>>) -> Self {
        let mut roots: Vec<PathBuf> = settings.linter.src.iter().filter(|path| path.is_dir()).cloned().collect();
        for root in packages.into_iter().filter_map(|package| package.path().parent()) {
            if !roots.iter().any(|known| known == root) {
                roots.push(root.to_path_buf());
            }
        }
        Self { roots, python_version: settings.linter.unresolved_target_version.linter_version() }
    }

    pub fn startup_names(&self) -> Vec<String> {
        startup::names(&self.roots)
    }

    // Ruff does not count a namespace package, one without an `__init__.py`, as a package, but Python still names its
    // modules from the root above them.
    pub fn module_path(&self, path: &Path) -> Option<Vec<String>> {
        self.module_paths(path).into_iter().next()
    }

    // The module a file imports as from each search root above it, the innermost first: a namespace package under `src`
    // is `namespace.package` from `src`, though the directory above the package is a search root too.
    pub fn module_paths(&self, path: &Path) -> Vec<Vec<String>> {
        let path = normalized(path);
        let mut roots: Vec<&PathBuf> = self.roots.iter().filter(|root| path.starts_with(root)).collect();
        roots.sort_by_key(|root| std::cmp::Reverse(root.components().count()));
        roots
            .into_iter()
            .filter_map(|root| {
                path.strip_prefix(root)
                    .ok()?
                    .iter()
                    .map(|part| Path::new(part).file_stem()?.to_str().map(str::to_owned))
                    .collect()
            })
            .collect()
    }
}

fn importable_directories(directory: &Path) -> Vec<(PathBuf, Option<PathBuf>)> {
    fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            name.to_str().is_some_and(|name| is_identifier(name.strip_suffix("-stubs").unwrap_or(name)))
        })
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            let path = entry.path();
            if kind.is_dir() {
                Some((path, None))
            } else if kind.is_symlink() && path.is_dir() {
                let target = fs::canonicalize(&path).ok()?;
                Some((path, Some(target)))
            } else {
                None
            }
        })
        .collect()
}

// The links to a `.py` or `.pyi` file in a directory. A link's target can go away without changing any entry of the
// directory, so each link is keyed by whether it still leads to a file.
fn module_links(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_symlink()))
        .map(|entry| entry.path())
        .filter(|path| matches!(path.extension().and_then(|extension| extension.to_str()), Some("py" | "pyi")))
        .collect()
}
