mod api;
mod cycles;
mod duplicates;
mod exceptions;
mod reach;
mod resolve;

pub use api::{Exposure, FileIndex, ModuleUse};
pub use reach::StarSite;
pub use resolve::ImportResolver;

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_graph::ResolverEnvironment;
use ruff_linter::package::PackageRoot;
use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_text_size::TextRange;

use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::{FileUses, RunCalls};
use crate::project::docs::stat_key;
use crate::project::facts::ImportBinding;
use crate::project::{DocReferences, FileFacts};
use crate::settings::ModuleRole;

use self::api::module_uses;
use self::cycles::{Cycle, RuntimeGraph};
use self::duplicates::DuplicateRun;
use self::exceptions::uncaught_exceptions;
use self::reach::{StarTargets, api_reached, reach_by_last_name, reexport, unqualified, user_facing};
use self::resolve::{Unreached, normalized, resolve};

// Where imports resolve, as `ruff analyze graph` searches: the configured `src` directories first, then the directory
// above each package.
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
    // What the resolver sees beyond the run's own files, which a cached run that skips it must not miss. It only asks
    // which files and directories exist, under each root and under each directory between an importing file and its
    // root, which ty searches too for a name the roots do not hold. A directory's time changes whenever an entry in it
    // is added, removed or renamed. Below those it walks only directories an import can name, `name-stubs` providing
    // `name`, and follows links as the resolver does, each directory once.
    pub fn layout_key<'a>(&self, files: impl Iterator<Item = &'a Path>, hasher: &mut CacheKeyHasher) {
        let mut pending: Vec<PathBuf> = self.roots.clone();
        pending.extend(files.filter_map(Path::parent).flat_map(|directory| {
            let root = self.roots.iter().find(|root| directory.starts_with(root));
            directory
                .ancestors()
                .take_while(move |ancestor| root.is_some_and(|root| ancestor.starts_with(root)))
                .map(Path::to_path_buf)
        }));
        let mut seen: FxHashSet<PathBuf> = FxHashSet::default();
        let mut walked: Vec<PathBuf> = Vec::new();
        while let Some(directory) = pending.pop() {
            if !seen.insert(fs::canonicalize(&directory).unwrap_or_else(|_| directory.clone())) {
                continue;
            }
            pending.extend(
                fs::read_dir(&directory)
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .filter(|entry| {
                        entry
                            .file_name()
                            .to_str()
                            .is_some_and(|name| is_identifier(name.strip_suffix("-stubs").unwrap_or(name)))
                    })
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir()),
            );
            walked.push(directory);
        }
        walked.sort_unstable();
        for directory in walked {
            stat_key(&directory, hasher);
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

    // The module a file imports as from the innermost search root above it, which is how Python names a module of a
    // namespace package, one without an `__init__.py` that Ruff does not count as a package.
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

// A run file by a number, so the index keys its maps by copies of it instead of hashing its path.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileId(u32);

// Every path the index has seen, numbered in the order they were first seen. Ids stay valid for the whole run, as a
// fix round indexes the same files again.
#[derive(Clone, Default)]
pub struct FileIds {
    pub(super) ids: FxHashMap<PathBuf, FileId>,
    pub(super) paths: Vec<PathBuf>,
}

impl FileIds {
    // Paths are absolute, as the run hands them over, so a lookup hashes the path as it is.
    // A path that was never interned has no entries in the index.
    fn get(&self, path: &Path) -> Option<FileId> {
        self.ids.get(path).copied()
    }

    fn path(&self, id: FileId) -> &Path {
        &self.paths[id.0 as usize]
    }

    // Copy on write, so a project that still holds the table keeps its ids; numbering only grows when a path is new.
    fn intern(files: &mut Arc<Self>, path: &Path) -> FileId {
        if let Some(id) = files.ids.get(path) {
            return *id;
        }
        let files = Arc::make_mut(files);
        let id = FileId(u32::try_from(files.paths.len()).expect("the run has fewer files than a FileId counts"));
        files.ids.insert(path.to_path_buf(), id);
        files.paths.push(path.to_path_buf());
        id
    }
}

// Which names of each file other files use, keyed by the file a name resolves to. Module names resolve with ty's module
// resolver, the one Ruff's import graph uses, so a module counts under the name Python would import it by, whether its
// package has an `__init__.py` or sits in a pytest rootdir.
pub struct Project {
    pub(super) imported: FxHashMap<FileId, FxHashSet<String>>,
    // Names other code reaches by a dotted string, such as a `mock.patch` target: renaming them breaks that code, yet
    // they are not part of the module's API.
    pub(super) referenced: FxHashMap<FileId, FxHashSet<String>>,
    // Modules without a static `__all__` that an `import *` may read, so each of their public names may be in use.
    pub(super) star_imported: FxHashSet<FileId>,
    // Modules their own package's `__init__.py` star-imports, where the package binds the submodule under its own name.
    pub(super) package_star_imported: FxHashSet<FileId>,
    // Modules some file star-imports, whatever their `__all__`, so a name they export may be in use where it lands.
    pub(super) star_sources: FxHashSet<FileId>,
    // Where each file sits in its library's public API.
    pub(super) roles: FxHashMap<FileId, ModuleRole>,
    // The names of each file a library's users can reach: those its public module exports, and those a public module
    // re-exports, each with the file that re-exports it, if any.
    pub(super) user_facing: FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    // The names a file uses from other files only when they are test modules, which no other file uses.
    pub(super) test_only: FxHashMap<FileId, FxHashSet<String>>,
    // The exception classes each file defines that no handler of the run names by name, or through a subclass.
    pub(super) uncaught_exceptions: FxHashMap<FileId, FxHashSet<String>>,
    // The classes and aliases each file defines that another exported name's signature reaches.
    pub(super) api_reached: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) assigned_attributes: FxHashSet<String>,
    pub(super) module_reads: FxHashSet<String>,
    pub(super) base_classes: FxHashSet<String>,
    // The fixtures test modules request by parameter, `usefixtures` or `getfixturevalue`, so pytest may use them.
    pub(super) fixture_requests: FxHashSet<String>,
    // The attributes the files read under computed names, which may reach any attribute or those with a prefix.
    pub(super) computed_reads: ComputedReads,
    pub(super) docs: DocReferences,
    // The names code the index cannot follow reads: those a file imports or reads from a module that resolves to no file
    // of the run, and those its doctest examples use.
    pub(super) opaque_names: FxHashSet<String>,
    // The statement each file's runtime imports close a cycle with, keyed by the file that reports it.
    pub(super) cycles: FxHashMap<FileId, Cycle>,
    // The `import *` statements of each file, with what each takes from the module it imports.
    pub(super) star_sites: FxHashMap<FileId, Vec<StarSite>>,
    // The statements each file shares with another file, which `duplicate-code` reports.
    pub(super) duplicates: FxHashMap<FileId, Vec<DuplicateRun>>,
    // The digest of each file's source the index read, whose offsets the ranges of `cycles`, `star_sites` and
    // `duplicates` are.
    pub(super) sources: FxHashMap<FileId, u64>,
    // The modules each file binds at its top level, keyed by that file.
    pub(super) module_uses: FxHashMap<FileId, Vec<ModuleUse>>,
    pub(super) writes: Writes,
    pub(super) calls: RunCalls,
    // What the files of the run read in common, which every file's answers depend on.
    pub(super) reads: RunReads,
    pub(super) digest: u64,
    pub(super) files: Arc<FileIds>,
}

