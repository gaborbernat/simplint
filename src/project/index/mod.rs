mod api;
mod calls;
mod classes;
mod cycles;
mod documented;
mod duplicates;
mod entries;
mod enums;
mod exceptions;
mod files;
mod hatch;
mod inherit;
mod prefixes;
mod reach;
mod resolve;
mod run_imports;
mod run_reads;
mod search;
mod startup;
mod writes;

pub use api::{Exposure, FileIndex, ModuleUse};
pub use files::FileId;
pub use resolve::ImportResolver;
pub use search::SearchRoots;

use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_text_size::TextRange;

use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::{FileUses, RunCalls};
use crate::project::facts::{Channel, Channels, ImportBinding, Signature};
use crate::project::{DocReferences, FileFacts};
use crate::settings::ModuleRole;

use self::api::{FileAnswers, RunModules, module_uses};
use self::calls::run_calls;
use self::classes::{ClassGraph, Reexports};
use self::cycles::{Cycle, RuntimeGraph};
use self::documented::documented_members;
use self::duplicates::DuplicateRun;
use self::entries::{coverage_plugin_names, interned_entry_names, startup_entries};
use self::enums::enum_classes;
use self::exceptions::exception_classes;
use self::files::FileIds;
use self::hatch::hatch_hook_classes;
use self::prefixes::{PrefixSiblings, prefix_siblings};
use self::reach::{LastNames, StarTargets, library_api, reexport, signature_reached};
use self::resolve::UnreachedPath;
use self::run_imports::{Members, RunImports, run_import_kinds};
use self::run_reads::{RunReads, RunWide, run_names};

// Module names resolve with ty's resolver, the one Ruff's import graph uses, so a module is named as Python imports it.
pub struct Project {
    pub(super) imported: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) entry_only: FxHashMap<FileId, FxHashSet<String>>,
    // Renaming these breaks the dotted-string code that reaches them, yet they are not part of the module's API.
    pub(super) referenced: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) star_imported: FxHashSet<FileId>,
    pub(super) package_star_imported: FxHashSet<FileId>,
    pub(super) star_sources: FxHashSet<FileId>,
    pub(super) roles: FxHashMap<FileId, ModuleRole>,
    pub(super) library_api: FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    pub(super) test_only: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) read_whole: FxHashSet<FileId>,
    pub(super) loaded_as_value: FxHashSet<FileId>,
    pub(super) uncaught_exceptions: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) exception_types: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) enum_types: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) hook_classes: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) run_imports: FxHashMap<FileId, FxHashMap<String, bool>>,
    pub(super) run_modules: FxHashMap<FileId, FxHashMap<String, FileId>>,
    pub(super) run_members: FxHashMap<FileId, Members>,
    pub(super) run_signatures: FxHashMap<FileId, FxHashMap<String, Signature>>,
    pub(super) api_reached: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) library_reached: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) inherited: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>>,
    pub(super) overridden: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>>,
    // The files that run when a top-level package of the run is imported or one of its entry points runs.
    pub(super) package_loaded: FxHashSet<FileId>,
    // The offsets of each file's runtime import statements that reach a file of the run.
    pub(super) resolved_imports: FxHashMap<FileId, FxHashSet<u32>>,
    // The modules the run's files outside the tests import, which a package's `__init__.py` answers for the project.
    pub(super) imported_modules: FxHashSet<String>,
    pub(super) docs: DocReferences,
    pub(super) cycles: FxHashMap<FileId, Cycle>,
    pub(super) duplicates: FxHashMap<FileId, Vec<DuplicateRun>>,
    pub(super) prefix_siblings: FxHashMap<FileId, PrefixSiblings>,
    // Cycle and duplicate ranges are offsets into these sources, so the digest tells when they went stale.
    pub(super) sources: FxHashMap<FileId, u64>,
    pub(super) module_uses: FxHashMap<FileId, Vec<ModuleUse>>,
    pub(super) computed_writes: FxHashSet<FileId>,
    pub(super) written: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) calls: RunCalls,
    pub(super) wide: RunWide,
    pub(super) digest: u64,
    pub(super) probed: Option<u64>,
    pub(super) files: Arc<FileIds>,
}

