mod api;
mod exceptions;
mod reach;
mod resolve;

pub use api::{Exposure, FileIndex, ModuleUse};
pub use resolve::ImportResolver;

use rustc_hash::{FxHashMap, FxHashSet};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_graph::ResolverEnvironment;
use ruff_linter::package::PackageRoot;
use ruff_python_ast::PythonVersion;

use crate::project::FileFacts;
use crate::project::constructors::{DataclassCalls, FileUses};
use crate::project::facts::ImportBinding;
use crate::settings::ModuleRole;

use self::api::module_uses;
use self::exceptions::uncaught_exceptions;
use self::reach::{StarTargets, api_reached, reach_by_last_name, reexport, unqualified, user_facing};
use self::resolve::{Unreached, normalized, resolve};

// Where imports resolve, as `ruff analyze graph` searches: the configured `src` directories first, then the directory
// above each package.
pub struct SearchRoots {
    pub(super) roots: Vec<PathBuf>,
    pub(super) python_version: PythonVersion,
}

impl SearchRoots {
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
    // A path that was never interned has no entries in the index.
    fn get(&self, path: &Path) -> Option<FileId> {
        self.ids.get(&normalized(path)).copied()
    }

    fn path(&self, id: FileId) -> &Path {
        &self.paths[id.0 as usize]
    }

    // Copy on write, so a project that still holds the table keeps its ids; numbering only grows when a path is new.
    fn intern(files: &mut Arc<Self>, path: &Path) -> FileId {
        let path = normalized(path);
        if let Some(id) = files.ids.get(&path) {
            return *id;
        }
        let files = Arc::make_mut(files);
        let id = FileId(u32::try_from(files.paths.len()).expect("the run has fewer files than a FileId counts"));
        files.ids.insert(path.clone(), id);
        files.paths.push(path);
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
    // The modules each file binds at its top level, keyed by that file.
    pub(super) module_uses: FxHashMap<FileId, Vec<ModuleUse>>,
    pub(super) writes: Writes,
    pub(super) calls: DataclassCalls,
    // Whether any file of the run reads docstrings off objects, so any docstring may be data it reads.
    pub(super) reads_docstrings: bool,
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

// What the index reads besides the files' facts: the entry points, the files whose own `dataclass-kw-only` reports
// rewrite their positional dataclass calls, and where each file sits in its library's public API.
pub struct IndexInputs<'a> {
    pub entry_points: &'a [String],
    pub rewrites_calls: &'a FxHashSet<&'a Path>,
    pub roles: &'a FxHashMap<&'a Path, ModuleRole>,
    pub test_modules: &'a FxHashSet<&'a Path>,
}

impl Project {
    pub fn index(files: &[(&Path, &FileFacts)], resolver: &mut ImportResolver, inputs: &IndexInputs) -> Self {
        let IndexInputs { entry_points, rewrites_calls, roles, test_modules } = *inputs;
        let ids = resolver.resolve(files);
        let rewrites_calls: FxHashSet<FileId> = rewrites_calls.iter().map(|path| resolver.intern(path)).collect();
        let test_modules: FxHashSet<FileId> = test_modules.iter().map(|path| resolver.intern(path)).collect();
        let roles: FxHashMap<FileId, ModuleRole> =
            roles.iter().map(|(path, role)| (resolver.intern(path), *role)).collect();
        let entry_names: Vec<(FileId, String)> = entry_point_names(resolver, entry_points)
            .into_iter()
            .map(|(file, name)| (resolver.intern(&file), name))
            .collect();
        let indexed: Vec<(FileId, &FileFacts)> = ids.iter().zip(files).map(|(id, (_, facts))| (*id, *facts)).collect();
        let modules = indexed.iter().map(|(id, _)| (*id, &resolver.modules[id]));
        let mut imported: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
        let mut referenced: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
        // The same names split by whether the file using them is a test module, so a name only tests use is found.
        let mut from_tests: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
        let mut from_others: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
        let mut stars: Vec<(FileId, FileId)> = Vec::new();
        let (mut unresolved_stars, mut unreached): (FxHashSet<&str>, Vec<Unreached>) =
            (FxHashSet::default(), Vec::new());
        let mut constructor_uses = Vec::new();
        let mut bindings: Vec<(FileId, FileId, &ImportBinding)> = Vec::new();
        let mut module_names: Vec<(FileId, String, String, FileId)> = Vec::new();
        for (id, module) in modules {
            module_names
                .extend(module.modules.iter().map(|(bound, name, target)| (id, bound.clone(), name.clone(), *target)));
            let uses = module.constructors.iter().map(|(file, uses)| (*file, uses.as_slice())).collect();
            constructor_uses.push((id, rewrites_calls.contains(&id), uses));
            bindings.extend(module.bindings.iter().map(|(source, binding)| (id, *source, binding)));
            let bucket = if test_modules.contains(&id) { &mut from_tests } else { &mut from_others };
            for (file, names) in &module.imports {
                imported.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in &module.references {
                referenced.entry(*file).or_default().extend(names.iter().cloned());
                bucket.entry(*file).or_default().extend(names.iter().cloned());
            }
            stars.extend(module.stars.iter().map(|source| (id, *source)));
            unresolved_stars.extend(module.unresolved_stars.iter().map(String::as_str));
            unreached.extend(
                module
                    .unreached
                    .iter()
                    .map(|(package, name, names)| (package.as_path(), name.as_str(), names.as_slice())),
            );
        }
        let sources: FxHashMap<FileId, &FileFacts> = ids
            .iter()
            .zip(files)
            .filter(|(_, (path, _))| path.extension().is_none_or(|extension| extension != "pyi"))
            .map(|(id, (_, facts))| (*id, *facts))
            .collect();
        // A dotted path that may reach any module of a package could come from any file, so it counts as used outside
        // the tests as well.
        reach_by_last_name(&mut referenced, &unreached, &resolver.files, sources.keys().copied());
        reach_by_last_name(&mut from_others, &unreached, &resolver.files, sources.keys().copied());
        reexport(&mut imported, &stars, &sources);
        reexport(&mut referenced, &stars, &sources);
        reexport(&mut from_tests, &stars, &sources);
        reexport(&mut from_others, &stars, &sources);
        let StarTargets { star_imported, package_star_imported, star_sources } =
            StarTargets::of(&stars, &sources, &unresolved_stars, &resolver.files);
        let assigned_attributes = every(&indexed, |facts| &facts.assigned_attributes);
        let module_reads = every(&indexed, |facts| &facts.module_reads);
        let base_classes = every(&indexed, |facts| &facts.base_classes);
        let writes = Writes::collect(&indexed);
        for (file, name) in &entry_names {
            imported.entry(*file).or_default().insert(name.clone());
            from_others.entry(*file).or_default().insert(name.clone());
        }
        let uncaught_exceptions = uncaught_exceptions(&indexed, resolver, &test_modules, &bindings, &stars);
        let api_reached = api_reached(&indexed);
        let calls =
            dataclass_calls(&indexed, &sources, &constructor_uses, &imported, &module_reads, entry_names, &referenced);
        let user_facing = user_facing(&sources, &roles, &imported, &bindings, &stars, &resolver.files);
        let module_uses = module_uses(module_names, &sources, &referenced);
        let reads_docstrings = indexed.iter().any(|(_, facts)| facts.reads_docstrings);
        let digest = digest(reads_docstrings, [&assigned_attributes, &module_reads, &base_classes], &writes);
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
            module_uses,
            writes,
            calls,
            reads_docstrings,
            digest,
            files: Arc::clone(&resolver.files),
        }
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
            .chain(writes.untyped.symmetric_difference(&others.untyped))
            .map(String::as_str)
            .chain(classes)
            .chain(pickling)
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
            calls: self.calls.file(id),
        }
    }
}