// The attribute writes of the run: by the qualified class of the instance written to when parsing tells, and otherwise
// by name.
#[derive(Default)]
pub(super) struct Writes {
    pub(super) on_class: FxHashMap<String, FxHashSet<String>>,
    pub(super) untyped: FxHashSet<String>,
    pub(super) unslottable_bases: FxHashSet<String>,
    pub(super) stateful_bases: FxHashSet<String>,
    pub(super) pickles_low_protocol: bool,
    // How many entries of `on_class` and `unslottable_bases` each unqualified class name has.
    pub(super) namesakes: FxHashMap<String, usize>,
}

// What the index reads besides the files' facts.
pub struct IndexInputs<'a> {
    pub entry_points: &'a [String],
    pub rewrites_calls: &'a FxHashSet<&'a Path>,
    pub roles: &'a FxHashMap<&'a Path, ModuleRole>,
    pub test_modules: &'a FxHashSet<&'a Path>,
    // The Python objects the project's documentation names.
    pub docs: &'a DocReferences,
    // The fewest statements a duplicate block of any file of the run spans, when a file checks for duplicate code.
    pub duplicate_statements: Option<usize>,
}

impl Project {
    #[expect(clippy::too_many_lines, reason = "the stages of the index pipeline share the gathered inputs")]
    pub fn index(files: &[(&Path, &FileFacts)], resolver: &mut ImportResolver, inputs: &IndexInputs) -> Self {
        let IndexInputs { entry_points, docs, duplicate_statements, .. } = *inputs;
        let ids = resolver.resolve(files);
        let (rewrites_calls, test_modules, roles) = interned_inputs(resolver, inputs);
        let entry_names = interned_entry_names(resolver, entry_points);
        let indexed: Vec<(FileId, &FileFacts)> = ids.iter().zip(files).map(|(id, (_, facts))| (*id, *facts)).collect();
        let test_support = test_support(&importers(&indexed, resolver), &test_modules, &entry_names);
        let Gathered {
            mut imported,
            mut referenced,
            mut from_tests,
            mut from_others,
            stars,
            unresolved_stars,
            unreached,
            constructor_uses,
            bindings,
            module_names,
            runtime,
            star_pairs,
            opaque_names,
        } = Gathered::of(&indexed, resolver, &rewrites_calls, &test_modules, &test_support);
        let sources = non_stub_facts(&ids, files);
        let ids_of = &*resolver.files;
        // The cycle and repeated statement reports and what the run reads take nothing from the reach and re-export
        // steps, so they are worked out alongside them.
        let (aside, ()) = rayon::join(
            || {
                let (cycles, duplicates) = rayon::join(
                    || runtime_cycles(runtime, &test_modules, ids_of),
                    || duplicate_reports(&sources, &test_modules, ids_of, duplicate_statements),
                );
                let (reads, api_reached) = rayon::join(|| RunReads::collect(&indexed), || api_reached(&indexed));
                (cycles, duplicates, reads, api_reached)
            },
            || {
                // A dotted path that may reach any module of a package could come from any file, so it counts as used
                // outside the tests as well.
                reach_by_last_name(&mut referenced, &unreached, ids_of, sources.keys().copied());
                reach_by_last_name(&mut from_others, &unreached, ids_of, sources.keys().copied());
                for names in [&mut imported, &mut referenced, &mut from_tests, &mut from_others] {
                    reexport(names, &stars, &sources);
                }
            },
        );
        let (cycle_reports, duplicate_reports, reads, api_reached) = aside;
        let StarTargets { star_imported, package_star_imported, star_sources } =
            StarTargets::of(&stars, &sources, &unresolved_stars, &resolver.files);
        let RunNames {
            assigned_attributes,
            module_reads,
            base_classes,
            fixture_requests,
            computed_reads,
            writes,
            digest,
        } = run_names(&indexed, &test_modules, (&reads, docs), &opaque_names);
        entry_uses(&entry_names, &mut imported, &mut from_others);
        let ((uncaught_exceptions, module_uses), (mut calls, user_facing)) = rayon::join(
            || {
                rayon::join(
                    || {
                        uncaught_exceptions(
                            &indexed,
                            &resolver.modules,
                            resolver.python_version,
                            &test_modules,
                            &bindings,
                            &stars,
                        )
                    },
                    || module_uses(module_names, &sources, &referenced),
                )
            },
            || {
                rayon::join(
                    || {
                        run_calls(
                            &indexed,
                            &sources,
                            &constructor_uses,
                            &imported,
                            &module_reads,
                            entry_names,
                            &referenced,
                        )
                    },
                    || user_facing(&sources, &roles, &imported, &bindings, &stars, &resolver.files),
                )
            },
        );
        calls.set_test_support(test_support);
        Self {
            imported,
            referenced,
            star_imported,
            package_star_imported,
            star_sources,
            roles,
            user_facing,
            test_only: only_tests(from_tests, &from_others),
            uncaught_exceptions,
            api_reached,
            assigned_attributes,
            module_reads,
            base_classes,
            fixture_requests,
            computed_reads,
            opaque_names,
            docs: docs.clone(),
            cycles: cycle_reports,
            star_sites: reach::star_site_map(star_pairs, &sources),
            duplicates: duplicate_reports,
            sources: indexed.iter().map(|(id, facts)| (*id, facts.source_digest)).collect(),
            module_uses,
            writes,
            calls,
            reads,
            digest,
            files: Arc::clone(&resolver.files),
        }
    }

