use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_db::files::{File, system_path_to_file};
use ruff_db::system::{OsSystem, SystemPathBuf};
use ruff_graph::{ModuleDb, ResolverEnvironment, resolve_search_paths};
use ruff_linter::package::PackageRoot;
use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::builtins::is_exception;
use ruff_python_stdlib::sys::is_known_standard_library;
use ty_module_resolver::{
    ImportingFile, ModuleName, SearchPaths, resolve_module, resolve_module_confident, resolve_real_module,
    resolve_real_module_confident,
};

use super::FileFacts;
use super::constructors::{DataclassCalls, FileCalls, FileUses, NameUse};
use super::facts::{ImportBinding, ModuleNames};
use crate::settings::ModuleRole;
use crate::syntax::Visibility;

// Where imports resolve, as `ruff analyze graph` searches: the configured `src` directories first, then the directory
// above each package.
pub struct SearchRoots {
    roots: Vec<PathBuf>,
    python_version: PythonVersion,
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
    ids: FxHashMap<PathBuf, FileId>,
    paths: Vec<PathBuf>,
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
    imported: FxHashMap<FileId, FxHashSet<String>>,
    // Names other code reaches by a dotted string, such as a `mock.patch` target: renaming them breaks that code, yet
    // they are not part of the module's API.
    referenced: FxHashMap<FileId, FxHashSet<String>>,
    // Modules without a static `__all__` that an `import *` may read, so each of their public names may be in use.
    star_imported: FxHashSet<FileId>,
    // Modules their own package's `__init__.py` star-imports, where the package binds the submodule under its own name.
    package_star_imported: FxHashSet<FileId>,
    // Modules some file star-imports, whatever their `__all__`, so a name they export may be in use where it lands.
    star_sources: FxHashSet<FileId>,
    // Where each file sits in its library's public API.
    roles: FxHashMap<FileId, ModuleRole>,
    // The names of each file a library's users can reach: those its public module exports, and those a public module
    // re-exports, each with the file that re-exports it, if any.
    user_facing: FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
    // The names a file uses from other files only when they are test modules, which no other file uses.
    test_only: FxHashMap<FileId, FxHashSet<String>>,
    // The exception classes each file defines that no handler of the run names by name, or through a subclass.
    uncaught_exceptions: FxHashMap<FileId, FxHashSet<String>>,
    // The classes and aliases each file defines that another exported name's signature reaches.
    api_reached: FxHashMap<FileId, FxHashSet<String>>,
    assigned_attributes: FxHashSet<String>,
    module_reads: FxHashSet<String>,
    base_classes: FxHashSet<String>,
    // The modules each file binds at its top level, keyed by that file.
    module_uses: FxHashMap<FileId, Vec<ModuleUse>>,
    writes: Writes,
    calls: DataclassCalls,
    // Whether any file of the run reads docstrings off objects, so any docstring may be data it reads.
    reads_docstrings: bool,
    digest: u64,
    files: Arc<FileIds>,
}

// The attribute writes of the run: by the qualified class of the instance written to when parsing tells, and otherwise
// by name.
#[derive(Default)]
struct Writes {
    on_class: FxHashMap<String, FxHashSet<String>>,
    untyped: FxHashSet<String>,
    unslottable_bases: FxHashSet<String>,
    stateful_bases: FxHashSet<String>,
    pickles_low_protocol: bool,
    // How many entries of `on_class` and `unslottable_bases` each unqualified class name has.
    namesakes: FxHashMap<String, usize>,
}

// What the index reads besides the files' facts: the entry points, the files whose own `dataclass-kw-only` reports
// rewrite their positional dataclass calls, and where each file sits in its library's public API.
pub struct IndexInputs<'a> {
    pub entry_points: &'a [String],
    pub rewrites_calls: &'a FxHashSet<&'a Path>,
    pub roles: &'a FxHashMap<&'a Path, ModuleRole>,
    pub test_modules: &'a FxHashSet<&'a Path>,
}

