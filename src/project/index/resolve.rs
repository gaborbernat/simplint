use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;
use ruff_db::files::{File, system_path_to_file};
use ruff_db::system::{OsSystem, SystemPathBuf};
use ruff_graph::{ModuleDb, ResolverEnvironment, resolve_search_paths};
use ruff_python_ast::PythonVersion;
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_text_size::TextRange;
use ty_module_resolver::{
    ImportingFile, Module, ModuleName, SearchPaths, resolve_module, resolve_module_confident, resolve_real_module,
    resolve_real_module_confident,
};

use crate::project::constructors::{NameUse, UseKind};
use crate::project::facts::{ImportBinding, ImportSite, ModuleNames};
use crate::project::{Ending, FileFacts};

use super::reach::last_name;
use super::{FileId, FileIds, SearchRoots};

// Where each file's module names lead, kept for the whole run so a fix round resolves only the files it rewrote. Fixes
// change what files say but never add or remove one, and resolving reads only which files and directories exist, so
// neither the database nor the modules of a file no fix touched go stale.
pub struct ImportResolver {
    // The roots the modules of the run are named from, as `SearchRoots::module_paths` names them.
    pub(super) search: SearchRoots,
    pub(super) db: ModuleDb,
    pub(super) paths: SearchPaths,
    // The search paths each entry point's project resolves its modules with, keyed by the project's directory. A flat
    // layout keeps its modules in that directory, which is not a source root unless `src` names it.
    pub(super) entry_paths: FxHashMap<PathBuf, SearchPaths>,
    // The search paths each coverage plugin's project resolves its plugin modules with, keyed by the project's
    // directory: its own, and the `pythonpath` and `extra-paths` its tools configure. Entry points do not search these.
    pub(super) coverage_paths: FxHashMap<PathBuf, SearchPaths>,
    pub(super) python_version: PythonVersion,
    // The names the files of the run can be imported as, from the components of each file's path.
    pub(super) names: FxHashSet<String>,
    pub(super) files: Arc<FileIds>,
    pub(super) modules: FxHashMap<FileId, ModuleFacts<FileId>>,
}

