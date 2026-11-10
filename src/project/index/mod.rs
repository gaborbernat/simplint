mod api;
mod cycles;
mod duplicates;
mod exceptions;
mod hatch;
mod inherit;
mod reach;
mod resolve;
mod run_imports;
mod startup;

pub use api::{Exposure, FileIndex, ModuleUse};
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
use crate::project::docs::{Module, Stat, stat};
use crate::project::facts::ImportBinding;
use crate::project::receivers::module_parts;
use crate::project::{DocReferences, FileFacts};
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

use self::api::module_uses;
use self::cycles::{Cycle, RuntimeGraph};
use self::duplicates::DuplicateRun;
use self::exceptions::exception_classes;
use self::hatch::hatch_hook_classes;
use self::reach::{StarTargets, library_api, reach_by_last_name, reexport, signature_reached, unqualified};
use self::resolve::{Unreached, normalized, resolve};
use self::run_imports::{RunImports, run_import_kinds};

// Where imports resolve, as `ruff analyze graph` searches: the configured `src` directories first, then the directory
// above each package.
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
    // What the resolver sees beyond the run's own files, which a cached run that skips it must not miss. It only asks
    // which files and directories exist, under each root and under each directory between an importing file and its
    // root, which ty searches too for a name the roots do not hold. A directory's time changes whenever an entry in it
    // is added, removed or renamed. Below those it walks only directories an import can name, `name-stubs` providing
    // `name`, and follows links as the resolver does, each directory once.
    pub fn layout_key<'a>(&self, files: impl Iterator<Item = &'a Path>, hasher: &mut CacheKeyHasher) {
        let mut level: Vec<PathBuf> = self.roots.clone();
        level.extend(files.filter_map(Path::parent).flat_map(|directory| {
            let root = self.roots.iter().find(|root| directory.starts_with(root));
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
        let stats: Vec<Option<Stat>> = walked.par_iter().map(|directory| stat(directory)).collect();
        for (directory, stat) in walked.iter().zip(stats) {
            directory.cache_key(hasher);
            stat.cache_key(hasher);
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

    // The dotted names the `.pth` files of the roots make the interpreter read at startup.
    pub fn startup_names(&self) -> Vec<String> {
        startup::names(&self.roots)
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

// The directories in `directory` an import can name, `name-stubs` providing `name`, each with the target it resolves
// to when it is a link to a directory, which the resolver follows.
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
    // The names the entry points of the run name that no file imports, which the host loads rather than a module.
    pub(super) entry_only: FxHashMap<FileId, FxHashSet<String>>,
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
    // The public API of each file: the names its public module exports, and those it re-exports, each with the file that
    // re-exports it, if any.
    pub(super) library_api: FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    // The names a file uses from other files only when they are test modules, which no other file uses.
    pub(super) test_only: FxHashMap<FileId, FxHashSet<String>>,
    // The exception classes each file defines that no handler of the run names by name, or through a subclass.
    pub(super) uncaught_exceptions: FxHashMap<FileId, FxHashSet<String>>,
    // The names each file gives to an exception class of the run, defined there or imported from there.
    pub(super) exception_types: FxHashMap<FileId, FxHashSet<String>>,
    // The classes each file defines that subclass a hook interface hatchling loads a custom hook's class from.
    pub(super) hook_classes: FxHashMap<FileId, FxHashSet<String>>,
    // Whether each name a file imports from a file of the run is a class there, by the name it binds.
    pub(super) run_imports: FxHashMap<FileId, FxHashMap<String, bool>>,
    // The members of each module a file binds by importing a file of the run as a module, by the name it binds.
    pub(super) run_modules: FxHashMap<FileId, FxHashMap<String, FxHashMap<String, bool>>>,
    // The classes and aliases each file defines that another exported name's signature reaches.
    pub(super) api_reached: FxHashMap<FileId, FxHashSet<String>>,
    pub(super) library_reached: FxHashMap<FileId, FxHashSet<String>>,
    // The members of each file's private classes that a public class of the run inherits unoverridden, by class.
    pub(super) inherited: FxHashMap<FileId, FxHashMap<String, FxHashSet<String>>>,
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
    // The statements each file shares with another file, which `duplicate-code` reports.
    pub(super) duplicates: FxHashMap<FileId, Vec<DuplicateRun>>,
    // The digest of each file's source the index read, whose offsets the ranges of `cycles` and `duplicates` are.
    pub(super) sources: FxHashMap<FileId, u64>,
    // The modules each file binds at its top level, keyed by that file.
    pub(super) module_uses: FxHashMap<FileId, Vec<ModuleUse>>,
    // The modules a write under a computed name reaches, whose names any of them may be set by it.
    pub(super) computed_writes: FxHashSet<FileId>,
    // The names a `setattr` call of each file sets on that file's module, which only the file's own names count.
    pub(super) written: FxHashMap<FileId, FxHashSet<String>>,
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
    pub(super) dict_stores: FxHashSet<String>,
    pub(super) unslottable_bases: FxHashSet<String>,
    pub(super) stateful_bases: FxHashSet<String>,
    pub(super) pickles_low_protocol: bool,
    // How many entries of `on_class` and `unslottable_bases` each unqualified class name has.
    pub(super) namesakes: FxHashMap<String, usize>,
}

// What the index reads besides the files' facts.
pub struct IndexInputs<'a> {
    // Each entry point with the directory its `pyproject.toml` is in, which the entry point's module resolves from.
    pub entry_points: &'a [(PathBuf, String)],
    // The build backend's hooks, each with the file that defines it, which a build frontend calls by name.
    pub build_hooks: &'a [(PathBuf, String)],
    // The coverage plugin modules, each with the directory its `pyproject.toml` is in, which coverage calls by name.
    pub coverage_plugins: &'a [(PathBuf, String)],
    // The dotted names the `.pth` files of the run's roots make the interpreter load and read at startup.
    pub startup: &'a [String],
    pub roles: &'a FxHashMap<&'a Path, ModuleRole>,
    pub test_modules: &'a FxHashSet<&'a Path>,
    pub distribution_roots: &'a FxHashMap<&'a Path, &'a Path>,
    // The Python objects the project's documentation names.
    pub docs: &'a DocReferences,
    // The fewest statements a duplicate block of any file of the run spans, when a file checks for duplicate code.
    pub duplicate_statements: Option<usize>,
    // What another project of the run reads, which counts as use in these files as an entry point's names do.
    pub external: &'a External,
}

// What the files of one project read that another project's files may define: the names it imports, the names it reads
// and stores to, the names its doctests use, and the attributes it reads under computed names.
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
        let entries: FxHashSet<FileId> = entry_names.iter().map(|(file, _)| *file).chain(loaded).collect();
        let test_support = test_support(&importers(&indexed, resolver), &test_modules, &entries);
        let Gathered {
            mut imported,
            mut referenced,
            mut stored_references,
            mut from_tests,
            mut from_others,
            stars,
            unresolved_stars,
            unreached_imports,
            unreached_references,
            unreached_doctests,
            constructor_uses,
            bindings,
            module_names,
            runtime,
            mut opaque_names,
            computed_writes,
            written,
        } = Gathered::of(&indexed, resolver, &test_modules, &test_support);
        for (file, name) in plugin_names {
            referenced.entry(file).or_default().insert(name.clone());
            from_others.entry(file).or_default().insert(name);
        }
        let sources = non_stub_facts(&ids, files);
        let ids_of = &*resolver.files;
        // The cycle and repeated statement reports and what the run reads take nothing from the reach and re-export
        // steps, so they are worked out alongside them.
        let (aside, ()) = rayon::join(
            || {
                let (cycles, duplicates) = rayon::join(
                    || runtime_cycles(runtime, &test_modules, ids_of),
                    || duplicate_reports(&sources, &test_modules, ids_of, duplicate_statements, distribution_roots),
                );
                (cycles, duplicates, RunReads::collect(&indexed))
            },
            || {
                // A dotted path that may reach any module of a package could come from any file, so it counts as used
                // outside the tests as well.
                reach_by_last_name(&mut imported, &unreached_imports, ids_of, sources.keys().copied());
                reach_by_last_name(&mut from_others, &unreached_imports, ids_of, sources.keys().copied());
                reach_by_last_name(&mut referenced, &unreached_references, ids_of, sources.keys().copied());
                reach_by_last_name(&mut stored_references, &unreached_references, ids_of, sources.keys().copied());
                reach_by_last_name(&mut from_others, &unreached_references, ids_of, sources.keys().copied());
                reach_by_last_name(&mut referenced, &unreached_doctests, ids_of, sources.keys().copied());
                reach_by_last_name(&mut from_others, &unreached_doctests, ids_of, sources.keys().copied());
                for names in [&mut imported, &mut referenced, &mut from_tests, &mut from_others] {
                    reexport(names, &stars, &sources);
                }
            },
        );
        let (cycle_reports, duplicate_reports, reads) = aside;
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
        } = run_names(&indexed, &test_modules, (&reads, docs), (&opaque_names, &external.assigned));
        let mut computed_reads = computed_reads;
        computed_reads.extend(&external.computed);
        external_uses(&indexed, external, &mut imported, &mut referenced, &mut stored_references, &mut from_others);
        let entry_only = entry_uses(&entry_names, &bare_names, &mut imported, &mut from_others);
        opaque_names.extend(external.opaque.iter().cloned());
        let hook_classes = hatch_hook_classes(&resolver.modules, &bindings, &stars);
        let RunImports { names: run_imports, modules: run_modules } =
            run_import_kinds(&resolver.modules, &bindings, &stars);
        let (
            ((uncaught_exceptions, exception_types), module_uses),
            (mut calls, library_api, (api_reached, library_reached)),
        ) = rayon::join(
            || {
                rayon::join(
                    || {
                        exception_classes(
                            &indexed,
                            &resolver.modules,
                            resolver.python_version,
                            &test_modules,
                            &bindings,
                            &stars,
                        )
                    },
                    || module_uses(module_names, &sources, &stored_references, &computed_writes, ids_of),
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
                            &module_reads,
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
        let inherited = inherit::inherited_api(&resolver.modules, &library_api, &library_reached, &bindings, &stars);
        calls.set_test_support(test_support);
        Self {
            imported,
            entry_only,
            referenced,
            star_imported,
            package_star_imported,
            star_sources,
            roles,
            library_api,
            test_only: only_tests(from_tests, &from_others),
            uncaught_exceptions,
            exception_types,
            hook_classes,
            run_imports,
            run_modules,
            api_reached,
            library_reached,
            inherited,
            assigned_attributes,
            module_reads,
            base_classes,
            fixture_requests,
            computed_reads,
            opaque_names,
            docs: docs.clone(),
            cycles: cycle_reports,
            duplicates: duplicate_reports,
            sources: indexed.iter().map(|(id, facts)| (*id, facts.source_digest)).collect(),
            module_uses,
            computed_writes,
            written,
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
            .chain(writes.dict_stores.symmetric_difference(&others.dict_stores))
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
            entry_only: entry(&self.entry_only, id),
            referenced: entry(&self.referenced, id),
            star_imported: id.is_some_and(|id| self.star_imported.contains(&id)),
            package_star_imported: id.is_some_and(|id| self.package_star_imported.contains(&id)),
            star_source: id.is_some_and(|id| self.star_sources.contains(&id)),
            id,
            role: entry(&self.roles, id).copied().unwrap_or_default(),
            library_api: entry(&self.library_api, id),
            modules: entry(&self.module_uses, id).map_or(&[], Vec::as_slice),
            test_only: entry(&self.test_only, id),
            uncaught_exceptions: entry(&self.uncaught_exceptions, id),
            exception_types: entry(&self.exception_types, id),
            hook_classes: entry(&self.hook_classes, id),
            run_imports: entry(&self.run_imports, id),
            run_modules: entry(&self.run_modules, id),
            api_reached: entry(&self.api_reached, id),
            library_reached: entry(&self.library_reached, id),
            inherited: entry(&self.inherited, id),
            cycle: entry(&self.cycles, id),
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
    // The references that patch a module, such as stores, dotted strings and `setattr` names. Doctests' reads are in
    // `referenced` but not here, as reading a name patches nothing.
    stored_references: FxHashMap<FileId, FxHashSet<String>>,
    // The names the test modules use and the names the other modules use, so a name only tests use is found.
    from_tests: FxHashMap<FileId, FxHashSet<String>>,
    from_others: FxHashMap<FileId, FxHashSet<String>>,
    stars: Vec<(FileId, FileId)>,
    opaque_names: FxHashSet<String>,
    unresolved_stars: FxHashSet<&'r str>,
    unreached_imports: Vec<Unreached<'r>>,
    unreached_references: Vec<Unreached<'r>>,
    unreached_doctests: Vec<Unreached<'r>>,
    constructor_uses: Vec<FileUses<'r>>,
    bindings: Vec<(FileId, FileId, &'r ImportBinding)>,
    module_names: Vec<(FileId, String, String, FileId)>,
    runtime: RuntimeGraph,
    computed_writes: FxHashSet<FileId>,
    written: FxHashMap<FileId, FxHashSet<String>>,
}

impl<'r> Gathered<'r> {
    fn of(
        indexed: &[(FileId, &FileFacts)],
        resolver: &'r ImportResolver,
        test_modules: &FxHashSet<FileId>,
        test_support: &FxHashSet<FileId>,
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
            let production = !test_modules.contains(&id) && !test_support.contains(&id);
            gathered.constructor_uses.push(FileUses {
                caller: id,
                path: resolver.files.path(id).to_path_buf(),
                digest: facts.source_digest,
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
                gathered.stored_references.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in &module.doctest_references {
                gathered.referenced.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            gathered.stars.extend(module.stars.iter().map(|source| (id, *source)));
            gathered.opaque_names.extend(module.unresolved_names.iter().cloned());
            gathered.unresolved_stars.extend(module.unresolved_stars.iter().map(String::as_str));
            gathered.unreached_imports.extend(module.unreached_imports.iter().map(borrowed_unreached));
            gathered.unreached_references.extend(module.unreached_references.iter().map(borrowed_unreached));
            gathered.unreached_doctests.extend(module.unreached_doctests.iter().map(borrowed_unreached));
        }
        gathered
    }
}

fn borrowed_unreached(entry: &(PathBuf, String, Vec<String>)) -> Unreached<'_> {
    (entry.0.as_path(), entry.1.as_str(), entry.2.as_slice())
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
    (opaque_names, outer_assigned): (&FxHashSet<String>, &FxHashSet<String>),
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
    // A store to the name in another project still makes it assigned here, as a store in this one does.
    let assigned_attributes: FxHashSet<String> =
        assigned_attributes.into_iter().chain(outer_assigned.iter().cloned()).collect();
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
// calls are test code though its path does not say so. An entry point or a module a `.pth` file loads runs without an
// import, so it is never one.
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

// The test modules and the roles, each keyed by file id.
fn interned_inputs(
    resolver: &mut ImportResolver,
    inputs: &IndexInputs,
) -> (FxHashSet<FileId>, FxHashMap<FileId, ModuleRole>) {
    let test_modules = inputs.test_modules.iter().map(|path| resolver.intern(path)).collect();
    let roles = inputs.roles.iter().map(|(path, role)| (resolver.intern(path), *role)).collect();
    (test_modules, roles)
}

// An entry point runs without any import, so its name counts as used by code outside the tests.
// The names the members of each module `automodule` documents with `:members:` add to the references, as Sphinx renders
// them: the module's `__all__` if it has one, else its public names. The directive's `:exclude-members:` leaves names out,
// and its `:private-members:` and `:special-members:` take in the underscore and dunder names.
fn documented_members(
    docs: &DocReferences,
    indexed: &[(FileId, &FileFacts)],
    resolver: &ImportResolver,
) -> DocReferences {
    let mut expanded = docs.clone();
    let named: Vec<(&FileFacts, Vec<String>)> = indexed
        .iter()
        .map(|(file, facts)| {
            let dotted = resolver
                .search
                .module_paths(resolver.files.path(*file))
                .iter()
                .map(|parts| module_parts(parts).join("."))
                .collect();
            (*facts, dotted)
        })
        .collect();
    for module in docs.modules() {
        for (facts, dotted_names) in &named {
            for dotted in dotted_names.iter().filter(|dotted| {
                dotted.as_str() == module.target
                    || dotted.strip_suffix(module.target.as_str()).is_some_and(|rest| rest.ends_with('.'))
            }) {
                let members: Vec<&str> = match &facts.exports {
                    Some(exports) => exports.iter().map(String::as_str).collect(),
                    None => facts.defined.iter().map(String::as_str).filter(|name| is_rendered(name, module)).collect(),
                };
                let members: Vec<&str> = members
                    .into_iter()
                    .filter(|name| !module.exclude.iter().any(|excluded| excluded == name))
                    .collect();
                // A class the directive renders documents its members too, as the class's own name does.
                for class in facts.classes.iter().filter(|class| members.contains(&class.name.as_str())) {
                    expanded.add_scope(format!("{dotted}.{}.", class.name));
                }
                expanded.add_names(members.into_iter().map(|name| format!("{dotted}.{name}")));
            }
        }
    }
    expanded
}

// Whether Sphinx renders a module's name with `:members:`: its public names, and with the directive's flags its
// underscore names and its special names.
fn is_rendered(name: &str, module: &Module) -> bool {
    let special = name.len() > 4 && name.starts_with("__") && name.ends_with("__");
    if special {
        module.special
    } else if name.starts_with('_') {
        module.private
    } else {
        true
    }
}

// The names another project reads that a file of this one defines, which count as used in it as an entry point's names
// do. A name the file only imports is no definition of its own, so another project reading it is no use of the file.
fn external_uses(
    indexed: &[(FileId, &FileFacts)],
    external: &External,
    imported: &mut FxHashMap<FileId, FxHashSet<String>>,
    referenced: &mut FxHashMap<FileId, FxHashSet<String>>,
    stored_references: &mut FxHashMap<FileId, FxHashSet<String>>,
    from_others: &mut FxHashMap<FileId, FxHashSet<String>>,
) {
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
                imported.entry(*file).or_default().insert(name.clone());
            }
            if external.read.contains(name.as_str()) {
                referenced.entry(*file).or_default().insert(name.clone());
                stored_references.entry(*file).or_default().insert(name.clone());
                from_others.entry(*file).or_default().insert(name.clone());
            }
        }
    }
}

// An entry point runs without any import, so its name counts as used by code outside the tests. The public names a bare
// module entry point loads are not imports either, so they are returned apart from the names other modules import.
fn entry_uses(
    entry_names: &[(FileId, String)],
    bare_names: &[(FileId, String)],
    imported: &mut FxHashMap<FileId, FxHashSet<String>>,
    from_others: &mut FxHashMap<FileId, FxHashSet<String>>,
) -> FxHashMap<FileId, FxHashSet<String>> {
    let mut entry_only: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in bare_names {
        if !imported.get(file).is_some_and(|names| names.contains(name)) {
            entry_only.entry(*file).or_default().insert(name.clone());
        }
    }
    for (file, name) in entry_names {
        imported.entry(*file).or_default().insert(name.clone());
        from_others.entry(*file).or_default().insert(name.clone());
    }
    entry_only
}

type FileNames = Vec<(FileId, String)>;

// The entry points' and build hooks' names, with each file interned, so the index keys them by file id, and the names
// the bare module entry points load among them. A bare module entry point, which the host loads whole and reads by
// convention, names every public name it defines or imports, such as the hooks and fixtures of a pytest plugin, so none
// of them is internal.
fn interned_entry_names(
    resolver: &mut ImportResolver,
    entry_points: &[(PathBuf, String)],
    build_hooks: &[(PathBuf, String)],
    indexed: &[(FileId, &FileFacts)],
) -> (FileNames, FileNames) {
    let facts: FxHashMap<FileId, &FileFacts> = indexed.iter().copied().collect();
    let hooks: FileNames = build_hooks.iter().map(|(file, name)| (resolver.intern(file), name.clone())).collect();
    let mut names = Vec::new();
    let mut bare = Vec::new();
    for (file, attribute) in entry_point_targets(resolver, entry_points) {
        let id = resolver.intern(&file);
        match attribute {
            Some(attribute) => names.push((id, attribute)),
            None => bare.extend(
                facts.get(&id).map_or_else(Vec::new, |facts| public_names(facts)).into_iter().map(|name| (id, name)),
            ),
        }
    }
    names.extend(bare.iter().cloned());
    names.extend(hooks);
    (names, bare)
}

// The modules the interpreter loads at startup through the `.pth` names, which no import reaches, with each attribute
// one of those names reads from its module. The longest prefix of a name that is a module is the one loaded, and the
// part after it is the attribute read.
fn startup_entries(resolver: &mut ImportResolver, startup: &[String]) -> (FxHashSet<FileId>, Vec<(FileId, String)>) {
    let mut loaded = FxHashSet::default();
    let mut reads = Vec::new();
    for (file, attribute) in startup_targets(resolver, startup) {
        let id = resolver.intern(&file);
        loaded.insert(id);
        reads.extend(attribute.map(|attribute| (id, attribute)));
    }
    (loaded, reads)
}

// The file each `.pth` name resolves to, with the attribute it reads from that file, if any.
fn startup_targets(resolver: &ImportResolver, startup: &[String]) -> Vec<(PathBuf, Option<String>)> {
    let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, resolver.paths.clone());
    startup
        .iter()
        .filter_map(|name| {
            let parts: Vec<&str> = name.split('.').collect();
            (1..=parts.len()).rev().find_map(|length| {
                let file = resolve(&resolver.db, environment, None, &parts[..length].join("."))?;
                Some((file, parts.get(length).map(|attribute| (*attribute).to_owned())))
            })
        })
        .collect()
}

// The `coverage_init` of each coverage plugin module that resolves to a file of the run. Coverage loads the module by
// its dotted name and calls `coverage_init` on it, so no import reaches the function, and the name is read as a string.
fn coverage_plugin_names(resolver: &mut ImportResolver, plugins: &[(PathBuf, String)]) -> Vec<(FileId, String)> {
    let files: Vec<PathBuf> = plugins
        .iter()
        .filter_map(|(root, module)| {
            let paths = resolver.coverage_paths.get(root)?;
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            resolve(&resolver.db, environment, None, module)
        })
        .collect();
    files.iter().map(|file| (resolver.intern(file), "coverage_init".to_owned())).collect()
}

// The file each entry point of the project names, with the top-level attribute of a `module:attribute` entry point.
// Each resolves with its project's directory on the search path, since a flat layout's modules sit there.
fn entry_point_targets(
    resolver: &ImportResolver,
    entry_points: &[(PathBuf, String)],
) -> Vec<(PathBuf, Option<String>)> {
    entry_points
        .iter()
        .filter_map(|(root, entry_point)| {
            let (module, attribute) = match entry_point.split_once(':') {
                Some((module, attribute)) => {
                    (module, Some(attribute.split('.').next().unwrap_or_default().trim().to_owned()))
                }
                None => (entry_point.as_str(), None),
            };
            let paths = resolver.entry_paths.get(root)?;
            let environment = ResolverEnvironment::new(&resolver.db, resolver.python_version, paths.clone());
            Some((resolve(&resolver.db, environment, None, module.trim())?, attribute))
        })
        .collect()
}

// The names a module defines or imports at its top level that are not private, which its users may reach.
fn public_names(facts: &FileFacts) -> Vec<String> {
    facts
        .defined
        .iter()
        .map(String::as_str)
        .chain(facts.bindings.iter().map(|binding| binding.bound.as_str()))
        .filter(|name| Visibility::of(name) == Visibility::Public)
        .map(str::to_owned)
        .collect()
}

// The calls of every function and dataclass of the run, with the names reached in ways simplint cannot follow marked.
#[expect(clippy::too_many_arguments, reason = "the run's inputs, which the index passes on")]
fn run_calls(
    files: &[(FileId, &FileFacts)],
    sources: &FxHashMap<FileId, &FileFacts>,
    constructor_uses: &[FileUses],
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    module_reads: &FxHashSet<String>,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
    library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
) -> RunCalls {
    let declared = files.iter().map(|(id, facts)| (*id, facts.constructors.dataclasses.as_slice()));
    let dynamic: FxHashSet<&str> =
        files.iter().flat_map(|(_, facts)| facts.constructors.dynamic.iter().map(String::as_str)).collect();
    let defined: FxHashMap<FileId, FxHashSet<&str>> =
        sources.iter().map(|(file, facts)| (*file, facts.defined.iter().map(String::as_str).collect())).collect();
    let hidden: FxHashSet<&str> = dynamic.iter().copied().chain(module_reads.iter().map(String::as_str)).collect();
    let mut calls = RunCalls::index(declared, constructor_uses, imported, &dynamic, &defined, &hidden, library_api);
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
        &writes.dict_stores,
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
            writes.dict_stores.extend(facts.dict_stores.iter().cloned());
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
