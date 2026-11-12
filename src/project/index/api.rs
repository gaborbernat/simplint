use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};

use crate::project::FileFacts;
use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::FileCalls;
use crate::project::facts::{PYTEST_PLUGINS_REQUEST, source_digest};
use crate::project::receivers::UNRESOLVED_CLASS;
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use super::cycles::Cycle;
use super::duplicates::DuplicateRun;
use super::reach::unqualified;
use super::run_imports::Members;
use super::{FileId, FileIds, Project, RunReads};

#[derive(PartialEq)]
pub struct ModuleUse {
    pub bound: String,
    pub module: String,
    // A dotted string or a store reaches these, and an import of one would miss the patch or the write.
    pub patched: FxHashSet<String>,
    pub defined: FxHashSet<String>,
    // A `global` statement rebinds these, and an import would keep the value from load time.
    pub rebound: FxHashSet<String>,
    // A write under a computed name may set the module's names, so an import would miss that write.
    pub computed_written: bool,
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

// The docstring rules require docs only for the first two.
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

#[derive(Clone, Copy)]
pub struct FileIndex<'a> {
    pub(super) project: &'a Project,
    pub(super) source: Option<u64>,
    pub(super) answers: FileAnswers<'a>,
}

// The members of the modules of the run a file binds. They compare and hash by content, so they are built only when
// two answers are compared or hashed, which is far less often than the answers are read.
#[derive(Clone, Copy)]
pub(super) struct RunModules<'a> {
    pub(super) modules: Option<&'a FxHashMap<String, FileId>>,
    pub(super) members: &'a FxHashMap<FileId, Members>,
}

impl PartialEq for RunModules<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.by_alias() == other.by_alias()
    }
}

impl<'a> RunModules<'a> {
    fn by_alias(&self) -> Option<BTreeMap<&'a str, &'a Members>> {
        self.modules
            .map(|modules| modules.iter().map(|(alias, target)| (alias.as_str(), &self.members[target])).collect())
    }
}

// The answers the index gives one file. Two indexes give a file the same answers when these match; the attribute and
// base class names of the whole run are compared apart, by `RunWide::changed`. `reads` and `computed_reads` belong to
// the run, and reach a file's answers through `project.digest` as well.
#[derive(Clone, Copy, PartialEq)]
#[expect(clippy::struct_excessive_bools, reason = "each flag answers a question about the file")]
pub(super) struct FileAnswers<'a> {
    pub(super) imported: Option<&'a FxHashSet<String>>,
    pub(super) entry_only: Option<&'a FxHashSet<String>>,
    pub(super) referenced: Option<&'a FxHashSet<String>>,
    pub(super) star_imported: bool,
    pub(super) package_star_imported: bool,
    pub(super) star_source: bool,
    pub(super) computed_written: bool,
    pub(super) written: Option<&'a FxHashSet<String>>,
    pub(super) role: ModuleRole,
    pub(super) library_api: Option<&'a FxHashMap<String, Option<PathBuf>>>,
    pub(super) modules: &'a [ModuleUse],
    pub(super) test_only: Option<&'a FxHashSet<String>>,
    pub(super) loaded_as_value: bool,
    pub(super) read_whole: bool,
    pub(super) uncaught_exceptions: Option<&'a FxHashSet<String>>,
    pub(super) exception_types: Option<&'a FxHashSet<String>>,
    pub(super) hook_classes: Option<&'a FxHashSet<String>>,
    pub(super) run_imports: Option<&'a FxHashMap<String, bool>>,
    pub(super) run_modules: RunModules<'a>,
    pub(super) api_reached: Option<&'a FxHashSet<String>>,
    pub(super) library_reached: Option<&'a FxHashSet<String>>,
    pub(super) inherited: Option<&'a FxHashMap<String, FxHashSet<String>>>,
    pub(super) cycle: Option<&'a Cycle>,
    pub(super) duplicates: &'a [DuplicateRun],
    pub(super) calls: FileCalls<'a>,
    pub(super) reads: &'a RunReads,
    pub(super) computed_reads: &'a ComputedReads,
}