impl ImportResolver {
    pub fn new<'a>(
        roots: &SearchRoots,
        entry_roots: impl IntoIterator<Item = &'a Path>,
        coverage: impl IntoIterator<Item = (&'a Path, &'a Path)>,
    ) -> Result<Self> {
        let cwd = SystemPathBuf::from_path_buf(std::env::current_dir()?)
            .map_err(|path| anyhow::anyhow!("{} is not a UTF-8 path", path.display()))?;
        let system = OsSystem::new(cwd);
        let src_roots: Vec<SystemPathBuf> =
            roots.roots.iter().filter_map(|root| SystemPathBuf::from_path_buf(root.clone()).ok()).collect();
        let paths = resolve_search_paths(&system, src_roots.clone(), None)?;
        // Like the source roots, a non-UTF-8 directory is no search path, so its entry points stay unresolved.
        let entry_paths = entry_roots
            .into_iter()
            .unique()
            .filter_map(|root| Some((root, SystemPathBuf::from_path_buf(root.to_path_buf()).ok()?)))
            .map(|(root, system_root)| {
                let search_roots = src_roots.iter().cloned().chain([system_root]).collect();
                Ok((root.to_path_buf(), resolve_search_paths(&system, search_roots, None)?))
            })
            .collect::<Result<FxHashMap<_, _>>>()?;
        let coverage_paths = coverage
            .into_iter()
            .into_group_map()
            .into_iter()
            .map(|(root, directories)| {
                let search_roots = src_roots
                    .iter()
                    .cloned()
                    .chain(
                        directories
                            .into_iter()
                            .filter_map(|directory| SystemPathBuf::from_path_buf(directory.to_path_buf()).ok()),
                    )
                    .collect();
                Ok((root.to_path_buf(), resolve_search_paths(&system, search_roots, None)?))
            })
            .collect::<Result<FxHashMap<_, _>>>()?;
        let db = ModuleDb::new(system);
        paths.try_register_static_roots(&db);
        for paths in entry_paths.values().chain(coverage_paths.values()) {
            paths.try_register_static_roots(&db);
        }
        Ok(Self {
            search: roots.clone(),
            db,
            paths,
            entry_paths,
            coverage_paths,
            python_version: roots.python_version,
            names: FxHashSet::default(),
            files: Arc::default(),
            modules: FxHashMap::default(),
        })
    }

    // Drops what the file's module names resolved to, as a fix rewrote the file. The path is absolute.
    pub fn forget(&mut self, path: &Path) {
        if let Some(id) = self.files.get(path) {
            self.modules.remove(&id);
        }
    }

    // Drops every file's resolved module names, for a run that indexes no more files after this.
    pub fn release(&mut self, ending: Ending) {
        ending.end(std::mem::take(&mut self.modules));
    }

    // Resolves the files it has not resolved yet, and returns the id of each file. The paths are absolute.
    pub(super) fn resolve(&mut self, files: &[(&Path, &FileFacts)]) -> Vec<FileId> {
        const GROUP: usize = 64;
        // A `name-stubs` package provides the module `name`, so its stripped name counts too.
        for (path, _) in files {
            for part in *path {
                let name = Path::new(part).file_stem().and_then(|stem| stem.to_str());
                self.names.extend(name.map(|name| name.strip_suffix("-stubs").unwrap_or(name).to_owned()));
            }
        }
        let ids: Vec<FileId> = files.iter().map(|(path, _)| self.intern(path)).collect();
        // The files of one directory share the resolution of each module name: a name the search paths do not hold
        // resolves from the directories above the importing file, which files of one directory have in common. Large
        // directories are split, so one of them does not run as a single job.
        let mut directories: FxHashMap<&Path, Vec<usize>> = FxHashMap::default();
        for (index, (path, _)) in files.iter().enumerate() {
            if !self.modules.contains_key(&ids[index]) {
                let directory = path.parent().unwrap_or(Path::new(""));
                directories.entry(directory).or_default().push(index);
            }
        }
        let groups: Vec<&[usize]> = directories.values().flat_map(|group| group.chunks(GROUP)).collect();
        let (python_version, paths, names) = (self.python_version, &self.paths, &self.names);
        let run_paths: FxHashSet<PathBuf> = files.iter().map(|(path, _)| path.to_path_buf()).collect();
        let mut namesakes: FxHashMap<&str, Vec<&PathBuf>> = FxHashMap::default();
        for path in &run_paths {
            if let Some(name) = last_name(path) {
                namesakes.entry(name).or_default().push(path);
            }
        }
        let run = RunFiles { names, paths: &run_paths, namesakes: &namesakes };
        // The database is not `Sync`, so each worker resolves with its own clone, which shares the cached results.
        let fresh: Vec<Vec<(FileId, ModuleFacts<PathBuf>)>> = groups
            .par_iter()
            .map_with(self.db.clone(), |db, group| {
                let environment = ResolverEnvironment::new(db, python_version, paths.clone());
                let cache = RefCell::new(FxHashMap::default());
                group
                    .iter()
                    .map(|&index| {
                        let (path, facts) = files[index];
                        let facts = ModuleFacts::resolve(db, environment, python_version, path, facts, run, &cache);
                        (ids[index], facts)
                    })
                    .collect()
            })
            .collect();
        for (id, facts) in fresh.into_iter().flatten() {
            let facts = facts.map_files(|path| FileIds::intern(&mut self.files, &path));
            self.modules.insert(id, facts);
        }
        ids
    }

    pub(super) fn intern(&mut self, path: &Path) -> FileId {
        FileIds::intern(&mut self.files, path)
    }
}

pub(super) fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// A dotted path that resolves to no module, by the directory of the package of the run it starts from and its last
// name, with the names it uses.
pub(super) type Unreached<'f> = (&'f Path, &'f str, &'f [String]);

// A class of the run, by the file that defines it and its name.
pub(super) type ClassKey = (FileId, String);

// A base class as the file it resolves to, if any, with the module and name the class imports it by.
pub(super) type ClassBase<F = FileId> = (Option<F>, String, String);

// A file and a name of it, as `class_home` walks the imports that re-export a class.
type Node<'n> = (FileId, &'n str);