    // The words whose count of files differs between the two indexes, or that only one holds.
    fn string_word_changes<'p>(&'p self, other: &'p Self) -> impl Iterator<Item = &'p str> {
        let (own, others) = (&self.reads.string_words, &other.reads.string_words);
        let counts =
            own.iter().filter(|(word, count)| others.get(*word) != Some(*count)).map(|(word, _)| word.as_str());
        let only_other = others.keys().filter(|word| !own.contains_key(*word));
        counts.chain(only_other.map(String::as_str))
    }

    // The attribute, module read and base class names one index has and the other lacks, whose answers differ between
    // the two, and the names of the classes whose attribute writes, or subclasses that a slotted base breaks, differ,
    // which a dataclass of that name, or a namesake of it, reads. Low protocol pickling concerns every dataclass, so its
    // change names `dataclass`.
    pub fn changed_names<'p>(&'p self, other: &'p Self) -> FxHashSet<&'p str> {
        let (writes, others) = (&self.writes, &other.writes);
        let classes = writes
            .on_class
            .iter()
            .filter(|(class, attributes)| others.on_class.get(*class) != Some(*attributes))
            .chain(others.on_class.iter().filter(|(class, _)| !writes.on_class.contains_key(*class)))
            .map(|(class, _)| class)
            .chain(writes.unslottable_bases.symmetric_difference(&others.unslottable_bases))
            .chain(writes.stateful_bases.symmetric_difference(&others.stateful_bases))
            .map(|class| unqualified(class));
        let pickling = (writes.pickles_low_protocol != others.pickles_low_protocol).then_some("dataclass");
        self.assigned_attributes
            .symmetric_difference(&other.assigned_attributes)
            .chain(self.module_reads.symmetric_difference(&other.module_reads))
            .chain(self.base_classes.symmetric_difference(&other.base_classes))
            .chain(self.fixture_requests.symmetric_difference(&other.fixture_requests))
            .chain(self.opaque_names.symmetric_difference(&other.opaque_names))
            .chain(writes.untyped.symmetric_difference(&others.untyped))
            .map(String::as_str)
            .chain(classes)
            .chain(pickling)
            .chain(self.string_word_changes(other))
            .collect()
    }

