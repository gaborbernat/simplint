// Names the documentation references, which count as used even when no code of the run reaches them.
mod pages;
mod store;
mod walk;

use std::fs;
use std::path::Path;

use ruff_cache::{CacheKey, CacheKeyHasher};
use rustc_hash::FxHashSet;

use crate::project::cache;
use walk::{pages, tree_key};

#[derive(Clone, Default, PartialEq)]
pub struct DocReferences {
    // Sphinx resolves a role against the page's module, its class or the whole project, and `conf.py` may put any
    // directory on `sys.path`, so any definition whose dotted name ends with one may be the target.
    names: FxHashSet<String>,
    scopes: FxHashSet<String>,
    // `automodule` with `:members:` alone documents what the module's `__all__` lists, or its public names, which the
    // index resolves into names.
    modules: FxHashSet<Module>,
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Module {
    pub target: String,
    pub exclude: Vec<String>,
    pub private: bool,
    pub special: bool,
}

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
            for path in pages(root, settings, main) {
                if let Ok(text) = fs::read_to_string(&path) {
                    found.read(&path, &text);
                }
            }
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
    }

    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.iter()
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
    }
}