// The class a name of a file is, following the imports that re-export it from the file that defines it. `is_class`
// tells the classes of the run apart from the names they bind.
pub(super) fn class_home<'b>(
    is_class: impl Fn(&ClassKey) -> bool + 'b,
    bindings: &[(FileId, FileId, &'b ImportBinding)],
    stars: &[(FileId, FileId)],
) -> impl Fn(FileId, &str) -> Option<ClassKey> + 'b {
    let mut reexports: FxHashMap<FileId, Vec<(&'b str, FileId, &'b str)>> = FxHashMap::default();
    for (importer, source, binding) in bindings {
        reexports.entry(*importer).or_default().push((&binding.bound, *source, &binding.name));
    }
    let mut starred: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in stars {
        starred.entry(*importer).or_default().push(*source);
    }
    let memoize = acyclic(bindings, stars);
    let known: RefCell<FxHashMap<(FileId, String), Option<ClassKey>>> = RefCell::default();
    move |file: FileId, name: &str| -> Option<ClassKey> {
        let mut pending: Vec<(Node, Option<Node>)> = vec![((file, name), None)];
        let mut parents: FxHashMap<Node, Option<Node>> = FxHashMap::default();
        let mut found = None;
        while let Some((node, parent)) = pending.pop() {
            if parents.contains_key(&node) {
                continue;
            }
            parents.insert(node, parent);
            let key = (node.0, node.1.to_owned());
            let cached = if memoize { known.borrow().get(&key).cloned() } else { None };
            match cached {
                Some(Some(class)) => {
                    found = Some((node, class));
                    break;
                }
                Some(None) => continue,
                None if is_class(&key) => {
                    found = Some((node, key));
                    break;
                }
                None => {}
            }
            let (file, name) = node;
            for (bound, source, original) in reexports.get(&file).into_iter().flatten() {
                if *bound == name {
                    pending.push(((*source, *original), Some(node)));
                }
            }
            pending.extend(starred.get(&file).into_iter().flatten().map(|source| ((*source, name), Some(node))));
        }
        if memoize {
            match &found {
                Some((node, class)) => {
                    let mut known = known.borrow_mut();
                    let mut at = Some(*node);
                    while let Some(member) = at {
                        known.insert((member.0, member.1.to_owned()), Some(class.clone()));
                        at = parents[&member];
                    }
                }
                None => {
                    known.borrow_mut().extend(parents.keys().map(|&(file, name)| ((file, name.to_owned()), None)));
                }
            }
        }
        found.map(|(_, class)| class)
    }
}

// Whether no file reaches itself through the imports that bind names or star-import them. `class_home` memoises only
// then: a walk that loops back to a file on its own path can reach a different class than a walk from a file it passed
// through would, so the memo would change which class a name is.
fn acyclic(bindings: &[(FileId, FileId, &ImportBinding)], stars: &[(FileId, FileId)]) -> bool {
    let mut successors: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    let mut indegree: FxHashMap<FileId, usize> = FxHashMap::default();
    for (importer, source) in
        bindings.iter().map(|(importer, source, _)| (*importer, *source)).chain(stars.iter().copied())
    {
        successors.entry(importer).or_default().push(source);
        *indegree.entry(source).or_default() += 1;
        indegree.entry(importer).or_default();
    }
    let files = indegree.len();
    let mut ready: Vec<FileId> = indegree.iter().filter(|(_, degree)| **degree == 0).map(|(file, _)| *file).collect();
    let mut ordered = 0;
    while let Some(file) = ready.pop() {
        ordered += 1;
        for source in successors.get(&file).into_iter().flatten() {
            let degree = indegree.entry(*source).or_default();
            *degree -= 1;
            if *degree == 0 {
                ready.push(*source);
            }
        }
    }
    ordered == files
}

// The modules the names of a file bind, each with the file it resolves to when that file is not the file itself.
fn named_modules<'f>(
    names: &'f [(String, String)],
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
    own: &Path,
) -> Vec<(String, String, PathBuf)> {
    names
        .iter()
        .filter_map(|(bound, module)| Some((bound.clone(), module.clone(), resolved(module)?)))
        .filter(|(_, _, file)| file != own)
        .collect()
}

// The calls a module's uses name in the file the module resolves to. A module that resolves to no file at all, such as
// `distutils.log` where the standard library has no `distutils` and the run shims its own `log`, may be the run's file of
// the same last name, so the calls it makes with their value used read that file.
fn constructor_reads<'f>(
    module: &'f str,
    uses: &[NameUse],
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
    found: &impl Fn(&str) -> bool,
    run: RunFiles,
) -> Vec<(PathBuf, Vec<NameUse>)> {
    if let Some(file) = resolved(module) {
        return vec![(file, uses.to_vec())];
    }
    let reads: Vec<NameUse> =
        uses.iter().filter(|name| matches!(name.kind, UseKind::Call(_)) && !name.discarded).cloned().collect();
    if reads.is_empty() {
        return Vec::new();
    }
    let last = module.rsplit('.').next().unwrap_or(module);
    match run.namesakes.get(last) {
        Some(namesakes) if !found(module) => namesakes.iter().map(|path| ((*path).clone(), reads.clone())).collect(),
        _ => Vec::new(),
    }
}