pub struct IndexInputs<'a> {
    pub entry_points: &'a [(PathBuf, String)],
    pub build_hooks: &'a [(PathBuf, String)],
    pub coverage_plugins: &'a [(PathBuf, String)],
    pub startup: &'a [String],
    pub roles: &'a FxHashMap<&'a Path, ModuleRole>,
    pub test_modules: &'a FxHashSet<&'a Path>,
    pub distribution_roots: &'a FxHashMap<&'a Path, &'a Path>,
    pub docs: &'a DocReferences,
    pub duplicate_statements: Option<usize>,
    pub probed: Option<u64>,
    pub external: &'a External,
}

#[derive(Default)]
pub struct External {
    pub imported: FxHashSet<String>,
    pub read: FxHashSet<String>,
    pub assigned: FxHashSet<String>,
    pub opaque: FxHashSet<String>,
    pub computed: ComputedReads,
}

impl Project {
    #[expect(clippy::too_many_lines, reason = "the stages of the index pipeline share the gathered inputs")]
    pub fn index(files: &[(&Path, &FileFacts)], resolver: &mut ImportResolver, inputs: &IndexInputs) -> Self {
        let IndexInputs {
            entry_points,
            build_hooks,
            coverage_plugins,
            startup,
            docs,
            duplicate_statements,
            distribution_roots,
            external,
            probed,
            ..
        } = *inputs;
        let ids = resolver.resolve(files);
        let (test_modules, roles) = interned_inputs(resolver, inputs);
        let indexed: Vec<(FileId, &FileFacts)> = ids.iter().zip(files).map(|(id, (_, facts))| (*id, *facts)).collect();
        let docs = &documented_members(docs, &indexed, resolver);
        let (mut entry_names, bare_names) = interned_entry_names(resolver, entry_points, build_hooks, &indexed);
        let plugin_names = coverage_plugin_names(resolver, coverage_plugins);
        let (loaded, startup_reads) = startup_entries(resolver, startup);
        entry_names.extend(startup_reads);
        let entries: FxHashSet<FileId> = entry_names
            .iter()
            .map(|(file, _)| *file)
            .chain(loaded)
            .chain(indexed.iter().filter(|(_, facts)| facts.main_guard).map(|(id, _)| *id))
            .collect();
        let test_support = test_support(&importers(&indexed, resolver), &test_modules, &entries);
        let production: FxHashSet<FileId> = indexed
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| !test_modules.contains(id) && !test_support.contains(id))
            .collect();
        let Gathered {
            mut uses,
            stars,
            unresolved_stars,
            unreached,
            constructor_uses,
            bindings,
            module_names,
            runtime,
            opaque_names,
            computed_writes,
            written,
            read_whole,
        } = Gathered::of(&indexed, resolver, &test_modules, &production);
        for (file, name) in plugin_names {
            uses.read(file, &[name], Origin::Other);
        }
        let sources = non_stub_facts(&ids, files);
        let ids_of = &*resolver.files;
        let package_loaded = package_loaded(&runtime, &indexed, ids_of, &test_modules);
        let imported_modules: FxHashSet<String> = indexed
            .iter()
            .filter(|(id, _)| !test_modules.contains(id))
            .flat_map(|(_, facts)| facts.imported_modules.iter().cloned())
            .collect();
        let resolved_imports: FxHashMap<FileId, FxHashSet<u32>> = runtime
            .iter()
            .map(|(file, imports)| (*file, imports.iter().map(|(_, range)| range.start().to_u32()).collect()))
            .collect();
        // The cycle and repeated statement reports and what the run reads take nothing from the reach and re-export
        // steps, so they are worked out alongside them.
        let (aside, ()) = rayon::join(
            || {
                let (cycles, duplicates) = rayon::join(
                    || runtime_cycles(runtime, &test_modules),
                    || duplicate_reports(&sources, &test_modules, ids_of, duplicate_statements, distribution_roots),
                );
                (cycles, duplicates, RunReads::collect(&indexed))
            },
            || {
                // A dotted path that may reach any module of a package could come from any file, so it counts as used
                // outside the tests as well.
                let last_names = LastNames::new(ids_of, sources.keys().copied());
                for (file, names) in last_names.reached(unreached[Channel::Import].iter().copied()) {
                    uses.import(file, names, Origin::Other);
                }
                for (file, names) in last_names.reached(unreached[Channel::Reference].iter().copied()) {
                    uses.reference(file, names, Origin::Other);
                }
                for (file, names) in last_names.reached(unreached[Channel::Doctest].iter().copied()) {
                    uses.read(file, names, Origin::Other);
                }
                uses.reexport(&stars, &sources);
            },
        );
        let (cycle_reports, duplicate_reports, reads) = aside;
        let StarTargets { star_imported, package_star_imported, star_sources, package_bound } =
            StarTargets::of(&stars, &sources, &unresolved_stars, &resolver.files, &production);
        let mut wide = run_names(&indexed, &test_modules, reads, opaque_names, &external.assigned);
        let digest = wide.digest(docs);
        wide.computed_reads.extend(&external.computed);
        external_uses(&indexed, external, &mut uses);
        let entry_only = entry_uses(&entry_names, &bare_names, &mut uses);
        let Uses { imported, referenced, stored_references, from_tests, from_others } = uses;
        wide.opaque_names.extend(external.opaque.iter().cloned());
        let reexports = Reexports::new(&bindings, &stars);
        let graph = ClassGraph::of(&resolver.modules, &reexports);
        let RunImports { names: run_imports, modules: run_modules, members: run_members, signatures: run_signatures } =
            run_import_kinds(&resolver.modules, &bindings, &reexports);
        let (
            ((uncaught_exceptions, exception_types), module_uses),
            (mut calls, library_api, (api_reached, library_reached)),
        ) = rayon::join(
            || {
                rayon::join(
                    || {
                        exception_classes(
                            &resolver.modules,
                            resolver.python_version,
                            &test_modules,
                            &graph,
                            &reexports,
                            &bindings,
                        )
                    },
                    || {
                        module_uses(
                            module_names,
                            &sources,
                            &stored_references,
                            &computed_writes,
                            &cycle_reports,
                            ids_of,
                        )
                    },
                )
            },
            || {
                // The public names are needed first, since a library's public dataclasses take no rewrite.
                let library_api =
                    library_api(&sources, &roles, &imported, &entry_names, &bindings, &stars, &resolver.files);
                let (calls, signatures) = rayon::join(
                    || {
                        run_calls(
                            &indexed,
                            &sources,
                            &constructor_uses,
                            &imported,
                            &wide.module_reads,
                            entry_names,
                            &referenced,
                            &library_api,
                        )
                    },
                    || signature_reached(&indexed, &library_api),
                );
                (calls, library_api, signatures)
            },
        );
        let inherited = inherit::inherited_api(&library_api, &library_reached, &graph, &reexports);
        let overridden = inherit::overridden_methods(&graph, &reexports);
        let enum_types = enum_classes(&graph, &reexports, &bindings);
        calls.set_test_support(test_support);
        Self {
            imported,
            entry_only,
            referenced,
            star_imported,
            package_star_imported,
            star_sources,
            read_whole: read_whole.iter().copied().chain(package_bound).collect(),
            roles,
            library_api,
            test_only: only_tests(from_tests, &from_others),
            loaded_as_value: read_whole,
            uncaught_exceptions,
            exception_types,
            enum_types,
            hook_classes: hatch_hook_classes(&graph),
            run_imports,
            run_modules,
            run_members,
            run_signatures,
            api_reached,
            library_reached,
            inherited,
            overridden,
            package_loaded,
            resolved_imports,
            imported_modules,
            wide,
            docs: docs.clone(),
            cycles: cycle_reports,
            duplicates: duplicate_reports,
            prefix_siblings: prefix_siblings(&indexed, &resolver.files),
            sources: indexed.iter().map(|(id, facts)| (*id, facts.source_digest)).collect(),
            module_uses,
            computed_writes,
            written,
            calls,
            digest,
            probed,
            files: Arc::clone(&resolver.files),
        }
    }

    pub fn changed_names<'p>(&'p self, other: &'p Self) -> FxHashSet<&'p str> {
        self.wide.changed(&other.wide)
    }

    // Looked up once, so rules asking about many names do not normalize and hash the path each time.
    pub fn file(&self, path: &Path) -> FileIndex<'_> {
        let id = self.files.get(path);
        FileIndex {
            project: self,
            source: entry(&self.sources, id).copied(),
            answers: FileAnswers {
                imported: entry(&self.imported, id),
                entry_only: entry(&self.entry_only, id),
                referenced: entry(&self.referenced, id),
                star_imported: id.is_some_and(|id| self.star_imported.contains(&id)),
                package_star_imported: id.is_some_and(|id| self.package_star_imported.contains(&id)),
                star_source: id.is_some_and(|id| self.star_sources.contains(&id)),
                computed_written: id.is_some_and(|id| self.computed_writes.contains(&id)),
                written: id.and_then(|id| self.written.get(&id)),
                role: entry(&self.roles, id).copied().unwrap_or_default(),
                library_api: entry(&self.library_api, id),
                modules: entry(&self.module_uses, id).map_or(&[], Vec::as_slice),
                test_only: entry(&self.test_only, id),
                loaded_as_value: id.is_some_and(|id| self.loaded_as_value.contains(&id)),
                read_whole: id.is_some_and(|id| self.read_whole.contains(&id)),
                uncaught_exceptions: entry(&self.uncaught_exceptions, id),
                exception_types: entry(&self.exception_types, id),
                enum_types: entry(&self.enum_types, id),
                hook_classes: entry(&self.hook_classes, id),
                run_imports: entry(&self.run_imports, id),
                run_modules: RunModules { modules: entry(&self.run_modules, id), members: &self.run_members },
                run_signatures: entry(&self.run_signatures, id),
                api_reached: entry(&self.api_reached, id),
                library_reached: entry(&self.library_reached, id),
                inherited: entry(&self.inherited, id),
                overridden: entry(&self.overridden, id),
                package_loaded: id.is_some_and(|id| self.package_loaded.contains(&id)),
                resolved_imports: entry(&self.resolved_imports, id),
                imported_modules: path
                    .file_name()
                    .is_some_and(|name| name == "__init__.py")
                    .then_some(&self.imported_modules),
                cycle: entry(&self.cycles, id),
                duplicates: entry(&self.duplicates, id).map_or(&[], Vec::as_slice),
                prefix_siblings: entry(&self.prefix_siblings, id),
                calls: self.calls.file(id),
                reads: &self.wide.reads,
                computed_reads: &self.wide.computed_reads,
            },
        }
    }
}