    // A file's entry is looked up once, so rules asking about many names do not normalize and hash its path each time.
    // A file the index never saw has no entries, so it gets the same empty answers as one with none.
    pub fn file(&self, path: &Path) -> FileIndex<'_> {
        let id = self.files.get(path);
        FileIndex {
            project: self,
            imported: entry(&self.imported, id),
            referenced: entry(&self.referenced, id),
            star_imported: id.is_some_and(|id| self.star_imported.contains(&id)),
            package_star_imported: id.is_some_and(|id| self.package_star_imported.contains(&id)),
            star_source: id.is_some_and(|id| self.star_sources.contains(&id)),
            role: entry(&self.roles, id).copied().unwrap_or_default(),
            user_facing: entry(&self.user_facing, id),
            modules: entry(&self.module_uses, id).map_or(&[], Vec::as_slice),
            test_only: entry(&self.test_only, id),
            uncaught_exceptions: entry(&self.uncaught_exceptions, id),
            api_reached: entry(&self.api_reached, id),
            cycle: entry(&self.cycles, id),
            star_sites: entry(&self.star_sites, id).map_or(&[], Vec::as_slice),
            duplicates: entry(&self.duplicates, id).map_or(&[], Vec::as_slice),
            source: entry(&self.sources, id).copied(),
            calls: self.calls.file(id),
        }
    }
}

// What the modules of the run contribute to the index, gathered in one pass over them.
#[derive(Default)]
struct Gathered<'r> {
    imported: FxHashMap<FileId, FxHashSet<String>>,
    referenced: FxHashMap<FileId, FxHashSet<String>>,
    // The names the test modules use and the names the other modules use, so a name only tests use is found.
    from_tests: FxHashMap<FileId, FxHashSet<String>>,
    from_others: FxHashMap<FileId, FxHashSet<String>>,
    stars: Vec<(FileId, FileId)>,
    opaque_names: FxHashSet<String>,
    unresolved_stars: FxHashSet<&'r str>,
    unreached: Vec<Unreached<'r>>,
    constructor_uses: Vec<FileUses<'r>>,
    bindings: Vec<(FileId, FileId, &'r ImportBinding)>,
    module_names: Vec<(FileId, String, String, FileId)>,
    runtime: RuntimeGraph,
    star_pairs: Vec<(FileId, TextRange, FileId)>,
}