// Where each file's module names lead, kept for the whole run so a fix round resolves only the files it rewrote. Fixes
// change what files say but never add or remove one, and resolving reads only which files and directories exist, so
// neither the database nor the modules of a file no fix touched go stale.
pub struct ImportResolver {
    db: ModuleDb,
    paths: SearchPaths,
    python_version: PythonVersion,
    // The names the files of the run can be imported as, from the components of each file's path.
    names: FxHashSet<String>,
    files: Arc<FileIds>,
    modules: FxHashMap<FileId, ModuleFacts<FileId>>,
}

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

impl ImportResolver {
    pub fn new(roots: &SearchRoots) -> Result<Self> {
        let cwd = SystemPathBuf::from_path_buf(std::env::current_dir()?)
            .map_err(|path| anyhow::anyhow!("{} is not a UTF-8 path", path.display()))?;
        let system = OsSystem::new(cwd);
        let src_roots = roots.roots.iter().filter_map(|root| SystemPathBuf::from_path_buf(root.clone()).ok()).collect();
        let paths = resolve_search_paths(&system, src_roots, None)?;
        let db = ModuleDb::new(system);
        paths.try_register_static_roots(&db);
        Ok(Self {
            db,
            paths,
            python_version: roots.python_version,
            names: FxHashSet::default(),
            files: Arc::default(),
            modules: FxHashMap::default(),
        })
    }

    // Drops what the file's module names resolved to, as a fix rewrote the file.
    pub fn forget(&mut self, path: &Path) {
        if let Some(id) = self.files.get(path) {
            self.modules.remove(&id);
        }
    }

    // Drops every file's resolved module names, for a run that indexes no more files after this.
    pub fn release(&mut self) {
        self.modules = FxHashMap::default();
    }

    // Resolves the files it has not resolved yet, and returns the id of each file.
    fn resolve(&mut self, files: &[(&Path, &FileFacts)]) -> Vec<FileId> {
        // A `name-stubs` package provides the module `name`, so its stripped name counts too.
        for (path, _) in files {
            for part in &normalized(path) {
                let name = Path::new(part).file_stem().and_then(|stem| stem.to_str());
                self.names.extend(name.map(|name| name.strip_suffix("-stubs").unwrap_or(name).to_owned()));
            }
        }
        let ids: Vec<FileId> = files.iter().map(|(path, _)| self.intern(path)).collect();
        let (python_version, paths, names) = (self.python_version, &self.paths, &self.names);
        // The database is not `Sync`, so each worker resolves with its own clone, which shares the cached results.
        let fresh: Vec<(FileId, ModuleFacts<PathBuf>)> = files
            .par_iter()
            .zip(&ids)
            .filter(|(_, id)| !self.modules.contains_key(id))
            .map_with(self.db.clone(), |db, ((path, facts), id)| {
                let environment = ResolverEnvironment::new(db, python_version, paths.clone());
                (*id, ModuleFacts::resolve(db, environment, path, facts, names))
            })
            .collect();
        for (id, facts) in fresh {
            let facts = facts.map_files(|path| FileIds::intern(&mut self.files, &path));
            self.modules.insert(id, facts);
        }
        ids
    }