// The files of the run, by the names they are called by and by their paths, which resolution checks a module against,
// and by their last names, which a module that resolves to no file is matched with.
#[derive(Clone, Copy)]
pub(super) struct RunFiles<'a> {
    names: &'a FxHashSet<String>,
    paths: &'a FxHashSet<PathBuf>,
    namesakes: &'a FxHashMap<&'a str, Vec<&'a PathBuf>>,
}

// The files a file's names resolve to, with the names it uses from each, and the files it star-imports, or the last
// name of each star-imported module that resolves to no file. It owns its names, as it outlives the facts of a round.
// `F` is the path of a file, as resolution yields it, or its id once the id table knows it.
pub(super) struct ModuleFacts<F> {
    pub(super) constructors: Vec<(F, Vec<NameUse>)>,
    pub(super) imports: Vec<(F, Vec<String>)>,
    pub(super) bindings: Vec<(F, ImportBinding)>,
    pub(super) references: Vec<(F, Vec<String>)>,
    pub(super) doctest_references: Vec<(F, Vec<String>)>,
    pub(super) stars: Vec<F>,
    pub(super) unresolved_stars: Vec<String>,
    pub(super) unreached_imports: Vec<(PathBuf, String, Vec<String>)>,
    pub(super) unreached_references: Vec<(PathBuf, String, Vec<String>)>,
    pub(super) unreached_doctests: Vec<(PathBuf, String, Vec<String>)>,
    // The names the file imports or reads from modules that resolve to no file of the run, which may be definitions of
    // the run under a name simplint cannot resolve, as CPython's frozen `_frozen_importlib_external` is.
    pub(super) unresolved_names: Vec<String>,
    // The name each module binds, the absolute dotted module it stands for, and the file that module resolves to.
    pub(super) modules: Vec<(String, String, F)>,
    pub(super) caught: Vec<(F, String)>,
    // Each class with the bases it lists and the names of the methods it defines.
    pub(super) classes: Vec<(String, Vec<ClassBase<F>>, Vec<String>)>,
    pub(super) functions: Vec<String>,
    // The modules a top-level `import pkg.module` binds by their root name, as `modules` gives them.
    pub(super) plain_modules: Vec<(String, String, F)>,
    // The files the import statements that run when the module loads import, and the range of each statement.
    pub(super) runtime: Vec<(F, TextRange)>,
    // The files a write under a computed name reaches, whose names any of them may be set by it.
    pub(super) computed_writes: Vec<F>,
    // The names a `setattr` call of the file sets on each module that resolves to a file of the run.
    pub(super) written: Vec<(F, Vec<String>)>,
}

impl<F> ModuleFacts<F> {
    // Gives each file the facts name its id, once resolution has run in parallel and the id table is updated.
    fn map_files<G>(self, mut to: impl FnMut(F) -> G) -> ModuleFacts<G> {
        ModuleFacts {
            constructors: self.constructors.into_iter().map(|(file, uses)| (to(file), uses)).collect(),
            imports: self.imports.into_iter().map(|(file, names)| (to(file), names)).collect(),
            bindings: self.bindings.into_iter().map(|(file, binding)| (to(file), binding)).collect(),
            references: self.references.into_iter().map(|(file, names)| (to(file), names)).collect(),
            doctest_references: self.doctest_references.into_iter().map(|(file, names)| (to(file), names)).collect(),
            stars: self.stars.into_iter().map(&mut to).collect(),
            unresolved_stars: self.unresolved_stars,
            unreached_imports: self.unreached_imports,
            unreached_references: self.unreached_references,
            unreached_doctests: self.unreached_doctests,
            unresolved_names: self.unresolved_names,
            modules: self.modules.into_iter().map(|(bound, module, file)| (bound, module, to(file))).collect(),
            caught: self.caught.into_iter().map(|(file, name)| (to(file), name)).collect(),
            classes: self
                .classes
                .into_iter()
                .map(|(name, bases, methods)| {
                    let bases = bases.into_iter().map(|(file, module, base)| (file.map(&mut to), module, base));
                    (name, bases.collect(), methods)
                })
                .collect(),
            functions: self.functions,
            plain_modules: self
                .plain_modules
                .into_iter()
                .map(|(bound, module, file)| (bound, module, to(file)))
                .collect(),
            runtime: self.runtime.into_iter().map(|(file, range)| (to(file), range)).collect(),
            computed_writes: self.computed_writes.into_iter().map(&mut to).collect(),
            written: self.written.into_iter().map(|(file, names)| (to(file), names)).collect(),
        }
    }
}

