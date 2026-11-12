// Names the documentation references, which count as used even when no code of the run reaches them.
mod conf;
mod pages;
mod store;
mod walk;

use std::fs;
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::project::cache;
pub use conf::AutoclassContent;
use walk::{is_configuration, is_foreign, pages, tree_key};

#[derive(Clone, Default, PartialEq)]
pub struct DocReferences {
    // Sphinx resolves a role against the page's module, its class or the whole project, and `conf.py` may put any
    // directory on `sys.path`, so any definition whose dotted name ends with one may be the target.
    names: FxHashSet<String>,
    scopes: FxHashSet<String>,
    // `automodule` with `:members:` alone documents what the module's `__all__` lists, or its public names, which the
    // index resolves into names.
    modules: FxHashSet<Module>,
    // Per project root, the class docstring its Sphinx configuration renders, and the `class-doc-from` options its
    // pages give to classes. Each project's files read their own root's entry.
    roots: FxHashMap<PathBuf, RootDocs>,
}

#[derive(Clone, Default, PartialEq)]
pub(super) struct RootDocs {
    // The docstring for every class of the project that no directive names.
    pub(super) default: AutoclassContent,
    // The `class-doc-from` option of each `autoclass` directive, by the class name the directive gives.
    pub(super) directives: FxHashMap<String, AutoclassContent>,
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Module {
    pub target: String,
    pub exclude: Vec<String>,
    pub private: bool,
    pub special: bool,
}

// The index files that make a directory a Sphinx documentation root, which Sphinx requires as its master document.
const INDEXES: [&str; 3] = ["index.rst", "index.md", "index.txt"];

impl DocReferences {
    // With a cache directory, a root's names are kept under `tree_key`, so a run where nothing under the root changed
    // reads no page.
    pub fn collect<'a>(
        roots: impl IntoIterator<Item = (&'a Path, &'a ruff_workspace::Settings)>,
        main: &Path,
        cache_dir: Option<&Path>,
    ) -> Self {
        let mut references = Self::default();
        for (root, settings) in roots {
            let cache =
                cache_dir.map(|cache_dir| (cache::documentation_file(cache_dir, root), tree_key(root, settings, main)));
            if let Some(cached) = cache.as_ref().and_then(|(path, key)| Self::cached(path, *key)) {
                references.extend(cached);
                continue;
            }
            let mut found = Self::default();
            let mut configurations = Vec::new();
            let mut directives = FxHashMap::default();
            for path in pages(root, settings, main) {
                if is_configuration(&path) {
                    configurations.push(path);
                } else if let Ok(text) = fs::read_to_string(&path) {
                    found.read(&path, &text, &mut directives);
                }
            }
            let default = conf::agreed(configurations.iter().filter_map(|path| configuration(root, path)));
            found.roots.insert(root.to_path_buf(), RootDocs { default, directives });
            if let Some((path, key)) = cache {
                found.store(&path, key);
            }
            references.extend(found);
        }
        references
    }

    fn extend(&mut self, other: Self) {
        self.names.extend(other.names);
        self.scopes.extend(other.scopes);
        self.modules.extend(other.modules);
        self.roots.extend(other.roots);
    }

    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.iter()
    }

    // The docstring a class of `root` renders, when the project's documentation was read. A project that was not read
    // has no answer, so the caller keeps the class docstring's copy as the only one it knows.
    pub fn class_doc_from(&self, root: &Path, qualified: &str) -> Option<AutoclassContent> {
        self.roots.get(root).map(|docs| {
            let mut suffixes =
                std::iter::once(qualified).chain(qualified.match_indices('.').map(|(at, _)| &qualified[at + 1..]));
            suffixes.find_map(|suffix| docs.directives.get(suffix).copied()).unwrap_or(docs.default)
        })
    }

    pub fn add_names(&mut self, names: impl IntoIterator<Item = String>) {
        self.names.extend(names);
    }

    pub fn add_scope(&mut self, scope: String) {
        self.scopes.insert(scope);
    }

    // When the package is unknown, `qualified` starts at the module's own name, and a documented name that ends with it
    // matches under any package.
    pub fn covers(&self, qualified: &str, anchored: bool) -> bool {
        let mut suffixes =
            std::iter::once(qualified).chain(qualified.match_indices('.').map(|(at, _)| &qualified[at + 1..]));
        let below = |outer: &str, inner: &str| {
            outer.strip_suffix(inner).is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
        };
        suffixes.any(|suffix| {
            self.names.contains(suffix) || self.scopes.iter().any(|scope| suffix.starts_with(scope.as_str()))
        }) || (!anchored
            && (self.names.iter().any(|name| below(name, qualified))
                || qualified
                    .match_indices('.')
                    .any(|(at, _)| self.scopes.iter().any(|scope| below(scope, &qualified[..=at])))))
    }
}

// The docstring a Sphinx configuration of `root` renders for its classes, or `None` when `path` is not one. A vendored
// or other project's `conf.py` is not the project's. A configuration that cannot be read, or is over the size limit,
// may set the option, so its docstring is unknown.
fn configuration(root: &Path, path: &Path) -> Option<AutoclassContent> {
    if is_foreign(root, path) {
        return None;
    }
    let directory = path.parent()?;
    let has_index = INDEXES.iter().any(|index| directory.join(index).is_file());
    let text = fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.len() <= 8 << 20)
        .and_then(|_| fs::read_to_string(path).ok());
    match text {
        Some(text) => conf::class_doc_from(&text, has_index),
        None => Some(AutoclassContent::Unknown),
    }
}

impl CacheKey for DocReferences {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        for names in [&self.names, &self.scopes] {
            let mut sorted: Vec<&String> = names.iter().collect();
            sorted.sort_unstable();
            sorted.cache_key(state);
        }
        let mut modules: Vec<&Module> = self.modules.iter().collect();
        modules.sort_unstable();
        for module in modules {
            module.target.cache_key(state);
            module.exclude.cache_key(state);
            module.private.cache_key(state);
            module.special.cache_key(state);
        }
        let mut roots: Vec<(&PathBuf, &RootDocs)> = self.roots.iter().collect();
        roots.sort_unstable_by_key(|(root, _)| *root);
        for (root, docs) in roots {
            root.cache_key(state);
            docs.default.cache_key(state);
            let mut directives: Vec<(&String, &AutoclassContent)> = docs.directives.iter().collect();
            directives.sort_unstable_by_key(|(name, _)| *name);
            for (name, content) in directives {
                name.cache_key(state);
                content.cache_key(state);
            }
        }
    }
}
