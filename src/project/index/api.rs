use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};

use ruff_text_size::TextRange;

use crate::project::FileFacts;
use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::FileCalls;
use crate::project::facts::source_digest;
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::cycles::Cycle;
use super::duplicates::DuplicateRun;
use super::reach::{StarSite, unqualified};
use super::{FileId, Project};

// A module a file binds at its top level, as `from pkg import module` and `import pkg.module as module` do, and the
// file it resolves to.
#[derive(PartialEq)]
pub struct ModuleUse {
    pub bound: String,
    // The absolute dotted name of the module, such as `pkg.module`.
    pub module: String,
    // Whether a dotted string names attributes of the module, as a `mock.patch` target does, so tests reach them there.
    pub patched: bool,
    // The names the module's top-level statements define, which `from module import name` can import.
    pub defined: FxHashSet<String>,
    // The names the module's functions rebind through `global`, whose value an import would keep from load time.
    pub rebound: FxHashSet<String>,
}

impl CacheKey for ModuleUse {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.bound.cache_key(state);
        self.module.cache_key(state);
        self.patched.cache_key(state);
        self.defined.cache_key(state);
        self.rebound.cache_key(state);
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
    pub(super) cycle: Option<&'a Cycle>,
    pub(super) star_sites: &'a [StarSite],
    pub(super) duplicates: &'a [DuplicateRun],
    pub(super) source: Option<u64>,
    pub(super) calls: FileCalls<'a>,
}

// Two indexes give a file the same answers when their `answers` match; the attribute and base class names of the whole
// run are compared apart, by `Project::changed_names`.
impl PartialEq for FileIndex<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.answers() == other.answers()
    }
}

impl FileIndex<'_> {
    // The answers the index gives its file, which `PartialEq` compares. `reads` and `computed_reads` belong to
    // the run, and reach a file's answers through `project.digest` as well.
    fn answers(&self) -> impl PartialEq + '_ {
        (
            (
                self.imported,
                self.referenced,
                self.star_imported,
                self.package_star_imported,
                self.star_source,
                self.role,
                self.user_facing,
                self.modules,
            ),
            (self.test_only, self.uncaught_exceptions, self.calls, (&self.project.reads, &self.project.computed_reads)),
            (self.api_reached, self.cycle, self.star_sites, self.duplicates),
        )
    }
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged. The run's
// `reads` and `computed_reads` are not hashed here: `project.digest` covers them.
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
        self.cycle.cache_key(state);
        self.star_sites.cache_key(state);
        self.duplicates.cache_key(state);
        self.calls.cache_key(state);
    }
}

impl<'a> FileIndex<'a> {
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported.is_some_and(|names| names.contains(name))
    }

    // How many files of the run hold `word` as a whole word of a string literal, the file being checked among them.
    pub fn string_word_count(&self, word: &str) -> u32 {
        self.project.reads.string_words.get(word).copied().unwrap_or_default()
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

    // Whether the file defines an exception class that no handler of the run catches by name, nor catches a subclass of.
    pub fn is_uncaught_exception(&self, name: &str) -> bool {
        self.uncaught_exceptions.is_some_and(|names| names.contains(name))
    }

    // Whether another exported name's annotations, bases or bounds reach the module's class or alias `name`, which makes
    // it public API though no file imports it.
    pub fn is_api_reached(&self, name: &str) -> bool {
        self.api_reached.is_some_and(|names| names.contains(name))
    }

    // Whether every use of the name in the run comes from a test module. An `import *` that may read a public name, or
    // a computed module read, may use it from anywhere, so those names count as used outside the tests.
    pub fn is_used_only_by_tests(&self, name: &str) -> bool {
        self.test_only.is_some_and(|names| names.contains(name))
            && !(self.star_imported && Visibility::of(name) == Visibility::Public)
            && !self.project.module_reads.contains(name)
    }

    // Whether some file star-imports the module, so the names its `__all__` lists land there.
    pub fn is_star_import_source(&self) -> bool {
        self.star_source
    }

    // The index as it holds for `source`. The cycle, `import *` and repeated statements it keeps for the file are ranges
    // of the source it read, so a source that fixes have changed since, which the fix loop lints, has none of them;
    // the next round indexes the fixed file again.
    pub fn at(self, source: &str) -> Self {
        if self.source == Some(source_digest(source)) {
            self
        } else {
            Self { cycle: None, star_sites: &[], duplicates: &[], ..self }
        }
    }

    // The statement that closes a cycle of runtime imports this file takes part in, if it is the file that reports it.
    pub fn import_cycle(&self) -> Option<&'a Cycle> {
        self.cycle
    }

    // The `import *` statement of this file at `range`, with the names it takes from the module it imports.
    pub fn star_site(&self, range: TextRange) -> Option<&'a StarSite> {
        self.star_sites.iter().find(|site| site.range() == range)
    }

    // The runs of statements of this file that another file repeats.
    pub fn duplicate_runs(&self) -> &'a [DuplicateRun] {
        self.duplicates
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

    // The attributes the run reads under computed names.
    pub fn computed_reads(&self) -> &'a ComputedReads {
        &self.project.computed_reads
    }

    // Whether code the index cannot follow reads the name: a module that resolves to no file of the run imports it, or
    // a doctest example uses it, so the name may be a definition of the run.
    pub fn is_read_opaquely(&self, name: &str) -> bool {
        self.project.opaque_names.contains(name)
    }

    // Whether the project's documentation names the definition with the dotted name `qualified`.
    pub fn is_documented(&self, qualified: &str, anchored: bool) -> bool {
        self.project.docs.covers(qualified, anchored)
    }

    // Whether a test module requests the fixture by parameter, `usefixtures` or `getfixturevalue`.
    pub fn is_fixture_requested(&self, name: &str) -> bool {
        self.project.fixture_requests.contains(name)
    }

    // Whether a test module sets `pytest_plugins`, whose fixtures may request a fixture of the run that overrides one of
    // theirs.
    pub fn loads_pytest_plugins(&self) -> bool {
        self.project.fixture_requests.contains("pytest_plugins")
    }

    pub fn reads_docstrings(&self) -> bool {
        self.project.reads.docstrings
    }

    // Whether some file of the run reads the `__name__` or `__qualname__` of an object named `name`, so renaming the
    // function or class changes what it returns.
    pub fn reads_name_of(&self, name: &str) -> bool {
        self.project.reads.reflected.contains(name)
    }

    // Whether a file of the run reads a class's name off an object, so a class renamed here may be read by that name.
    pub fn reads_class_names(&self) -> bool {
        self.project.reads.class_names
    }

    // How other files of the run use this file's dataclass `name`, and the positional fields of the dataclass a call
    // spelled `written` constructs in this file.
    pub fn calls(&self) -> FileCalls<'a> {
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
        let rebound = facts.global_names.iter().cloned().collect();
        let defined = facts.defined.iter().cloned().collect();
        let module = ModuleUse { bound, module, patched, defined, rebound };
        uses.entry(importer).or_default().push(module);
    }
    uses
}
