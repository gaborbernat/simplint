use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};

use crate::project::FileFacts;
use crate::project::constructors::FileCalls;
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::reach::unqualified;
use super::{FileId, Project};

// A module a file binds at its top level, as `from pkg import module` and `import pkg.module as module` do, and the
// file it resolves to.
pub struct ModuleUse {
    pub bound: String,
    // The absolute dotted name of the module, such as `pkg.module`.
    pub module: String,
    // Whether a dotted string names attributes of the module, as a `mock.patch` target does, so tests reach them there.
    pub patched: bool,
    // The names the module's top-level statements define, which `from module import name` can import.
    pub defined: FxHashSet<String>,
}

impl PartialEq for ModuleUse {
    fn eq(&self, other: &Self) -> bool {
        (&self.bound, &self.module, self.patched, &self.defined)
            == (&other.bound, &other.module, other.patched, &other.defined)
    }
}

impl CacheKey for ModuleUse {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.bound.cache_key(state);
        self.module.cache_key(state);
        self.patched.cache_key(state);
        self.defined.cache_key(state);
    }
}

// How users reach a name of a module: the module offers it, or another module re-exports it.
#[derive(Clone, Copy)]
pub enum Exposure<'a> {
    Offered,
    Reexported(&'a Path),
}

impl<'a> Exposure<'a> {
    pub fn reexporter(self) -> Option<&'a Path> {
        match self {
            Self::Offered => None,
            Self::Reexported(path) => Some(path),
        }
    }
}

// What the index knows about one file: the names other files use from it, and the attribute and base class names of
// the whole run.
#[derive(Clone, Copy)]
pub struct FileIndex<'a> {
    pub(super) project: &'a Project,
    pub(super) imported: Option<&'a FxHashSet<String>>,
    pub(super) referenced: Option<&'a FxHashSet<String>>,
    pub(super) star_imported: bool,
    pub(super) package_star_imported: bool,
    pub(super) star_source: bool,
    pub(super) role: ModuleRole,
    pub(super) user_facing: Option<&'a FxHashMap<String, Option<PathBuf>>>,
    pub(super) modules: &'a [ModuleUse],
    pub(super) test_only: Option<&'a FxHashSet<String>>,
    pub(super) uncaught_exceptions: Option<&'a FxHashSet<String>>,
    pub(super) api_reached: Option<&'a FxHashSet<String>>,
    pub(super) calls: FileCalls<'a>,
}

// Two indexes give a file the same answers when the names other files use from it, its dataclass calls and whether the
// run reads docstrings match; the attribute and base class names of the whole run are compared apart, by
// `Project::changed_names`.
impl PartialEq for FileIndex<'_> {
    fn eq(&self, other: &Self) -> bool {
        (
            self.imported,
            self.referenced,
            self.star_imported,
            self.package_star_imported,
            self.star_source,
            self.role,
            self.user_facing,
            self.modules,
            self.test_only,
            self.uncaught_exceptions,
            self.calls,
            self.project.reads_docstrings,
        ) == (
            other.imported,
            other.referenced,
            other.star_imported,
            other.package_star_imported,
            other.star_source,
            other.role,
            other.user_facing,
            other.modules,
            other.test_only,
            other.uncaught_exceptions,
            other.calls,
            other.project.reads_docstrings,
        ) && self.api_reached == other.api_reached
    }
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged.
impl CacheKey for FileIndex<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.project.digest.cache_key(state);
        self.imported.cache_key(state);
        self.referenced.cache_key(state);
        self.star_imported.cache_key(state);
        self.package_star_imported.cache_key(state);
        self.star_source.cache_key(state);
        self.role.cache_key(state);
        self.user_facing.cache_key(state);
        self.modules.iter().for_each(|module| module.cache_key(state));
        self.test_only.cache_key(state);
        self.uncaught_exceptions.cache_key(state);
        self.api_reached.cache_key(state);
        self.calls.cache_key(state);
    }
}

