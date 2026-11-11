use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};

use crate::project::FileFacts;
use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::FileCalls;
use crate::project::facts::source_digest;
use crate::project::receivers::UNRESOLVED_CLASS;
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::cycles::Cycle;
use super::duplicates::DuplicateRun;
use super::reach::unqualified;
use super::run_imports::Members;
use super::{FileId, FileIds, Project};

// A module a file binds at its top level, as `from pkg import module` and `import pkg.module as module` do, and the
// file it resolves to.
#[derive(PartialEq)]
pub struct ModuleUse {
    pub bound: String,
    // The absolute dotted name of the module, such as `pkg.module`.
    pub module: String,
    // The names a dotted string or a store names in the module, as a `mock.patch` target or `module.NAME = value` does,
    // so an import of one of them would miss the patch or the write.
    pub patched: FxHashSet<String>,
    // The names the module's top-level statements define, which `from module import name` can import.
    pub defined: FxHashSet<String>,
    // The names the module's functions rebind through `global`, whose value an import would keep from load time.
    pub rebound: FxHashSet<String>,
    // Whether a write under a computed name may set any of the module's names, so an import would miss the write.
    pub computed_written: bool,
    // Whether the module is a package at the top of a package root, such as `pkg` for `pkg/__init__.py`.
    pub top_level_package: bool,
}

impl CacheKey for ModuleUse {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.bound.cache_key(state);
        self.module.cache_key(state);
        self.patched.cache_key(state);
        self.defined.cache_key(state);
        self.rebound.cache_key(state);
        self.computed_written.cache_key(state);
        self.top_level_package.cache_key(state);
    }
}

// How users reach a name of a module: the module offers it (through `__all__`, or as a public name without one), another
// module re-exports it, or a name the module exposes reaches it through a signature, as a base class or an annotation
// does. The docstring rules require docs only for the first two.
#[derive(Clone, Copy)]
pub enum Exposure<'a> {
    Offered,
    Reexported(&'a Path),
    ReachedBySignature,
}

impl<'a> Exposure<'a> {
    pub fn reexporter(self) -> Option<&'a Path> {
        match self {
            Self::Offered | Self::ReachedBySignature => None,
            Self::Reexported(path) => Some(path),
        }
    }

    pub fn is_offered(self) -> bool {
        !matches!(self, Self::ReachedBySignature)
    }
}

// What the index knows about one file: the names other files use from it, and the attribute and base class names of
// the whole run.
#[derive(Clone, Copy)]
pub struct FileIndex<'a> {
    pub(super) project: &'a Project,
    pub(super) imported: Option<&'a FxHashSet<String>>,
    pub(super) entry_only: Option<&'a FxHashSet<String>>,
    pub(super) referenced: Option<&'a FxHashSet<String>>,
    pub(super) star_imported: bool,
    pub(super) package_star_imported: bool,
    pub(super) star_source: bool,
    pub(super) id: Option<FileId>,
    pub(super) role: ModuleRole,
    pub(super) library_api: Option<&'a FxHashMap<String, Option<PathBuf>>>,
    pub(super) modules: &'a [ModuleUse],
    pub(super) test_only: Option<&'a FxHashSet<String>>,
    pub(super) uncaught_exceptions: Option<&'a FxHashSet<String>>,
    pub(super) exception_types: Option<&'a FxHashSet<String>>,
    pub(super) hook_classes: Option<&'a FxHashSet<String>>,
    pub(super) run_imports: Option<&'a FxHashMap<String, bool>>,
    pub(super) run_modules: Option<&'a FxHashMap<String, FileId>>,
    pub(super) api_reached: Option<&'a FxHashSet<String>>,
    pub(super) library_reached: Option<&'a FxHashSet<String>>,
    pub(super) inherited: Option<&'a FxHashMap<String, FxHashSet<String>>>,
    pub(super) cycle: Option<&'a Cycle>,
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
                self.entry_only,
                self.referenced,
                self.star_imported,
                self.package_star_imported,
                self.star_source,
                self.is_computed_written(),
                self.written(),
                self.role,
                self.library_api,
                self.modules,
            ),
            (
                self.test_only,
                self.is_loaded_as_value(),
                self.is_read_whole(),
                self.uncaught_exceptions,
                self.exception_types,
                self.hook_classes,
                self.run_imports,
                self.run_module_members(),
                self.calls,
                (&self.project.reads, &self.project.computed_reads),
            ),
            (self.api_reached, self.library_reached, self.inherited, self.cycle, self.duplicates),
        )
    }
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged. The run's
// `reads` and `computed_reads` are not hashed here: `project.digest` covers them.
impl CacheKey for FileIndex<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.project.digest.cache_key(state);
        self.imported.cache_key(state);
        self.entry_only.cache_key(state);
        self.referenced.cache_key(state);
        self.star_imported.cache_key(state);
        self.package_star_imported.cache_key(state);
        self.star_source.cache_key(state);
        self.is_computed_written().cache_key(state);
        self.written().cache_key(state);
        self.role.cache_key(state);
        self.library_api.cache_key(state);
        self.modules.iter().for_each(|module| module.cache_key(state));
        self.test_only.cache_key(state);
        self.is_loaded_as_value().cache_key(state);
        self.is_read_whole().cache_key(state);
        self.uncaught_exceptions.cache_key(state);
        self.exception_types.cache_key(state);
        self.hook_classes.cache_key(state);
        self.run_imports.cache_key(state);
        self.run_module_members().cache_key(state);
        self.api_reached.cache_key(state);
        self.library_reached.cache_key(state);
        self.inherited.cache_key(state);
        self.cycle.is_some().cache_key(state);
        if let Some(cycle) = self.cycle {
            cycle.cache_key_with(&self.project.files, state);
        }
        self.duplicates.cache_key(state);
        self.calls.cache_key(state);
    }
}