impl<'r> Gathered<'r> {
    fn of(
        indexed: &[(FileId, &FileFacts)],
        resolver: &'r ImportResolver,
        rewrites_calls: &FxHashSet<FileId>,
        test_modules: &FxHashSet<FileId>,
        test_support: &FxHashSet<FileId>,
    ) -> Self {
        let mut gathered = Self { opaque_names: every(indexed, |facts| &facts.doctest_names), ..Self::default() };
        for (id, module) in indexed.iter().map(|(id, _)| (*id, &resolver.modules[id])) {
            gathered
                .module_names
                .extend(module.modules.iter().map(|(bound, name, target)| (id, bound.clone(), name.clone(), *target)));
            gathered.runtime.entry(id).or_default().extend(module.runtime.iter().copied());
            gathered.star_pairs.extend(module.star_sites.iter().map(|(target, range)| (id, *range, *target)));
            let uses = module.constructors.iter().map(|(file, uses)| (*file, uses.as_slice())).collect();
            let production = !test_modules.contains(&id) && !test_support.contains(&id);
            gathered.constructor_uses.push(FileUses {
                caller: id,
                rewrites_calls: rewrites_calls.contains(&id),
                production,
                modules: uses,
            });
            gathered.bindings.extend(module.bindings.iter().map(|(source, binding)| (id, *source, binding)));
            let bucket = if test_modules.contains(&id) { &mut gathered.from_tests } else { &mut gathered.from_others };
            for (file, names) in &module.imports {
                gathered.imported.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in &module.references {
                gathered.referenced.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            gathered.stars.extend(module.stars.iter().map(|source| (id, *source)));
            gathered.opaque_names.extend(module.unresolved_names.iter().cloned());
            gathered.unresolved_stars.extend(module.unresolved_stars.iter().map(String::as_str));
            gathered.unreached.extend(
                module
                    .unreached
                    .iter()
                    .map(|(package, name, names)| (package.as_path(), name.as_str(), names.as_slice())),
            );
        }
        gathered
    }
}

// The facts of the files that are not stubs, by id: a stub declares the names of its module, whose source holds them.
fn non_stub_facts<'a>(ids: &[FileId], files: &[(&Path, &'a FileFacts)]) -> FxHashMap<FileId, &'a FileFacts> {
    ids.iter()
        .zip(files)
        .filter(|(_, (path, _))| path.extension().is_none_or(|extension| extension != "pyi"))
        .map(|(id, (_, facts))| (*id, *facts))
        .collect()
}

// The cycles of the runtime imports outside test modules, which import each other as fixtures and deliberately close
// cycles when they test the import system.
fn runtime_cycles(
    mut runtime: RuntimeGraph,
    test_modules: &FxHashSet<FileId>,
    files: &FileIds,
) -> FxHashMap<FileId, Cycle> {
    runtime.retain(|file, _| !test_modules.contains(file));
    cycles::cycles(runtime, files)
}

// The statements each file shares with another, which `duplicate-code` reports. Stubs repeat each other's signatures by
// design, and tests repeat their setup and checks so each reads on its own, or copy the code they test to compare
// against it, so neither is compared.
fn duplicate_reports(
    sources: &FxHashMap<FileId, &FileFacts>,
    test_modules: &FxHashSet<FileId>,
    files: &FileIds,
    window: Option<usize>,
) -> FxHashMap<FileId, Vec<DuplicateRun>> {
    let Some(window) = window else { return FxHashMap::default() };
    let mut compared: Vec<(FileId, &FileFacts)> =
        sources.iter().filter(|(file, _)| !test_modules.contains(file)).map(|(file, facts)| (*file, *facts)).collect();
    compared.sort_unstable_by_key(|(file, _)| *file);
    duplicates::duplicate_runs(&compared, files, window)
}

// The entry of a file's id, if the file has one.
fn entry<V>(map: &FxHashMap<FileId, V>, id: Option<FileId>) -> Option<&V> {
    map.get(&id?)
}