    fn intern(&mut self, path: &Path) -> FileId {
        FileIds::intern(&mut self.files, path)
    }
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
fn entry<V>(map: &FxHashMap<FileId, V>, id: Option<FileId>) -> Option<&V> {
    map.get(&id?)
}

// The files `import *` reaches: those it reads every public name of, those a package's `__init__.py` star-imports, and
// every file a star import names.
struct StarTargets {
    star_imported: FxHashSet<FileId>,
    package_star_imported: FxHashSet<FileId>,
    star_sources: FxHashSet<FileId>,
}

impl StarTargets {
    fn of(
        stars: &[(FileId, FileId)],
        sources: &FxHashMap<FileId, &FileFacts>,
        unresolved: &FxHashSet<&str>,
        files: &FileIds,
    ) -> Self {
        let star_imported = stars
            .iter()
            .map(|(_, source)| *source)
            .chain(unresolved_candidates(files, sources.keys().copied(), unresolved))
            .filter(|file| sources.get(file).is_some_and(|facts| facts.exports.is_none()))
            .collect();
        let package_star_imported = stars
            .iter()
            .filter(|(importer, source)| {
                files.path(*source).parent().is_some_and(|parent| files.path(*importer) == parent.join("__init__.py"))
            })
            .map(|(_, source)| *source)
            .collect();
        let star_sources = stars.iter().map(|(_, source)| *source).collect();
        Self { star_imported, package_star_imported, star_sources }
    }
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
fn dataclass_calls(
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

// The exception classes of the run that no handler catches, by name or through a subclass, which is how a handler of a
// base class reads. A class is an exception when a base is an exception: a builtin one, a standard-library one, or an
// exception class of the run. A warning is not reported, since a warning is raised to be seen, not caught.
fn uncaught_exceptions(
    files: &[(FileId, &FileFacts)],
    resolver: &ImportResolver,
    test_modules: &FxHashSet<FileId>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
) -> FxHashMap<FileId, FxHashSet<String>> {
    let minor = resolver.python_version.minor;
    let (mut caught, mut classes): (Vec<ClassKey>, FxHashMap<ClassKey, Vec<ClassBase>>) =
        (Vec::new(), FxHashMap::default());
    for (id, _) in files {
        let module = &resolver.modules[id];
        if !test_modules.contains(id) {
            caught.extend(module.caught.iter().map(|(file, name)| (*file, name.clone())));
        }
        classes.extend(module.classes.iter().map(|(name, bases)| ((*id, name.clone()), bases.clone())));
    }
    let mut reexports: FxHashMap<FileId, Vec<(&str, FileId, &str)>> = FxHashMap::default();
    for (importer, source, binding) in bindings {
        reexports.entry(*importer).or_default().push((&binding.bound, *source, &binding.name));
    }
    let mut starred: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in stars {
        starred.entry(*importer).or_default().push(*source);
    }
    // The class a name of a file is, following the imports that re-export it from the file that defines it.
    let home = |file: FileId, name: &str| -> Option<ClassKey> {
        let mut pending: Vec<(FileId, &str)> = vec![(file, name)];
        let mut seen: FxHashSet<(FileId, &str)> = FxHashSet::default();
        while let Some((file, name)) = pending.pop() {
            if !seen.insert((file, name)) {
                continue;
            }
            let key = (file, name.to_owned());
            if classes.contains_key(&key) {
                return Some(key);
            }
            for (bound, source, original) in reexports.get(&file).into_iter().flatten() {
                if *bound == name {
                    pending.push((*source, original));
                }
            }
            pending.extend(starred.get(&file).into_iter().flatten().map(|source| (*source, name)));
        }
        None
    };
    let mut parents: FxHashMap<&ClassKey, Vec<ClassKey>> = FxHashMap::default();
    let mut outside: FxHashMap<&ClassKey, Vec<(&str, &str)>> = FxHashMap::default();
    for (key, bases) in &classes {
        for (file, module, name) in bases {
            match file.and_then(|file| home(file, name)) {
                Some(parent) => parents.entry(key).or_default().push(parent),
                None => outside.entry(key).or_default().push((module, name)),
            }
        }
    }
    let (mut exceptions, mut warnings): (FxHashSet<&ClassKey>, FxHashSet<&ClassKey>) =
        (FxHashSet::default(), FxHashSet::default());
    loop {
        let mut grew = false;
        for key in classes.keys() {
            let bases = outside.get(key).map_or(&[][..], Vec::as_slice);
            let from_parents = |set: &FxHashSet<&ClassKey>| {
                parents.get(key).is_some_and(|found| found.iter().any(|parent| set.contains(parent)))
            };
            let raised = bases.iter().any(|(module, name)| is_exception_name(module, name, minor));
            let warns =
                bases.iter().any(|(module, name)| name.ends_with("Warning") && is_exception_name(module, name, minor));
            if (raised || from_parents(&exceptions)) && exceptions.insert(key) {
                grew = true;
            }
            if (warns || from_parents(&warnings)) && warnings.insert(key) {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    // A handler catches its class and every base of it, so each base of a caught class is covered too.
    let mut covered: FxHashSet<ClassKey> = FxHashSet::default();
    let mut pending: Vec<ClassKey> = caught.iter().filter_map(|(file, name)| home(*file, name)).collect();
    while let Some(key) = pending.pop() {
        if covered.insert(key.clone()) {
            pending.extend(parents.get(&key).into_iter().flatten().cloned());
        }
    }
    let mut uncaught: FxHashMap<FileId, FxHashSet<String>> = FxHashMap::default();
    for (file, name) in classes.keys() {
        let key = (*file, name.clone());
        if exceptions.contains(&key) && !warnings.contains(&key) && !covered.contains(&key) {
            uncaught.entry(*file).or_default().insert(name.clone());
        }
    }
    uncaught
}

// Whether a base class a run does not define is an exception: a builtin one, or a standard-library one, which by its
// name ends in `Error`, `Exception` or `Warning`.
fn is_exception_name(module: &str, name: &str, minor: u8) -> bool {
    let standard = is_known_standard_library(minor, module.split('.').next().unwrap_or_default());
    is_exception(name, minor)
        || standard && ["Error", "Exception", "Warning"].iter().any(|suffix| name.ends_with(suffix))
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
fn digest(
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

// The classes and aliases each file defines that another exported name's signature reaches, keyed by that file.
fn api_reached(indexed: &[(FileId, &FileFacts)]) -> FxHashMap<FileId, FxHashSet<String>> {
    indexed
        .iter()
        .filter(|(_, facts)| !facts.api_reached.is_empty())
        .map(|(id, facts)| (*id, facts.api_reached.iter().cloned().collect()))
        .collect()
}

fn unqualified(class: &str) -> &str {
    class.rsplit('.').next().unwrap_or(class)
}

// The names a library's users reach. A public module offers what its `__all__` lists, or, without one, every name it
// defines or re-exports without a leading underscore, as the typing spec reads a module; a name another module imports
// from it counts too, since `missing-from-all` lists it there. A module that re-exports a public name, through
// `__all__`, a redundant alias or `import *`, makes it public where it is defined, through any chain of re-exports.
// Names only spread, so this settles.
fn user_facing(
    sources: &FxHashMap<FileId, &FileFacts>,
    roles: &FxHashMap<FileId, ModuleRole>,
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    stars: &[(FileId, FileId)],
    files: &FileIds,
) -> FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>> {
    // What `import *` of a module takes: its `__all__`, or every name it defines without a leading underscore.
    let starred = |name: &&str| Visibility::of(name) == Visibility::Public;
    let star_exports: FxHashMap<FileId, Vec<&str>> = stars
        .iter()
        .map(|(_, source)| {
            let exported = sources.get(source).map_or_else(Vec::new, |facts| match &facts.exports {
                Some(listed) => listed.iter().map(String::as_str).collect(),
                None => facts.defined.iter().map(String::as_str).filter(starred).collect(),
            });
            (*source, exported)
        })
        .collect();
    let mut names: FxHashMap<FileId, FxHashMap<String, Option<FileId>>> = FxHashMap::default();
    for (path, facts) in sources {
        if !roles.get(path).is_some_and(|role| role.public) {
            continue;
        }
        let own = names.entry(*path).or_default();
        let mut add = |name: &str| {
            own.entry(name.to_owned()).or_insert(None);
        };
        match &facts.exports {
            Some(listed) => listed.iter().for_each(|name| add(name)),
            None if facts.exports_unknown => {}
            None => {
                let redundant = facts.bindings.iter().filter(|binding| binding.redundant).map(|binding| &binding.bound);
                facts
                    .defined
                    .iter()
                    .map(String::as_str)
                    .chain(redundant.map(String::as_str))
                    .filter(starred)
                    .for_each(&mut add);
                for (_, source) in stars.iter().filter(|(importer, _)| importer == path) {
                    star_exports[source].iter().copied().for_each(&mut add);
                }
            }
        }
        imported.get(path).into_iter().flatten().map(String::as_str).filter(starred).for_each(&mut add);
    }
    loop {
        let mut reached: Vec<(FileId, String, FileId)> = Vec::new();
        for (importer, source, binding) in bindings {
            if names.get(importer).is_some_and(|known| known.contains_key(&binding.bound)) {
                reached.push((*source, binding.name.clone(), *importer));
            }
        }
        for (importer, source) in stars {
            let Some(known) = names.get(importer) else {
                continue;
            };
            reached.extend(
                star_exports[source]
                    .iter()
                    .filter(|name| known.contains_key(**name))
                    .map(|name| (*source, (*name).to_owned(), *importer)),
            );
        }
        let mut spread = false;
        for (source, name, importer) in reached {
            if let std::collections::hash_map::Entry::Vacant(entry) = names.entry(source).or_default().entry(name) {
                entry.insert(Some(importer));
                spread = true;
            }
        }
        if !spread {
            return names
                .into_iter()
                .map(|(file, names)| {
                    let names = names
                        .into_iter()
                        .map(|(name, importer)| (name, importer.map(|importer| files.path(importer).to_path_buf())))
                        .collect();
                    (file, names)
                })
                .collect();
        }
    }
}

// A name another module takes from a module that star-imports it, as `from pkg import Name` where `pkg/__init__.py` has
// `from ._types import *`, comes from the star-imported module, so it counts there too, through any chain of star
// re-exports, unless that module's `__all__` leaves it out of the `import *`. Names only spread, so this settles.
fn reexport(
    names: &mut FxHashMap<FileId, FxHashSet<String>>,
    stars: &[(FileId, FileId)],
    sources: &FxHashMap<FileId, &FileFacts>,
) {
    loop {
        let mut spread = false;
        for (importer, source) in stars {
            let listed = sources.get(source).and_then(|facts| facts.exports.as_deref());
            let reached: Vec<String> = names
                .get(importer)
                .into_iter()
                .flatten()
                .filter(|name| listed.is_none_or(|listed| listed.contains(name)))
                .filter(|name| !names.get(source).is_some_and(|known| known.contains(*name)))
                .cloned()
                .collect();
            spread |= !reached.is_empty();
            names.entry(*source).or_default().extend(reached);
        }
        if !spread {
            return;
        }
    }
}

// An `import *` of a module that does not resolve, as one found on a `sys.path` entry a `conftest.py` adds, may read any
// file of the run Python could import under that module's last name: `name.py` or `name/__init__.py`. A module no file
// of the run is named after is outside the run, so it reads none of them.
fn unresolved_candidates(
    paths: &FileIds,
    files: impl Iterator<Item = FileId>,
    unresolved: &FxHashSet<&str>,
) -> Vec<FileId> {
    files.filter(|file| last_name(paths.path(*file)).is_some_and(|name| unresolved.contains(name))).collect()
}

// A dotted path through a package that names no module, as `nx.reportviews.Name` where `networkx` re-exports its
// `networkx.classes.reportviews` submodule, may reach any module of that package with the path's last name. Those
// names count as referenced rather than imported: they are kept, yet not made part of the module's API.
fn reach_by_last_name(
    referenced: &mut FxHashMap<FileId, FxHashSet<String>>,
    unreached: &[Unreached],
    paths: &FileIds,
    files: impl Iterator<Item = FileId>,
) {
    if unreached.is_empty() {
        return;
    }
    let mut by_name: FxHashMap<&str, Vec<FileId>> = FxHashMap::default();
    for file in files {
        if let Some(name) = last_name(paths.path(file)) {
            by_name.entry(name).or_default().push(file);
        }
    }
    for (package, name, names) in unreached {
        for file in by_name.get(name).into_iter().flatten().filter(|file| paths.path(**file).starts_with(package)) {
            referenced.entry(*file).or_default().extend(names.iter().cloned());
        }
    }
}

// The last name of the module a file defines: `name` for `name.py` and `name/__init__.py`.
fn last_name(file: &Path) -> Option<&str> {
    let stem = file.file_stem().and_then(|stem| stem.to_str());
    if stem == Some("__init__") { file.parent().and_then(Path::file_name).and_then(|name| name.to_str()) } else { stem }
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
    project: &'a Project,
    imported: Option<&'a FxHashSet<String>>,
    referenced: Option<&'a FxHashSet<String>>,
    star_imported: bool,
    package_star_imported: bool,
    star_source: bool,
    role: ModuleRole,
    user_facing: Option<&'a FxHashMap<String, Option<PathBuf>>>,
    modules: &'a [ModuleUse],
    test_only: Option<&'a FxHashSet<String>>,
    uncaught_exceptions: Option<&'a FxHashSet<String>>,
    api_reached: Option<&'a FxHashSet<String>>,
    calls: FileCalls<'a>,
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
fn module_uses(
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

fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// A dotted path that resolves to no module, by the directory of the package of the run it starts from and its last
// name, with the names it uses.
type Unreached<'f> = (&'f Path, &'f str, &'f [String]);

// A class of the run, by the file that defines it and its name.
type ClassKey = (FileId, String);

// A base class as the file it resolves to, if any, with the module and name the class imports it by.
type ClassBase<F = FileId> = (Option<F>, String, String);

// The files a file's names resolve to, with the names it uses from each, and the files it star-imports, or the last
// name of each star-imported module that resolves to no file. It owns its names, as it outlives the facts of a round.
// `F` is the path of a file, as resolution yields it, or its id once the id table knows it.
struct ModuleFacts<F> {
    constructors: Vec<(F, Vec<NameUse>)>,
    imports: Vec<(F, Vec<String>)>,
    bindings: Vec<(F, ImportBinding)>,
    references: Vec<(F, Vec<String>)>,
    stars: Vec<F>,
    unresolved_stars: Vec<String>,
    unreached: Vec<(PathBuf, String, Vec<String>)>,
    // The name each module binds, the absolute dotted module it stands for, and the file that module resolves to.
    modules: Vec<(String, String, F)>,
    caught: Vec<(F, String)>,
    classes: Vec<(String, Vec<ClassBase<F>>)>,
}

impl<F> ModuleFacts<F> {
    // Gives each file the facts name its id, once resolution has run in parallel and the id table is updated.
    fn map_files<G>(self, mut to: impl FnMut(F) -> G) -> ModuleFacts<G> {
        ModuleFacts {
            constructors: self.constructors.into_iter().map(|(file, uses)| (to(file), uses)).collect(),
            imports: self.imports.into_iter().map(|(file, names)| (to(file), names)).collect(),
            bindings: self.bindings.into_iter().map(|(file, binding)| (to(file), binding)).collect(),
            references: self.references.into_iter().map(|(file, names)| (to(file), names)).collect(),
            stars: self.stars.into_iter().map(&mut to).collect(),
            unresolved_stars: self.unresolved_stars,
            unreached: self.unreached,
            modules: self.modules.into_iter().map(|(bound, module, file)| (bound, module, to(file))).collect(),
            caught: self.caught.into_iter().map(|(file, name)| (to(file), name)).collect(),
            classes: self
                .classes
                .into_iter()
                .map(|(name, bases)| {
                    let bases = bases.into_iter().map(|(file, module, base)| (file.map(&mut to), module, base));
                    (name, bases.collect())
                })
                .collect(),
        }
    }
}

impl ModuleFacts<PathBuf> {
    fn resolve<'f>(
        db: &ModuleDb,
        environment: ResolverEnvironment<'_>,
        path: &Path,
        facts: &'f FileFacts,
        names: &FxHashSet<String>,
    ) -> Self {
        let own = normalized(path);
        let importing =
            SystemPathBuf::from_path_buf(own.clone()).ok().and_then(|path| system_path_to_file(db, path).ok());
        // A file names most modules several times: in its imports, its references and its dataclass uses.
        let cache: RefCell<FxHashMap<&str, Option<PathBuf>>> = RefCell::new(FxHashMap::default());
        let resolve_cached = |module: &'f str| -> Option<PathBuf> {
            if let Some(file) = cache.borrow().get(module) {
                return file.clone();
            }
            let file = resolve(db, environment, importing, module);
            cache.borrow_mut().insert(module, file.clone());
            file
        };
        // A module whose top name no file of the run is named after resolves outside the run, so the resolver is not
        // asked. A star import is the exception: it needs to tell an unresolved module from one outside the run.
        let resolved = |module: &'f str| -> Option<PathBuf> {
            let top = module.split_once('.').map_or(module, |(top, _)| top);
            if !names.contains(top) {
                return None;
            }
            resolve_cached(module)
        };
        let resolve_all = |modules: &'f [ModuleNames]| -> Vec<(PathBuf, Vec<String>)> {
            modules
                .iter()
                .filter_map(|(module, names)| Some((resolved(module)?, names.clone())))
                .filter(|(file, _)| *file != own)
                .collect()
        };
        let mut imports = resolve_all(&facts.imports);
        let references = resolve_all(&facts.references);
        let unreached = facts
            .imports
            .iter()
            .chain(&facts.references)
            .filter_map(|(module, names)| {
                let (top, rest) = module.split_once('.')?;
                let package = resolved(top)?;
                let directory = package.parent().filter(|_| package.ends_with("__init__.py"))?.to_path_buf();
                let last = rest.rsplit('.').next()?;
                resolved(module).is_none().then(|| (directory, last.to_owned(), names.clone()))
            })
            .collect();
        if !facts.declared.is_empty() {
            imports.push((own.with_extension("py"), facts.declared.clone()));
        }
        let mut stars = Vec::new();
        let mut unresolved_stars = Vec::new();
        for module in &facts.star_imports {
            match resolve_cached(module) {
                Some(file) => stars.extend((file != own).then_some(file)),
                None => unresolved_stars.extend(module.rsplit('.').next().map(str::to_owned)),
            }
        }
        let constructors = facts
            .constructors
            .uses
            .iter()
            .filter_map(|(module, uses)| Some((resolved(module)?, uses.clone())))
            .filter(|(file, _)| *file != own)
            .collect();
        let bindings = facts
            .bindings
            .iter()
            .filter_map(|binding| Some((resolved(&binding.module)?, binding.clone())))
            .filter(|(file, _)| *file != own)
            .collect();
        let modules = facts
            .module_names
            .iter()
            .filter_map(|(bound, module)| Some((bound.clone(), module.clone(), resolved(module)?)))
            .filter(|(_, _, file)| *file != own)
            .collect();
        let caught = facts.caught.iter().filter_map(|(module, name)| Some((resolved(module)?, name.clone()))).collect();
        let classes = facts
            .classes
            .iter()
            .map(|class| {
                let bases = class.bases.iter().map(|(module, name)| (resolved(module), module.clone(), name.clone()));
                (class.name.clone(), bases.collect())
            })
            .collect();
        Self {
            constructors,
            imports,
            bindings,
            references,
            stars,
            unresolved_stars,
            unreached,
            modules,
            caught,
            classes,
        }
    }
}

// The source file a module name imports: the `.py` file even when a stub sits next to it, since that is where its names
// are defined; a module with only a stub counts as its `.py` path, which the stub declarations also use.
fn resolve<'db>(
    db: &'db ModuleDb,
    environment: ResolverEnvironment<'db>,
    importing: Option<File>,
    module: &str,
) -> Option<PathBuf> {
    let name = ModuleName::new(module)?;
    let module = match importing {
        Some(file) => resolve_real_module(db, ImportingFile::File(file, environment), &name)
            .or_else(|| resolve_module(db, ImportingFile::File(file, environment), &name)),
        None => resolve_real_module_confident(db, environment, &name)
            .or_else(|| resolve_module_confident(db, environment, &name)),
    }?;
    let path = module.file(db)?.path(db).as_system_path()?.as_std_path().to_path_buf();
    Some(normalized(&path.with_extension("py")))
}