impl ModuleFacts<PathBuf> {
    fn resolve<'f>(
        db: &ModuleDb,
        environment: ResolverEnvironment<'_>,
        python_version: PythonVersion,
        path: &Path,
        facts: &'f FileFacts,
        run: RunFiles,
        cache: &RefCell<FxHashMap<&'f str, Option<PathBuf>>>,
    ) -> Self {
        let own = path.to_path_buf();
        let importing =
            SystemPathBuf::from_path_buf(own.clone()).ok().and_then(|path| system_path_to_file(db, path).ok());
        // The files of a directory name most modules several times, in imports, references and dataclass uses.
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
        let found = |module: &str| is_module_found(db, environment, importing, module);
        let resolved = |module: &'f str| -> Option<PathBuf> {
            let top = module.split_once('.').map_or(module, |(top, _)| top);
            if !run.names.contains(top) {
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
        let written = written_names(facts, &resolved);
        let unresolved_names = facts
            .imports
            .iter()
            .chain(&facts.references)
            .chain(&facts.doctest_references)
            .filter(|(module, _)| resolved(module).is_none())
            .flat_map(|(_, names)| names.iter().cloned())
            .collect();
        let unreached_imports = unreached_names(&facts.imports, &resolved, run, &own);
        let unreached_references = unreached_names(&facts.references, &resolved, run, &own);
        if !facts.declared.is_empty() {
            imports.push((own.with_extension("py"), facts.declared.clone()));
        }
        let (stars, unresolved_stars) = star_targets(&facts.star_imports, &resolve_cached, &own);
        let constructors = facts
            .constructors
            .uses
            .iter()
            .flat_map(|(module, uses)| constructor_reads(module, uses, &resolved, &found, run))
            .filter(|(file, _)| *file != own)
            .collect();
        let bindings = facts
            .bindings
            .iter()
            .filter_map(|binding| Some((resolved(&binding.module)?, binding.clone())))
            .filter(|(file, _)| *file != own)
            .collect();
        let modules = named_modules(&facts.module_names, &resolved, &own);
        let caught = facts.caught.iter().filter_map(|(module, name)| Some((resolved(module)?, name.clone()))).collect();
        let runtime = runtime_targets(&facts.runtime_imports, python_version, &resolved, &own);
        // A module may write its own names under a computed name too, so its own file stays.
        let computed_writes = facts.computed_writes.iter().filter_map(|module| resolved(module)).collect();
        let classes = facts
            .classes
            .iter()
            .map(|class| {
                let bases = class.bases.iter().map(|(module, name)| (resolved(module), module.clone(), name.clone()));
                (class.name.clone(), bases.collect(), class.methods.clone())
            })
            .collect();
        Self {
            constructors,
            imports,
            bindings,
            references,
            doctest_references: resolve_all(&facts.doctest_references),
            stars,
            unresolved_stars,
            unreached_imports,
            unreached_references,
            unreached_doctests: unreached_names(&facts.doctest_references, &resolved, run, &own),
            unresolved_names,
            modules,
            caught,
            classes,
            functions: facts.functions.clone(),
            plain_modules: named_modules(&facts.plain_modules, &resolved, &own),
            runtime,
            computed_writes,
            written,
        }
    }
}

// The source file a module name imports: the `.py` file even when a stub sits next to it, since that is where its names
// are defined; a module with only a stub counts as its `.py` path, which the stub declarations also use.
// Whether the module exists for the run: a standard-library module, or one the resolver finds, including one that ships
// with Python, whose stub has no file on disk. The newest Python's standard library decides, so a module older Pythons
// ship and the newest lacks, such as `distutils`, is one a run file of its name may shadow.
pub(super) fn is_module_found<'db>(
    db: &'db ModuleDb,
    environment: ResolverEnvironment<'db>,
    importing: Option<File>,
    module: &str,
) -> bool {
    is_known_standard_library(PythonVersion::latest().minor, module.split('.').next().unwrap_or_default())
        || ModuleName::new(module).is_some_and(|name| locate(db, environment, importing, &name).is_some())
}