// The names the files of the run list, and the writes they make, each gathered in parallel as the files are read in
// common by every one.
struct RunNames {
    assigned_attributes: FxHashSet<String>,
    module_reads: FxHashSet<String>,
    base_classes: FxHashSet<String>,
    fixture_requests: FxHashSet<String>,
    computed_reads: ComputedReads,
    writes: Writes,
    digest: u64,
}

fn run_names(
    indexed: &[(FileId, &FileFacts)],
    test_modules: &FxHashSet<FileId>,
    reads: (&RunReads, &DocReferences),
    opaque_names: &FxHashSet<String>,
) -> RunNames {
    let ((assigned_attributes, module_reads, base_classes), (fixture_requests, (computed_reads, writes))) = rayon::join(
        || {
            (
                every(indexed, |facts| &facts.assigned_attributes),
                every(indexed, |facts| &facts.module_reads),
                every(indexed, |facts| &facts.base_classes),
            )
        },
        || {
            rayon::join(
                || fixture_requests(indexed, test_modules),
                || rayon::join(|| computed_reads(indexed), || Writes::collect(indexed)),
            )
        },
    );
    let names = [&assigned_attributes, &module_reads, &base_classes, &fixture_requests, opaque_names];
    let digest = digest(reads, &computed_reads, names, &writes);
    RunNames { assigned_attributes, module_reads, base_classes, fixture_requests, computed_reads, writes, digest }
}

// The names any file of the run lists in one of its facts.
fn every(files: &[(FileId, &FileFacts)], names: impl Fn(&FileFacts) -> &Vec<String>) -> FxHashSet<String> {
    every_in(files, |_| true, names)
}

// The files that import each file of the run, by an import or a dotted reference.
fn importers(files: &[(FileId, &FileFacts)], resolver: &ImportResolver) -> FxHashMap<FileId, FxHashSet<FileId>> {
    let mut importers: FxHashMap<FileId, FxHashSet<FileId>> = FxHashMap::default();
    for (id, _) in files {
        let module = &resolver.modules[id];
        for (file, _) in module.imports.iter().chain(&module.references) {
            importers.entry(*file).or_default().insert(*id);
        }
    }
    importers
}

// The modules that only test modules, or modules found this way, import: helpers such as a `testing/fixtures.py`, whose
// calls are test code though its path does not say so. An entry point runs without an import, so it is never one.
fn test_support(
    importers: &FxHashMap<FileId, FxHashSet<FileId>>,
    test_modules: &FxHashSet<FileId>,
    entry_names: &[(FileId, String)],
) -> FxHashSet<FileId> {
    let entries: FxHashSet<FileId> = entry_names.iter().map(|(file, _)| *file).collect();
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

// The fixtures the test modules request, and those the tests and fixtures of any other file request.
fn fixture_requests(files: &[(FileId, &FileFacts)], test_modules: &FxHashSet<FileId>) -> FxHashSet<String> {
    let mut requests = every_in(files, |id| test_modules.contains(&id), |facts| &facts.fixture_requests);
    requests.extend(every(files, |facts| &facts.pytest_requests));
    requests
}

// The attributes the files read under names the program computes.
fn computed_reads(files: &[(FileId, &FileFacts)]) -> ComputedReads {
    let mut reads = ComputedReads::default();
    for (_, facts) in files {
        reads.extend(&facts.constructors.computed_reads);
    }
    reads
}

// The names the files `selected` accepts list in one of their facts.
fn every_in(
    files: &[(FileId, &FileFacts)],
    selected: impl Fn(FileId) -> bool,
    names: impl Fn(&FileFacts) -> &Vec<String>,
) -> FxHashSet<String> {
    files.iter().filter(|(id, _)| selected(*id)).flat_map(|(_, facts)| names(facts).iter().cloned()).collect()
}

// The files whose own reports rewrite their calls, the test modules and the roles, each keyed by file id.
fn interned_inputs(
    resolver: &mut ImportResolver,
    inputs: &IndexInputs,
) -> (FxHashSet<FileId>, FxHashSet<FileId>, FxHashMap<FileId, ModuleRole>) {
    let rewrites_calls = inputs.rewrites_calls.iter().map(|path| resolver.intern(path)).collect();
    let test_modules = inputs.test_modules.iter().map(|path| resolver.intern(path)).collect();
    let roles = inputs.roles.iter().map(|(path, role)| (resolver.intern(path), *role)).collect();
    (rewrites_calls, test_modules, roles)
}

// An entry point runs without any import, so its name counts as used by code outside the tests.
fn entry_uses(
    entry_names: &[(FileId, String)],
    imported: &mut FxHashMap<FileId, FxHashSet<String>>,
    from_others: &mut FxHashMap<FileId, FxHashSet<String>>,
) {
    for (file, name) in entry_names {
        imported.entry(*file).or_default().insert(name.clone());
        from_others.entry(*file).or_default().insert(name.clone());
    }
}

// The entry points' names, with each file interned, so the index keys them by file id.
fn interned_entry_names(resolver: &mut ImportResolver, entry_points: &[String]) -> Vec<(FileId, String)> {
    entry_point_names(resolver, entry_points).into_iter().map(|(file, name)| (resolver.intern(&file), name)).collect()
}

// The file and top-level name each `module:attribute` entry point of the project names.
fn entry_point_names(resolver: &ImportResolver, entry_points: &[String]) -> Vec<(PathBuf, String)> {
    let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, resolver.paths.clone());
    entry_points
        .iter()
        .filter_map(|entry_point| entry_point.split_once(':'))
        .filter_map(|(module, attribute)| {
            let file = resolve(&resolver.db, environment, None, module.trim())?;
            Some((file, attribute.split('.').next().unwrap_or_default().trim().to_owned()))
        })
        .collect()
}