fn interned_inputs(
    resolver: &mut ImportResolver,
    inputs: &IndexInputs,
) -> (FxHashSet<FileId>, FxHashMap<FileId, ModuleRole>) {
    let test_modules = inputs.test_modules.iter().map(|path| FileIds::intern(&mut resolver.files, path)).collect();
    let roles = inputs.roles.iter().map(|(path, role)| (FileIds::intern(&mut resolver.files, path), *role)).collect();
    (test_modules, roles)
}

fn importers(files: &[(FileId, &FileFacts)], resolver: &ImportResolver) -> FxHashMap<FileId, FxHashSet<FileId>> {
    let mut importers: FxHashMap<FileId, FxHashSet<FileId>> = FxHashMap::default();
    for (id, _) in files {
        let module = &resolver.modules[id];
        // A star import uses the names of the module it reads, as a named import does.
        for file in module.names[Channel::Import]
            .files
            .iter()
            .chain(&module.names[Channel::Reference].files)
            .map(|(file, _)| file)
            .chain(&module.stars)
        {
            importers.entry(*file).or_default().insert(*id);
        }
    }
    importers
}

// The modules that only test modules, or modules found this way, import: helpers such as a `testing/fixtures.py`, whose
// calls are test code though its path does not say so. An entry point or a module a `.pth` file loads runs without an
// import, and a module with a `__main__` guard runs as a program, so none of them is one.
fn test_support(
    importers: &FxHashMap<FileId, FxHashSet<FileId>>,
    test_modules: &FxHashSet<FileId>,
    entries: &FxHashSet<FileId>,
) -> FxHashSet<FileId> {
    let mut support: FxHashSet<FileId> = FxHashSet::default();
    loop {
        let found: Vec<FileId> = importers
            .iter()
            .filter(|(file, _)| !test_modules.contains(*file) && !support.contains(*file) && !entries.contains(*file))
            .filter(|(_, by)| by.iter().all(|importer| test_modules.contains(importer) || support.contains(importer)))
            .map(|(file, _)| *file)
            .collect();
        if found.is_empty() {
            return support;
        }
        support.extend(found);
    }
}