impl PartialEq for FileIndex<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.answers == other.answers
    }
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged. The run's
// `reads` and `computed_reads` are not hashed here: `project.digest` covers them.
impl CacheKey for FileIndex<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        let FileAnswers {
            imported,
            entry_only,
            referenced,
            star_imported,
            package_star_imported,
            star_source,
            computed_written,
            written,
            role,
            library_api,
            modules,
            test_only,
            loaded_as_value,
            read_whole,
            uncaught_exceptions,
            exception_types,
            hook_classes,
            run_imports,
            run_modules,
            api_reached,
            library_reached,
            inherited,
            cycle,
            duplicates,
            calls,
            reads: _,
            computed_reads: _,
        } = self.answers;
        self.project.digest.cache_key(state);
        self.project.probed.cache_key(state);
        imported.cache_key(state);
        entry_only.cache_key(state);
        referenced.cache_key(state);
        star_imported.cache_key(state);
        package_star_imported.cache_key(state);
        star_source.cache_key(state);
        computed_written.cache_key(state);
        written.cache_key(state);
        role.cache_key(state);
        library_api.cache_key(state);
        for module in modules {
            module.cache_key(state);
        }
        test_only.cache_key(state);
        loaded_as_value.cache_key(state);
        read_whole.cache_key(state);
        uncaught_exceptions.cache_key(state);
        exception_types.cache_key(state);
        hook_classes.cache_key(state);
        run_imports.cache_key(state);
        run_modules.by_alias().cache_key(state);
        api_reached.cache_key(state);
        library_reached.cache_key(state);
        inherited.cache_key(state);
        cycle.is_some().cache_key(state);
        if let Some(cycle) = cycle {
            cycle.cache_key_with(&self.project.files, state);
        }
        duplicates.cache_key(state);
        calls.cache_key(state);
    }
}

impl<'a> FileIndex<'a> {
    // Whether another module of the run imports the name, as opposed to only an entry point naming it, which the host
    // loads without an import.
    pub fn is_imported_by_others(&self, name: &str) -> bool {
        self.is_imported(name) && !self.answers.entry_only.is_some_and(|names| names.contains(name))
    }

    pub fn is_imported(&self, name: &str) -> bool {
        self.answers.imported.is_some_and(|names| names.contains(name))
    }

    pub fn string_word_count(&self, word: &str) -> u32 {
        self.project.wide.reads.string_words.get(word).copied().unwrap_or_default()
    }