// The calls of every function and dataclass of the run, with the names reached in ways simplint cannot follow marked.
fn run_calls(
    files: &[(FileId, &FileFacts)],
    sources: &FxHashMap<FileId, &FileFacts>,
    constructor_uses: &[FileUses],
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    module_reads: &FxHashSet<String>,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
) -> RunCalls {
    let declared = files.iter().map(|(id, facts)| (*id, facts.constructors.dataclasses.as_slice()));
    let dynamic: FxHashSet<&str> =
        files.iter().flat_map(|(_, facts)| facts.constructors.dynamic.iter().map(String::as_str)).collect();
    let defined: FxHashMap<FileId, FxHashSet<&str>> =
        sources.iter().map(|(file, facts)| (*file, facts.defined.iter().map(String::as_str).collect())).collect();
    let hidden: FxHashSet<&str> = dynamic.iter().copied().chain(module_reads.iter().map(String::as_str)).collect();
    let mut calls = RunCalls::index(declared, constructor_uses, imported, &dynamic, &defined, &hidden);
    hide_names(&mut calls, entry_names, referenced);
    method_loads(&mut calls, files);
    calls
}

// The functions a file's entry points and dotted strings name are reached by calls simplint cannot follow.
fn hide_names(
    calls: &mut RunCalls,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
) {
    for (file, name) in entry_names {
        calls.mark_opaque(file, name);
    }
    for (file, names) in referenced {
        for name in names {
            calls.mark_opaque(*file, name.clone());
        }
    }
}

// How many times the run loads each method, class attribute and module-level name of each file as `obj.name`, so a
// member's calls can be checked against every read.
fn method_loads(calls: &mut RunCalls, files: &[(FileId, &FileFacts)]) {
    let mut loads: FxHashMap<&str, u32> = FxHashMap::default();
    for (_, facts) in files {
        for (name, count) in &facts.constructors.attribute_loads {
            *loads.entry(name.as_str()).or_default() += count;
        }
    }
    for (id, facts) in files {
        let constructors = &facts.constructors;
        let members = constructors.methods.iter().chain(&constructors.attributes).chain(&constructors.definitions);
        let method_loads =
            members.map(|member| (member.clone(), loads.get(member.as_str()).copied().unwrap_or_default())).collect();
        calls.set_method_loads(*id, method_loads);
    }
}

// The names each file's test modules use that no other file uses, so only tests use them.
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

// What the files of the run read in common: whether any reads docstrings or a class's name off an object, so the names
// of the run's code may be read, and the number of files each word of a string literal appears in, which a rename must
// not change.
#[derive(Default, PartialEq)]
pub(super) struct RunReads {
    pub(super) docstrings: bool,
    pub(super) class_names: bool,
    // The names whose `__name__` or `__qualname__` some file of the run reads.
    pub(super) reflected: FxHashSet<String>,
    pub(super) string_words: FxHashMap<String, u32>,
}