#[derive(Default)]
struct Gathered<'r> {
    uses: Uses,
    stars: Vec<(FileId, FileId)>,
    opaque_names: FxHashSet<String>,
    unresolved_stars: FxHashSet<&'r str>,
    unreached: Channels<Vec<&'r UnreachedPath>>,
    constructor_uses: Vec<FileUses<'r>>,
    bindings: Vec<(FileId, FileId, &'r ImportBinding)>,
    module_names: Vec<(FileId, String, String, FileId)>,
    runtime: RuntimeGraph,
    computed_writes: FxHashSet<FileId>,
    written: FxHashMap<FileId, FxHashSet<String>>,
    read_whole: FxHashSet<FileId>,
}

impl<'r> Gathered<'r> {
    fn of(
        indexed: &[(FileId, &FileFacts)],
        resolver: &'r ImportResolver,
        test_modules: &FxHashSet<FileId>,
        production: &FxHashSet<FileId>,
    ) -> Self {
        let mut gathered = Self { opaque_names: every(indexed, |facts| &facts.doctest_names), ..Self::default() };
        for (id, facts, module) in indexed.iter().map(|(id, facts)| (*id, *facts, &resolver.modules[id])) {
            gathered
                .module_names
                .extend(module.modules.iter().map(|(bound, name, target)| (id, bound.clone(), name.clone(), *target)));
            gathered.runtime.entry(id).or_default().extend(module.runtime.iter().copied());
            gathered.computed_writes.extend(module.computed_writes.iter().copied());
            for (file, names) in &module.written {
                gathered.written.entry(*file).or_default().extend(names.iter().cloned());
            }
            let uses = module.constructors.iter().map(|(file, uses)| (*file, uses.as_slice())).collect();
            let production = production.contains(&id);
            if production {
                gathered.read_whole.extend(module.values.iter().copied());
            }
            gathered.constructor_uses.push(FileUses {
                caller: id,
                path: resolver.files.path(id).to_path_buf(),
                digest: facts.source_digest,
                production,
                modules: uses,
            });
            gathered.bindings.extend(module.bindings.iter().map(|(source, binding)| (id, *source, binding)));
            let origin = if test_modules.contains(&id) { Origin::Test } else { Origin::Other };
            for (file, names) in &module.names[Channel::Import].files {
                gathered.uses.import(*file, names, origin);
            }
            for (file, names) in &module.names[Channel::Reference].files {
                gathered.uses.reference(*file, names, origin);
            }
            for (file, names) in &module.names[Channel::Doctest].files {
                gathered.uses.read(*file, names, origin);
            }
            gathered.stars.extend(module.stars.iter().map(|source| (id, *source)));
            gathered.opaque_names.extend(module.unresolved_names.iter().cloned());
            gathered.unresolved_stars.extend(module.unresolved_stars.iter().map(String::as_str));
            for channel in [Channel::Import, Channel::Reference, Channel::Doctest] {
                gathered.unreached[channel].extend(&module.names[channel].unreached);
            }
        }
        gathered
    }
}

