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
use ruff_text_size::TextRange;
use ty_module_resolver::{
    ImportingFile, ModuleName, SearchPaths, resolve_module, resolve_module_confident, resolve_real_module,
    resolve_real_module_confident,
};

use crate::project::constructors::NameUse;
use crate::project::facts::{ImportBinding, ImportSite, ModuleNames};
use crate::project::{Ending, FileFacts};

use super::{FileId, FileIds, SearchRoots};

// Where each file's module names lead, kept for the whole run so a fix round resolves only the files it rewrote. Fixes
// change what files say but never add or remove one, and resolving reads only which files and directories exist, so
// neither the database nor the modules of a file no fix touched go stale.
pub struct ImportResolver {
    pub(super) db: ModuleDb,
    pub(super) paths: SearchPaths,
    pub(super) python_version: PythonVersion,
    // The names the files of the run can be imported as, from the components of each file's path.
    pub(super) names: FxHashSet<String>,
    pub(super) files: Arc<FileIds>,
    pub(super) modules: FxHashMap<FileId, ModuleFacts<FileId>>,
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
        let run = RunFiles { names, paths: &run_paths };
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

// The files of the run, by the names they are called by and by their paths, which resolution checks a module against.
#[derive(Clone, Copy)]
pub(super) struct RunFiles<'a> {
    names: &'a FxHashSet<String>,
    paths: &'a FxHashSet<PathBuf>,
}

// The files a file's names resolve to, with the names it uses from each, and the files it star-imports, or the last
// name of each star-imported module that resolves to no file. It owns its names, as it outlives the facts of a round.
// `F` is the path of a file, as resolution yields it, or its id once the id table knows it.
pub(super) struct ModuleFacts<F> {
    pub(super) constructors: Vec<(F, Vec<NameUse>)>,
    pub(super) imports: Vec<(F, Vec<String>)>,
    pub(super) bindings: Vec<(F, ImportBinding)>,
    pub(super) references: Vec<(F, Vec<String>)>,
    pub(super) stars: Vec<F>,
    pub(super) unresolved_stars: Vec<String>,
    pub(super) unreached: Vec<(PathBuf, String, Vec<String>)>,
    // The names the file imports or reads from modules that resolve to no file of the run, which may be definitions of
    // the run under a name simplint cannot resolve, as CPython's frozen `_frozen_importlib_external` is.
    pub(super) unresolved_names: Vec<String>,
    // The name each module binds, the absolute dotted module it stands for, and the file that module resolves to.
    pub(super) modules: Vec<(String, String, F)>,
    pub(super) caught: Vec<(F, String)>,
    pub(super) classes: Vec<(String, Vec<ClassBase<F>>)>,
    // The files the import statements that run when the module loads import, and the range of each statement.
    pub(super) runtime: Vec<(F, TextRange)>,
    // The files each `import *` statement of the module imports, and the range of each statement.
    pub(super) star_sites: Vec<(F, TextRange)>,
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
            unresolved_names: self.unresolved_names,
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
            runtime: self.runtime.into_iter().map(|(file, range)| (to(file), range)).collect(),
            star_sites: self.star_sites.into_iter().map(|(file, range)| (to(file), range)).collect(),
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
        let unresolved_names = facts
            .imports
            .iter()
            .chain(&facts.references)
            .filter(|(module, _)| resolved(module).is_none())
            .flat_map(|(_, names)| names.iter().cloned())
            .collect();
        let unreached = unreached_names(facts, &resolved, run);
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
        let runtime = runtime_targets(&facts.runtime_imports, python_version, &resolved, &own);
        let star_sites = facts
            .star_sites
            .iter()
            .filter_map(|site| Some((resolved(&site.module)?, site.range())))
            .filter(|(file, _)| *file != own)
            .collect();
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
            unresolved_names,
            modules,
            caught,
            classes,
            runtime,
            star_sites,
        }
    }
}

// The source file a module name imports: the `.py` file even when a stub sits next to it, since that is where its names
// are defined; a module with only a stub counts as its `.py` path, which the stub declarations also use.
pub(super) fn resolve<'db>(
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

// The names a module reaches through a package of the run whose submodule no file of the run is.
fn unreached_names<'f>(
    facts: &'f FileFacts,
    resolved: &impl Fn(&'f str) -> Option<PathBuf>,
    run: RunFiles,
) -> Vec<(PathBuf, String, Vec<String>)> {
    facts
        .imports
        .iter()
        .chain(&facts.references)
        .filter_map(|(module, names)| {
            // A module named like a file of the run that resolves outside the run, such as the standard library's
            // `typing_extensions` for a `typing_extensions.py` of the run, may be that file, imported from anywhere.
            let Some((top, rest)) = module.split_once('.') else {
                let shadows = run.names.contains(module.as_str())
                    && !resolved(module).is_some_and(|file| run.paths.contains(&file));
                return shadows.then(|| (PathBuf::new(), module.clone(), names.clone()));
            };
            let package = resolved(top)?;
            let directory = package.parent().filter(|_| package.ends_with("__init__.py"))?.to_path_buf();
            let last = rest.rsplit('.').next()?;
            resolved(module).is_none().then(|| (directory, last.to_owned(), names.clone()))
        })
        .collect()
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