impl RunReads {
    fn collect(indexed: &[(FileId, &FileFacts)]) -> Self {
        Self {
            docstrings: indexed.iter().any(|(_, facts)| facts.reads_docstrings),
            class_names: indexed.iter().any(|(_, facts)| facts.reads_class_names),
            reflected: indexed.iter().flat_map(|(_, facts)| facts.reflected_names.iter().cloned()).collect(),
            string_words: string_word_counts(indexed),
        }
    }
}

// The number of files each word appears in. The files are counted in parallel by borrowed word, so each distinct word
// is copied once rather than once per file that holds it.
fn string_word_counts(indexed: &[(FileId, &FileFacts)]) -> FxHashMap<String, u32> {
    let counts = indexed
        .par_iter()
        .fold(FxHashMap::default, |mut counts: FxHashMap<&str, u32>, (_, facts)| {
            for word in facts.string_words.lines() {
                *counts.entry(word).or_default() += 1;
            }
            counts
        })
        .reduce(FxHashMap::default, |first, second| {
            let (mut larger, smaller) = if first.len() < second.len() { (second, first) } else { (first, second) };
            for (word, count) in smaller {
                *larger.entry(word).or_default() += count;
            }
            larger
        });
    counts.into_iter().map(|(word, count)| (word.to_owned(), count)).collect()
}

// The names of the whole run that every file's answers depend on: what the run's attribute writes, module reads and base
// classes are, and whether it reads docstrings.
fn digest(
    (reads, docs): (&RunReads, &DocReferences),
    computed_reads: &ComputedReads,
    [assigned, module_reads, base_classes, fixtures, opaque]: [&FxHashSet<String>; 5],
    writes: &Writes,
) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    reads.docstrings.cache_key(&mut hasher);
    reads.class_names.cache_key(&mut hasher);
    reads.reflected.cache_key(&mut hasher);
    docs.cache_key(&mut hasher);
    computed_reads.cache_key(&mut hasher);
    string_words_key(&reads.string_words).cache_key(&mut hasher);
    // Each set is hashed behind its length, so a name moving from one set to the next changes the digest.
    for names in [
        assigned,
        module_reads,
        base_classes,
        fixtures,
        opaque,
        &writes.untyped,
        &writes.unslottable_bases,
        &writes.stateful_bases,
    ] {
        names.cache_key(&mut hasher);
    }
    writes.on_class.cache_key(&mut hasher);
    writes.pickles_low_protocol.cache_key(&mut hasher);
    hasher.finish()
}

// A digest of the word counts that does not depend on the map's order, so the many words need no sorting: each word
// and its count hash on their own, and the hashes add up.
fn string_words_key(words: &FxHashMap<String, u32>) -> (usize, u64) {
    let sum = words.par_iter().map(|entry| {
        let mut hasher = CacheKeyHasher::new();
        entry.cache_key(&mut hasher);
        hasher.finish()
    });
    (words.len(), sum.reduce(|| 0, u64::wrapping_add))
}

impl Writes {
    fn collect(files: &[(FileId, &FileFacts)]) -> Self {
        let mut writes = Self::default();
        for (_, facts) in files {
            let facts = &facts.attribute_writes;
            for (class, attribute) in &facts.typed {
                writes.on_class.entry(class.clone()).or_default().insert(attribute.clone());
            }
            writes.untyped.extend(facts.untyped.iter().cloned());
            writes.unslottable_bases.extend(facts.unslottable_bases.iter().cloned());
            writes.stateful_bases.extend(facts.stateful_bases.iter().cloned());
            writes.pickles_low_protocol |= facts.pickles_low_protocol;
        }
        let mut namesakes: FxHashMap<String, usize> = FxHashMap::default();
        for class in writes.on_class.keys().chain(&writes.unslottable_bases) {
            *namesakes.entry(unqualified(class).to_owned()).or_default() += 1;
        }
        writes.namesakes = namesakes;
        writes
    }
}

// A statement's range, keyed by its offsets; the cache key of each index value that holds one hashes it this way.
fn range_key(range: TextRange, state: &mut CacheKeyHasher) {
    u32::from(range.start()).cache_key(state);
    u32::from(range.end()).cache_key(state);
}