fn every(files: &[(FileId, &FileFacts)], names: impl Fn(&FileFacts) -> &Vec<String>) -> FxHashSet<String> {
    every_in(files, |_| true, names)
}

fn every_in(
    files: &[(FileId, &FileFacts)],
    selected: impl Fn(FileId) -> bool,
    names: impl Fn(&FileFacts) -> &Vec<String>,
) -> FxHashSet<String> {
    files.iter().filter(|(id, _)| selected(*id)).flat_map(|(_, facts)| names(facts).iter().cloned()).collect()
}

#[derive(Clone, Copy)]
enum Origin {
    Test,
    Other,
}

#[derive(Default)]
struct Uses {
    imported: FxHashMap<FileId, FxHashSet<String>>,
    referenced: FxHashMap<FileId, FxHashSet<String>>,
    // Doctests' reads are in `referenced` but not here, as reading a name patches nothing.
    stored_references: FxHashMap<FileId, FxHashSet<String>>,
    from_tests: FxHashMap<FileId, FxHashSet<String>>,
    from_others: FxHashMap<FileId, FxHashSet<String>>,
}

impl Uses {
    fn import(&mut self, file: FileId, names: &[String], origin: Origin) {
        self.imported.entry(file).or_default().extend(names.iter().cloned());
        self.bucket(file, names, origin);
    }