// The entry of a file's id, if the file has one.
pub(super) fn entry<V>(map: &FxHashMap<FileId, V>, id: Option<FileId>) -> Option<&V> {
    map.get(&id?)
}

// The names any file of the run lists in one of its facts.
fn every(files: &[(FileId, &FileFacts)], names: impl Fn(&FileFacts) -> &Vec<String>) -> FxHashSet<String> {
    files.iter().flat_map(|(_, facts)| names(facts).iter().cloned()).collect()
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
pub(super) fn dataclass_calls(
    files: &[(FileId, &FileFacts)],
    sources: &FxHashMap<FileId, &FileFacts>,
    constructor_uses: &[FileUses],
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    module_reads: &FxHashSet<String>,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
) -> DataclassCalls {
    let declared = files.iter().map(|(id, facts)| (*id, facts.constructors.dataclasses.as_slice()));
    let dynamic: FxHashSet<&str> =
        files.iter().flat_map(|(_, facts)| facts.constructors.dynamic.iter().map(String::as_str)).collect();
    let defined: FxHashMap<FileId, FxHashSet<&str>> =
        sources.iter().map(|(file, facts)| (*file, facts.defined.iter().map(String::as_str).collect())).collect();
    let hidden: FxHashSet<&str> = dynamic.iter().copied().chain(module_reads.iter().map(String::as_str)).collect();
    let mut calls = DataclassCalls::index(declared, constructor_uses, imported, &dynamic, &defined, &hidden);
    hide_names(&mut calls, entry_names, referenced);
    method_loads(&mut calls, files);
    calls
}

// The functions a file's entry points and dotted strings name are reached by calls simplint cannot follow.
fn hide_names(
    calls: &mut DataclassCalls,
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

// How many times the run loads each method name of each file, so a method's calls can be checked against every read.
fn method_loads(calls: &mut DataclassCalls, files: &[(FileId, &FileFacts)]) {
    let mut loads: FxHashMap<&str, u32> = FxHashMap::default();
    for (_, facts) in files {
        for (name, count) in &facts.constructors.attribute_loads {
            *loads.entry(name.as_str()).or_default() += count;
        }
    }
    for (id, facts) in files {
        let method_loads = facts
            .constructors
            .methods
            .iter()
            .map(|method| (method.clone(), loads.get(method.as_str()).copied().unwrap_or_default()))
            .collect();
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

// The names of the whole run that every file's answers depend on: what the run's attribute writes, module reads and base
// classes are, and whether it reads docstrings.
pub(super) fn digest(
    reads_docstrings: bool,
    [assigned, module_reads, base_classes]: [&FxHashSet<String>; 3],
    writes: &Writes,
) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    reads_docstrings.cache_key(&mut hasher);
    // Each set is hashed behind its length, so a name moving from one set to the next changes the digest.
    for names in
        [assigned, module_reads, base_classes, &writes.untyped, &writes.unslottable_bases, &writes.stateful_bases]
    {
        names.cache_key(&mut hasher);
    }
    writes.on_class.cache_key(&mut hasher);
    writes.pickles_low_protocol.cache_key(&mut hasher);
    hasher.finish()
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