impl<'a> FileIndex<'a> {
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported.is_some_and(|names| names.contains(name))
    }

    // Whether another module of the run imports the name, as opposed to only an entry point naming it, which the host
    // loads without an import.
    pub fn is_imported_by_others(&self, name: &str) -> bool {
        self.is_imported(name) && !self.entry_only.is_some_and(|names| names.contains(name))
    }

    // How many files of the run hold `word` as a whole word of a string literal, the file being checked among them.
    pub fn string_word_count(&self, word: &str) -> u32 {
        self.project.reads.string_words.get(word).copied().unwrap_or_default()
    }

    // Whether the name is part of a library's public API, and how users reach it: what a public module's `__all__`
    // lists, or its public names without one, a re-export, or a name another exported name's signature reaches.
    pub fn library_exposure(&self, name: &str) -> Option<Exposure<'a>> {
        exposure(self.library_api, self.library_reached, name)
    }

    pub fn is_library_api(&self, name: &str) -> bool {
        self.library_exposure(name).is_some()
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

    // Whether the name, as the file binds it, is an exception class of the run: one it defines, or imports from a file of
    // the run, which derives from a builtin or standard-library exception directly or through other classes of the run.
    pub fn is_exception_type(&self, name: &str) -> bool {
        self.exception_types.is_some_and(|names| names.contains(name))
    }

    // Whether the file defines a class that subclasses hatchling's hook interface, directly or through a base of the run.
    pub fn is_hatch_hook_class(&self, name: &str) -> bool {
        self.hook_classes.is_some_and(|names| names.contains(name))
    }

    // Whether the name the file binds by an import from a file of the run is a class there (true), or a function (false).
    // `None` when the import comes from outside the run, or the run defines the name as neither.
    pub fn run_import_class(&self, name: &str) -> Option<bool> {
        self.run_imports.and_then(|names| names.get(name).copied())
    }

    // The members of each module the file binds, by the name it binds the module as. They compare and hash by content,
    // not by the files that hold them, so a fix that changes what a module defines reaches the files that import it.
    fn run_module_members(&self) -> Option<BTreeMap<&'a str, &'a Members>> {
        self.run_modules.map(|modules| {
            modules.iter().map(|(alias, target)| (alias.as_str(), &self.project.run_members[target])).collect()
        })
    }

    // Whether the attribute `name` of a module of the run that the file binds as `module` is a class there (true), or a
    // function (false). `None` when the index does not know it.
    pub fn run_module_member(&self, module: &str, name: &str) -> Option<bool> {
        let target = self.run_modules?.get(module)?;
        self.project.run_members[target].get(name).copied()
    }

    // Whether another exported name's annotations, bases or bounds reach the module's class or alias `name`, which makes
    // it public API though no file imports it.
    pub fn is_api_reached(&self, name: &str) -> bool {
        self.api_reached.is_some_and(|names| names.contains(name))
    }

    // Whether a public class of the library inherits the member `member` of the class `class` this file defines
    // without overriding it, so users reach the member through that public class.
    pub fn is_inherited_api(&self, class: &str, member: &str) -> bool {
        self.inherited.and_then(|classes| classes.get(class)).is_some_and(|members| members.contains(member))
    }

    // Whether production code of the run loads the file as a value, so a call through that value reaches its names, and
    // no import of a name shows the use.
    pub fn is_loaded_as_value(&self) -> bool {
        self.id.is_some_and(|id| self.project.loaded_as_value.contains(&id))
    }

    // Whether the module is read as a whole, so any of its names may be read: a production file of the run loads it as
    // a value, or a star import binds it from its package's `__all__`.
    pub fn is_read_whole(&self) -> bool {
        self.id.is_some_and(|id| self.project.read_whole.contains(&id))
    }

    // Whether every use of the name in the run comes from a test module. An `import *` that may read a public name, or
    // a computed module read, may use it from anywhere, so those names count as used outside the tests.
    pub fn is_used_only_by_tests(&self, name: &str) -> bool {
        self.test_only.is_some_and(|names| names.contains(name)) && !self.is_read_unnamed(name)
    }

    // Whether some file star-imports the module, so the names its `__all__` lists land there.
    pub fn is_star_import_source(&self) -> bool {
        self.star_source
    }

    // Whether a write under a computed name may set any name of the module, as `setattr(module, name, value)` does, so
    // a rename or move of its variables breaks the write.
    pub fn is_computed_written(&self) -> bool {
        self.id.is_some_and(|id| self.project.computed_writes.contains(&id))
    }

    // Whether a `setattr` call of the run sets the module's name, as `monkeypatch.setattr(module, "NAME", value)` does.
    pub fn is_module_written(&self, name: &str) -> bool {
        self.written().is_some_and(|names| names.contains(name))
    }

    fn written(&self) -> Option<&'a FxHashSet<String>> {
        self.id.and_then(|id| self.project.written.get(&id))
    }

    // The index as it holds for `source`. The cycle and repeated statements it keeps for the file are ranges
    // of the source it read, so a source that fixes have changed since, which the fix loop lints, has none of them;
    // a fix round lints the fixed file again, when it reports one.
    pub fn at(self, source: &str) -> Self {
        if self.source == Some(source_digest(source)) { self } else { Self { cycle: None, duplicates: &[], ..self } }
    }

    // The statement that closes a cycle of runtime imports this file takes part in, if it is the file that reports it.
    pub fn import_cycle(&self) -> Option<&'a Cycle> {
        self.cycle
    }

    // The path of a file of the run, by the id a cycle names it with.
    pub fn path_of(&self, id: FileId) -> &'a Path {
        self.project.files.path(id)
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
            || self.is_read_unnamed(name)
    }

    // Whether code reads the name without naming it: an `import *` of the module reads its public names, and a computed
    // module read may read any name.
    fn is_read_unnamed(&self, name: &str) -> bool {
        (self.star_imported && Visibility::of(name) == Visibility::Public) || self.project.module_reads.contains(name)
    }

    // Whether another file, or a doctest of the run, reads the name, or the signature of an exported name reaches it,
    // so it is not unused. Code the index cannot follow may read any bare name, so it does not count here.
    pub fn is_used_elsewhere(&self, name: &str) -> bool {
        self.is_referenced(name) || self.is_api_reached(name)
    }

    // Whether another file imports the module's `__all__`, or reads it off an import of the module, as `module.__all__`.
    pub fn is_all_read(&self) -> bool {
        self.is_imported("__all__") || self.referenced.is_some_and(|names| names.contains("__all__"))
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

    // Whether code the index cannot follow may read the name: a module that resolves to no file of the run imports it,
    // or a doctest example spells it. Only a fix that would break such a read is withheld, never a report.
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

    // Whether the run stores `__dict__` on an instance of the class with this qualified name, which a frozen instance
    // rejects, unlike a read of it such as `vars(obj)`.
    pub fn dict_stored_on(&self, class: &str) -> bool {
        self.project.writes.dict_stores.contains(class)
    }

    // Whether the run sets or deletes an attribute with this name on an object of unknown class; mocks do not count.
    pub fn is_attribute_written(&self, name: &str) -> bool {
        self.project.writes.untyped.contains(name)
    }

    // Whether the run reads, sets or deletes the attribute `name` through the class with this qualified name, as
    // `Config.timeout` does, through a subclass of it, or through an access simplint cannot tie to a class.
    pub fn is_class_attribute_accessed(&self, class: &str, name: &str) -> bool {
        let accessed =
            |class: &str| self.project.writes.class_attributes.get(class).is_some_and(|names| names.contains(name));
        accessed(class) || accessed(UNRESOLVED_CLASS)
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
    computed_writes: &FxHashSet<FileId>,
    files: &FileIds,
) -> FxHashMap<FileId, Vec<ModuleUse>> {
    let mut uses: FxHashMap<FileId, Vec<ModuleUse>> = FxHashMap::default();
    for (importer, bound, module, target) in names {
        let Some(facts) = sources.get(&target) else {
            continue;
        };
        let patched = referenced.get(&target).cloned().unwrap_or_default();
        let rebound = facts.global_names.iter().cloned().collect();
        let defined = facts.defined.iter().cloned().collect();
        let computed_written = computed_writes.contains(&target);
        let top_level_package = !module.contains('.') && files.path(target).ends_with("__init__.py");
        let module = ModuleUse { bound, module, patched, defined, rebound, computed_written, top_level_package };
        uses.entry(importer).or_default().push(module);
    }
    uses
}

// How users reach `name` of a file, given the names it offers or re-exports, and the names a signature of an exposed
// name reaches.
pub(super) fn exposure<'a>(
    offered: Option<&'a FxHashMap<String, Option<PathBuf>>>,
    signatures: Option<&FxHashSet<String>>,
    name: &str,
) -> Option<Exposure<'a>> {
    match offered.and_then(|names| names.get(name)) {
        Some(importer) => Some(importer.as_deref().map_or(Exposure::Offered, Exposure::Reexported)),
        None => signatures.is_some_and(|names| names.contains(name)).then_some(Exposure::ReachedBySignature),
    }
}