    fn reference(&mut self, file: FileId, names: &[String], origin: Origin) {
        self.referenced.entry(file).or_default().extend(names.iter().cloned());
        self.stored_references.entry(file).or_default().extend(names.iter().cloned());
        self.bucket(file, names, origin);
    }

    fn read(&mut self, file: FileId, names: &[String], origin: Origin) {
        self.referenced.entry(file).or_default().extend(names.iter().cloned());
        self.bucket(file, names, origin);
    }

    fn bucket(&mut self, file: FileId, names: &[String], origin: Origin) {
        let bucket = match origin {
            Origin::Test => &mut self.from_tests,
            Origin::Other => &mut self.from_others,
        };
        bucket.entry(file).or_default().extend(names.iter().cloned());
    }

    fn reexport(&mut self, stars: &[(FileId, FileId)], sources: &FxHashMap<FileId, &FileFacts>) {
        for names in [&mut self.imported, &mut self.referenced, &mut self.from_tests, &mut self.from_others] {
            reexport(names, stars, sources);
        }
    }
}

// A stub declares its module's names, which the module's own source already holds.
fn non_stub_facts<'a>(ids: &[FileId], files: &[(&Path, &'a FileFacts)]) -> FxHashMap<FileId, &'a FileFacts> {
    ids.iter()
        .zip(files)
        .filter(|(_, (path, _))| path.extension().is_none_or(|extension| extension != "pyi"))
        .map(|(id, (_, facts))| (*id, *facts))
        .collect()
}

// The files a top-level package's `__init__.py` or `__main__.py` runs through runtime imports, itself included: the
// modules an install loads on its own, apart from the plugins other tools load, the integrations users import by choice
// and the scripts a checkout runs.
fn package_loaded(
    runtime: &RuntimeGraph,
    indexed: &[(FileId, &FileFacts)],
    files: &FileIds,
    test_modules: &FxHashSet<FileId>,
) -> FxHashSet<FileId> {
    let inits: FxHashSet<&Path> = indexed
        .iter()
        .map(|(id, _)| files.path(*id))
        .filter(|path| path.file_name().is_some_and(|name| name == "__init__.py"))
        .filter_map(Path::parent)
        .collect();
    let mut pending: Vec<FileId> = indexed
        .iter()
        .filter(|(id, _)| !test_modules.contains(id))
        .filter(|(id, _)| {
            let path = files.path(*id);
            path.file_name().is_some_and(|name| name == "__main__.py")
                || path.file_name().is_some_and(|name| name == "__init__.py")
                    && path.parent().and_then(Path::parent).is_none_or(|outer| !inits.contains(outer))
        })
        .map(|(id, _)| *id)
        .collect();
    let mut loaded = FxHashSet::default();
    while let Some(file) = pending.pop() {
        if loaded.insert(file) {
            pending.extend(runtime.get(&file).into_iter().flatten().map(|(target, _)| *target));
        }
    }
    loaded
}