fn locate<'db>(
    db: &'db ModuleDb,
    environment: ResolverEnvironment<'db>,
    importing: Option<File>,
    name: &ModuleName,
) -> Option<Module<'db>> {
    match importing {
        Some(file) => resolve_real_module(db, ImportingFile::File(file, environment), name)
            .or_else(|| resolve_module(db, ImportingFile::File(file, environment), name)),
        None => resolve_real_module_confident(db, environment, name)
            .or_else(|| resolve_module_confident(db, environment, name)),
    }
}

pub(super) fn resolve<'db>(
    db: &'db ModuleDb,
    environment: ResolverEnvironment<'db>,
    importing: Option<File>,
    module: &str,
) -> Option<PathBuf> {
    let name = ModuleName::new(module)?;
    let module = locate(db, environment, importing, &name)?;
    let path = module.file(db)?.path(db).as_system_path()?.as_std_path().to_path_buf();
    Some(normalized(&path.with_extension("py")))
}

// The names a file sets on each module of the run; a `setattr` on an object that is no module of the run sets none.
fn written_names<'f>(
    facts: &'f FileFacts,
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
) -> Vec<(PathBuf, Vec<String>)> {
    facts.module_writes.iter().filter_map(|(module, names)| Some((resolved(module)?, names.clone()))).collect()
}

// The names a module reaches through a package of the run whose submodule no file of the run is.
fn unreached_names<'f>(
    modules: &'f [ModuleNames],
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
    run: RunFiles,
    own: &Path,
) -> Vec<(PathBuf, String, Vec<String>)> {
    modules
        .iter()
        .filter_map(|(module, names)| {
            // A module named like a file of the run that resolves outside the run, such as the standard library's
            // `typing_extensions` for a `typing_extensions.py` of the run, may be that file, imported from anywhere.
            // A file that imports a module of its own name from outside the run, as a `tomllib` backport does, is not
            // that module, so the import is no import of the file's own names.
            let Some((top, rest)) = module.split_once('.') else {
                let shadows = run.names.contains(module.as_str())
                    && !resolved(module).is_some_and(|file| run.paths.contains(&file))
                    && last_name(own) != Some(module.as_str());
                return shadows.then(|| (PathBuf::new(), module.clone(), names.clone()));
            };
            let package = resolved(top)?;
            let directory = package.parent().filter(|_| package.ends_with("__init__.py"))?.to_path_buf();
            let last = rest.rsplit('.').next()?;
            resolved(module).is_none().then(|| (directory, last.to_owned(), names.clone()))
        })
        .collect()
}

// The files a module's star imports read, and the last name of each star-imported module the resolver does not find.
fn star_targets<'f>(
    modules: &'f [String],
    resolve: &impl Fn(&'f str) -> Option<PathBuf>,
    own: &Path,
) -> (Vec<PathBuf>, Vec<String>) {
    let mut stars = Vec::new();
    let mut unresolved = Vec::new();
    for module in modules {
        match resolve(module) {
            Some(file) => stars.extend((file != own).then_some(file)),
            None => unresolved.extend(module.rsplit('.').next().map(str::to_owned)),
        }
    }
    (stars, unresolved)
}

// The files each runtime import statement runs, with the statement's range. A `from` import reads each name as a
// submodule when one exists, and from the module otherwise, as Python does. An import a version check keeps to Pythons
// older than the project's target never runs.
fn runtime_targets<'f>(
    sites: &'f [ImportSite],
    python_version: PythonVersion,
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
    own: &Path,
) -> Vec<(PathBuf, TextRange)> {
    let version = (python_version.major, python_version.minor);
    let mut runtime = Vec::new();
    for site in sites.iter().filter(|site| site.below.is_none_or(|below| version < below)) {
        let mut targets = Vec::new();
        if site.submodules.is_empty() {
            targets.extend(resolved(&site.module));
        } else {
            let mut reads_module = false;
            for submodule in &site.submodules {
                match resolved(submodule) {
                    Some(file) => targets.push(file),
                    None => reads_module = true,
                }
            }
            if reads_module {
                targets.extend(resolved(&site.module));
            }
        }
        runtime.extend(targets.into_iter().filter(|file| file != own).map(|file| (file, site.range())));
    }
    runtime
}