impl<'a> FileIndex<'a> {
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported.is_some_and(|names| names.contains(name))
    }

    // Whether a library's users reach the name, and how.
    pub fn user_facing(&self, name: &str) -> Option<Exposure<'a>> {
        self.user_facing?.get(name).map(|importer| importer.as_deref().map_or(Exposure::Offered, Exposure::Reexported))
    }

    pub fn role(&self) -> ModuleRole {
        self.role
    }

    // The modules this file binds at its top level, which a rule may reach through an attribute.
    pub fn modules(&self) -> &'a [ModuleUse] {
        self.modules
    }

    // Whether every use of the name in the run comes from a test module. An `import *` that may read a public name, or
    // a computed module read, may use it from anywhere, so those names count as used outside the tests.
    // Whether the file defines an exception class that no handler of the run catches by name, nor catches a subclass of.
    pub fn is_uncaught_exception(&self, name: &str) -> bool {
        self.uncaught_exceptions.is_some_and(|names| names.contains(name))
    }

    // Whether another exported name's annotations, bases or bounds reach the module's class or alias `name`, which makes
    // it public API though no file imports it.
    pub fn is_api_reached(&self, name: &str) -> bool {
        self.api_reached.is_some_and(|names| names.contains(name))
    }

    pub fn is_used_only_by_tests(&self, name: &str) -> bool {
        self.test_only.is_some_and(|names| names.contains(name))
            && !(self.star_imported && Visibility::of(name) == Visibility::Public)
            && !self.project.module_reads.contains(name)
    }

    // Whether some file star-imports the module, so the names its `__all__` lists land there.
    pub fn is_star_import_source(&self) -> bool {
        self.star_source
    }

    // A public name of a module some `import *` reads may be used under that import, though no file names it, and any
    // name may be read off a module loaded by a computed name.
    pub fn is_referenced(&self, name: &str) -> bool {
        self.is_imported(name)
            || self.referenced.is_some_and(|names| names.contains(name))
            || (self.star_imported && Visibility::of(name) == Visibility::Public)
            || self.project.module_reads.contains(name)
    }

    // Whether an `import *` takes every public name of the module, which adding an `__all__` would narrow.
    pub fn is_star_imported(&self) -> bool {
        self.star_imported
    }

    // Whether the `__init__.py` of the module's package star-imports it, so a name the module exports lands in the
    // package next to its submodules.
    pub fn is_package_star_imported(&self) -> bool {
        self.package_star_imported
    }

    pub fn is_attribute_assigned(&self, name: &str) -> bool {
        self.project.assigned_attributes.contains(name)
    }

    pub fn is_base_class(&self, name: &str) -> bool {
        self.project.base_classes.contains(name)
    }

    pub fn reads_docstrings(&self) -> bool {
        self.project.reads_docstrings
    }

    // How other files of the run use this file's dataclass `name`, and the positional fields of the dataclass a call
    // spelled `written` constructs in this file.
    pub fn dataclass_calls(&self) -> FileCalls<'a> {
        self.calls
    }

    // The attributes the run sets or deletes on instances it knows are of the class with this qualified name, `self`
    // included.
    pub fn attributes_written_on(&self, class: &str) -> impl Iterator<Item = &str> {
        self.project.writes.on_class.get(class).into_iter().flatten().map(String::as_str)
    }

    // Whether the run writes attributes on instances of another class with the same name, or has subclasses of it that a
    // slotted base breaks: a name that resolves differently than simplint sees, as through a re-export, may still be
    // this class.
    pub fn has_namesake_hazards(&self, class: &str) -> bool {
        let writes = &self.project.writes;
        let own =
            usize::from(writes.on_class.contains_key(class)) + usize::from(writes.unslottable_bases.contains(class));
        writes.namesakes.get(unqualified(class)).is_some_and(|count| *count > own)
    }

    // Whether the run sets or deletes an attribute with this name on an object of unknown class; mocks do not count.
    pub fn is_attribute_written(&self, name: &str) -> bool {
        self.project.writes.untyped.contains(name)
    }

    // Whether a class of the run that a slotted base breaks inherits from the class with this qualified name.
    pub fn has_unslottable_subclass(&self, name: &str) -> bool {
        self.project.writes.unslottable_bases.contains(name)
    }

    // Whether a class of the run whose methods store attributes on `self` inherits from the class with this qualified
    // name.
    pub fn has_stateful_subclass(&self, name: &str) -> bool {
        self.project.writes.stateful_bases.contains(name)
    }

    // Whether the run pickles with a protocol that may be 0 or 1.
    pub fn pickles_low_protocol(&self) -> bool {
        self.project.writes.pickles_low_protocol
    }
}

// The modules each file binds, with what the run knows of each. Only modules that resolve to a file of the run count,
// so a third-party or standard-library module never does.
pub(super) fn module_uses(
    names: Vec<(FileId, String, String, FileId)>,
    sources: &FxHashMap<FileId, &FileFacts>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
) -> FxHashMap<FileId, Vec<ModuleUse>> {
    let mut uses: FxHashMap<FileId, Vec<ModuleUse>> = FxHashMap::default();
    for (importer, bound, module, target) in names {
        let Some(facts) = sources.get(&target) else {
            continue;
        };
        let patched = referenced.get(&target).is_some_and(|names| !names.is_empty());
        let module = ModuleUse { bound, module, patched, defined: facts.defined.iter().cloned().collect() };
        uses.entry(importer).or_default().push(module);
    }
    uses
}
