// The Python objects a project's documentation names: Sphinx cross-reference roles, autodoc directives and the
// options of directives that render a parser from a module's function, in reStructuredText and MyST Markdown. A
// definition the documentation names is used, even when no code of the run reaches it.
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
    // The dotted names roles and directives give. Sphinx resolves a role against the page's module, its class or the
    // whole project, and `conf.py` may put any directory on `sys.path`, so any definition whose dotted name ends with
    // one may be the target.
    names: FxHashSet<String>,
    // `pkg.Class.` of `autoclass` with `:members:`, whose members are all documented, wherever the module's path starts.
    scopes: FxHashSet<String>,
    // The modules of `automodule` with `:members:` alone, whose members Sphinx renders: the ones the module's `__all__`
    // lists, or else its public names. The index resolves them to names, which needs the module's facts.
    modules: FxHashSet<Module>,
}

// A module `automodule` documents with `:members:`, with the members it leaves out and the private and special members
// it takes in.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Module {
    pub target: String,
    pub exclude: Vec<String>,
    pub private: bool,
    pub special: bool,
}

impl DocReferences {
    // The modules `automodule` documents with `:members:` alone.
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.iter()
    }

    // Adds the dotted names of the definitions a directive documents, which a reference reaches as it does a name.
    pub fn add_names(&mut self, names: impl IntoIterator<Item = String>) {
        self.names.extend(names);
    }

    // Adds a class whose members a directive documents, as `pkg.Class.`.
    pub fn add_scope(&mut self, scope: String) {
        self.scopes.insert(scope);
    }

    // Reads the `.rst` and `.md` files under each project root, skipping hidden directories, the paths Ruff excludes
    // from discovery there, and the nested projects of the run's project `main`, whose pages are their own. The walk is
    // the one discovery makes: `.gitignore` and `.ignore` files apply when the settings respect them, and symlinks are
    // not followed. With a cache directory, what a root's pages name is kept there under the key `tree_key` gives, so a
    // run where nothing under the root changed neither walks it nor reads a page.
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
                if let Ok(text) = fs::read_to_string(path) {
                    found.read(&text);
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

    // Whether the documentation names the definition with the dotted name `qualified`, such as `pkg.module.Class.name`.
    // When the run cannot tell the module's package, `qualified` starts at the module's own name, and a documented name
    // that ends with it, under whichever package, matches too.
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