    pub fn library_exposure(&self, name: &str) -> Option<Exposure<'a>> {
        exposure(self.answers.library_api, self.answers.library_reached, name)
    }

    pub fn is_library_api(&self, name: &str) -> bool {
        self.library_exposure(name).is_some()
    }

    pub fn role(&self) -> ModuleRole {
        self.answers.role
    }

    pub fn modules(&self) -> &'a [ModuleUse] {
        self.answers.modules
    }

    pub fn is_uncaught_exception(&self, name: &str) -> bool {
        self.answers.uncaught_exceptions.is_some_and(|names| names.contains(name))
    }

    pub fn is_exception_type(&self, name: &str) -> bool {
        self.answers.exception_types.is_some_and(|names| names.contains(name))
    }

    pub fn is_hatch_hook_class(&self, name: &str) -> bool {
        self.answers.hook_classes.is_some_and(|names| names.contains(name))
    }

    pub fn run_import_class(&self, name: &str) -> Option<bool> {
        self.answers.run_imports.and_then(|names| names.get(name).copied())
    }

    pub fn run_module_member(&self, module: &str, name: &str) -> Option<bool> {
        let target = self.answers.run_modules.modules?.get(module)?;
        self.answers.run_modules.members[target].get(name).copied()
    }

    pub fn is_api_reached(&self, name: &str) -> bool {
        self.answers.api_reached.is_some_and(|names| names.contains(name))
    }

    // Users reach the member through the public class that inherits it, so it is part of the API.
    pub fn is_inherited_api(&self, class: &str, member: &str) -> bool {
        self.answers.inherited.and_then(|classes| classes.get(class)).is_some_and(|members| members.contains(member))
    }

    // A call through a value loaded from the file reaches its names, and no import shows that.
    pub fn is_loaded_as_value(&self) -> bool {
        self.answers.loaded_as_value
    }

    // A production file loads the module as a value, or a star import binds it, so any of its names may be read.
    pub fn is_read_whole(&self) -> bool {
        self.answers.read_whole
    }

    // An `import *` or a computed module read may use a public name from anywhere, so it is not test-only.
    pub fn is_used_only_by_tests(&self, name: &str) -> bool {
        self.answers.test_only.is_some_and(|names| names.contains(name)) && !self.is_read_unnamed(name)
    }

    fn is_read_unnamed(&self, name: &str) -> bool {
        (self.answers.star_imported && Visibility::of(name) == Visibility::Public)
            || self.project.wide.module_reads.contains(name)
    }

    pub fn is_star_import_source(&self) -> bool {
        self.answers.star_source
    }

    // Whether a write under a computed name may set any name of the module, as `setattr(module, name, value)` does, so
    // a rename or move of its variables breaks the write.
    pub fn is_computed_written(&self) -> bool {
        self.answers.computed_written
    }

    pub fn is_module_written(&self, name: &str) -> bool {
        self.answers.written.is_some_and(|names| names.contains(name))
    }

    // The cycle and duplicate ranges are offsets into the source the index read, so a changed source has none of them.
    // A fix round lints the fixed file again, when it reports one.
    pub fn at(self, source: &str) -> Self {
        if self.source == Some(source_digest(source)) {
            self
        } else {
            Self { answers: FileAnswers { cycle: None, duplicates: &[], ..self.answers }, ..self }
        }
    }

    pub fn import_cycle(&self) -> Option<&'a Cycle> {
        self.answers.cycle
    }

    pub fn path_of(&self, id: FileId) -> &'a Path {
        self.project.files.path(id)
    }

    pub fn duplicate_runs(&self) -> &'a [DuplicateRun] {
        self.answers.duplicates
    }

    pub fn is_referenced(&self, name: &str) -> bool {
        self.is_imported(name)
            || self.answers.referenced.is_some_and(|names| names.contains(name))
            || self.is_read_unnamed(name)
    }

    pub fn is_used_elsewhere(&self, name: &str) -> bool {
        self.is_referenced(name) || self.is_api_reached(name)
    }

    pub fn is_all_read(&self) -> bool {
        self.is_imported("__all__") || self.answers.referenced.is_some_and(|names| names.contains("__all__"))
    }

    pub fn is_star_imported(&self) -> bool {
        self.answers.star_imported
    }

    pub fn is_package_star_imported(&self) -> bool {
        self.answers.package_star_imported
    }

    // Whether a string or store of the run reaches `module.name`, as a test patching it does.
    pub fn is_path_referenced(&self, module: &str, name: &str) -> bool {
        self.project.wide.referenced_paths.contains(&format!("{module}.{name}"))
    }

    pub fn is_attribute_assigned(&self, name: &str) -> bool {
        self.project.wide.assigned_attributes.contains(name)
    }

    pub fn is_base_class(&self, name: &str) -> bool {
        self.project.wide.base_classes.contains(name)
    }

    pub fn computed_reads(&self) -> &'a ComputedReads {
        self.answers.computed_reads
    }

    // Whether code the index cannot follow may read the name: a module that resolves to no file of the run imports it,
    // or a doctest example spells it. Only a fix that would break such a read is withheld, never a report.
    pub fn is_read_opaquely(&self, name: &str) -> bool {
        self.project.wide.opaque_names.contains(name)
    }

    pub fn is_documented(&self, qualified: &str, anchored: bool) -> bool {
        self.project.docs.covers(qualified, anchored)
    }

    pub fn is_fixture_requested(&self, name: &str) -> bool {
        self.project.wide.fixture_requests.contains(name)
    }

    // A plugin's fixtures may request a fixture that overrides one of the run's, so the requests count.
    pub fn loads_pytest_plugins(&self) -> bool {
        self.project.wide.fixture_requests.contains(PYTEST_PLUGINS_REQUEST)
    }

    pub fn reads_docstrings(&self) -> bool {
        self.project.wide.reads.docstrings
    }

    // Renaming a function or class changes what a read of its `__name__` or `__qualname__` returns.
    pub fn reads_name_of(&self, name: &str) -> bool {
        self.project.wide.reads.reflected.contains(name)
    }

    // A class renamed here may be read by its name off an object.
    pub fn reads_class_names(&self) -> bool {
        self.project.wide.reads.class_names
    }

    pub fn calls(&self) -> FileCalls<'a> {
        self.answers.calls
    }

    pub fn attributes_written_on(&self, class: &str) -> impl Iterator<Item = &str> {
        self.project.wide.writes.on_class.get(class).into_iter().flatten().map(String::as_str)
    }

    // A re-export can hide which class a name is, so a namesake of this class may be the one written to.
    pub fn has_namesake_hazards(&self, class: &str) -> bool {
        let writes = &self.project.wide.writes;
        let own =
            usize::from(writes.on_class.contains_key(class)) + usize::from(writes.unslottable_bases.contains(class));
        writes.namesakes.get(unqualified(class)).is_some_and(|count| *count > own)
    }

    // Storing `__dict__` fails on a frozen instance, unlike reading it through `vars(obj)`.
    pub fn dict_stored_on(&self, class: &str) -> bool {
        self.project.wide.writes.dict_stores.contains(class)
    }

    // An object of unknown class may have this attribute set; mocks do not count.
    pub fn is_attribute_written(&self, name: &str) -> bool {
        self.project.wide.writes.untyped.contains(name)
    }

    pub fn is_class_attribute_accessed(&self, class: &str, name: &str) -> bool {
        let accessed = |class: &str| {
            self.project.wide.writes.class_attributes.get(class).is_some_and(|names| names.contains(name))
        };
        accessed(class) || accessed(UNRESOLVED_CLASS)
    }

    pub fn has_unslottable_subclass(&self, name: &str) -> bool {
        self.project.wide.writes.unslottable_bases.contains(name)
    }

    pub fn has_stateful_subclass(&self, name: &str) -> bool {
        self.project.wide.writes.stateful_bases.contains(name)
    }

    pub fn pickles_low_protocol(&self) -> bool {
        self.project.wide.writes.pickles_low_protocol
    }
}

// Only modules that resolve to a file of the run count, so a third-party or standard-library module never does.
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