// The cycles of the runtime imports outside test modules, which import each other as fixtures and deliberately close
// cycles when they test the import system.
fn runtime_cycles(mut runtime: RuntimeGraph, test_modules: &FxHashSet<FileId>) -> FxHashMap<FileId, Cycle> {
    runtime.retain(|file, _| !test_modules.contains(file));
    cycles::cycles(runtime)
}

// The statements each file shares with another, which `duplicate-code` reports. Stubs repeat each other's signatures by
// design, and tests repeat their setup and checks so each reads on its own, or copy the code they test to compare
// against it, so neither is compared.
fn duplicate_reports(
    sources: &FxHashMap<FileId, &FileFacts>,
    test_modules: &FxHashSet<FileId>,
    files: &FileIds,
    window: Option<usize>,
    distribution_roots: &FxHashMap<&Path, &Path>,
) -> FxHashMap<FileId, Vec<DuplicateRun>> {
    let Some(window) = window else { return FxHashMap::default() };
    let mut distributions: FxHashMap<&Path, Vec<(FileId, &FileFacts)>> = FxHashMap::default();
    for (file, facts) in sources.iter().filter(|(file, _)| !test_modules.contains(file)) {
        let path = files.path(*file);
        let root = distribution_roots.get(path).copied().unwrap_or(path);
        distributions.entry(root).or_default().push((*file, *facts));
    }
    distributions
        .into_values()
        .flat_map(|mut compared| {
            compared.sort_unstable_by_key(|(file, _)| *file);
            duplicates::duplicate_runs(&compared, files, window)
        })
        .collect()
}

// A name the file only imports is no definition of its own, so another project reading it is no use of the file.
fn external_uses(indexed: &[(FileId, &FileFacts)], external: &External, uses: &mut Uses) {
    for (file, facts) in indexed {
        let constructors = &facts.constructors;
        let defined = facts
            .defined
            .iter()
            .chain(&constructors.methods)
            .chain(&constructors.attributes)
            .chain(facts.classes.iter().map(|class| &class.name));
        for name in defined {
            if external.imported.contains(name.as_str()) {
                uses.imported.entry(*file).or_default().insert(name.clone());
            }
            if external.read.contains(name.as_str()) {
                uses.reference(*file, std::slice::from_ref(name), Origin::Other);
            }
        }
    }
}

// An entry point runs without any import, so its name counts as used by code outside the tests. The public names a bare
// module entry point loads are not imports either, so they are returned apart from the names other modules import.
fn entry_uses(
    entry_names: &[(FileId, String)],
    bare_names: &[(FileId, String)],
    uses: &mut Uses,
) -> FxHashMap<FileId, FxHashSet<String>> {
    let mut entry_only: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in bare_names {
        if !uses.imported.get(file).is_some_and(|names| names.contains(name)) {
            entry_only.entry(*file).or_default().insert(name.clone());
        }
    }
    for (file, name) in entry_names {
        uses.import(*file, std::slice::from_ref(name), Origin::Other);
    }
    entry_only
}

fn only_tests(
    from_tests: FxHashMap<FileId, FxHashSet<String>>,
    from_others: &FxHashMap<FileId, FxHashSet<String>>,
) -> FxHashMap<FileId, FxHashSet<String>> {
    from_tests
        .into_iter()
        .map(|(file, names)| {
            let others = from_others.get(&file);
            let names = names.into_iter().filter(|name| !others.is_some_and(|others| others.contains(name)));
            (file, names.collect())
        })
        .collect()
}

fn entry<V>(map: &FxHashMap<FileId, V>, id: Option<FileId>) -> Option<&V> {
    map.get(&id?)
}

fn range_key(range: TextRange, state: &mut CacheKeyHasher) {
    u32::from(range.start()).cache_key(state);
    u32::from(range.end()).cache_key(state);
}
