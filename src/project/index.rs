use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rayon::prelude::*;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_db::files::{File, system_path_to_file};
use ruff_db::system::{OsSystem, SystemPathBuf};
use ruff_graph::{ModuleDb, ResolverEnvironment, resolve_search_paths};
use ruff_linter::package::PackageRoot;
use ruff_python_ast::PythonVersion;
use ty_module_resolver::{
    ImportingFile, ModuleName, resolve_module, resolve_module_confident, resolve_real_module,
    resolve_real_module_confident,
};

use super::FileFacts;
use super::facts::ModuleNames;

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
}

// Which names of each file other files use, keyed by the file a name resolves to. Module names resolve with ty's module
// resolver, the one Ruff's import graph uses, so a module counts under the name Python would import it by, whether its
// package has an `__init__.py` or sits in a pytest rootdir.
pub struct Project {
    imported: HashMap<PathBuf, HashSet<String>>,
    // Names other code reaches by a dotted string, such as a `mock.patch` target: renaming them breaks that code, yet
    // they are not part of the module's API.
    referenced: HashMap<PathBuf, HashSet<String>>,
    assigned_attributes: HashSet<String>,
    base_classes: HashSet<String>,
    digest: u64,
}

impl Project {
    pub fn index(files: &[(&Path, &FileFacts)], entry_points: &[String], roots: &SearchRoots) -> Result<Self> {
        let cwd = SystemPathBuf::from_path_buf(std::env::current_dir()?)
            .map_err(|path| anyhow::anyhow!("{} is not a UTF-8 path", path.display()))?;
        let system = OsSystem::new(cwd);
        let src_roots = roots.roots.iter().filter_map(|root| SystemPathBuf::from_path_buf(root.clone()).ok()).collect();
        let paths = resolve_search_paths(&system, src_roots, None)?;
        let db = ModuleDb::new(system);
        paths.try_register_static_roots(&db);
        let python_version = roots.python_version;
        // The database is not `Sync`, so each worker resolves with its own clone, which shares the cached results.
        let modules: Vec<ModuleFacts> = files
            .par_iter()
            .map_with(db.clone(), |db, (path, facts)| {
                let environment = ResolverEnvironment::new(db, python_version, paths.clone());
                ModuleFacts::resolve(db, environment, path, facts)
            })
            .collect();
        let mut imported: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        let mut referenced: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        for module in modules {
            for (file, names) in module.imports {
                imported.entry(file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in module.references {
                referenced.entry(file).or_default().extend(names.iter().cloned());
            }
        }
        let assigned_attributes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.assigned_attributes.iter().cloned()).collect();
        let base_classes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.base_classes.iter().cloned()).collect();
        let environment = ResolverEnvironment::new(&db, python_version, paths);
        for (module, attribute) in entry_points.iter().filter_map(|entry_point| entry_point.split_once(':')) {
            if let Some(file) = resolve(&db, environment, None, module.trim()) {
                let name = attribute.split('.').next().unwrap_or_default().trim();
                imported.entry(file).or_default().insert(name.to_owned());
            }
        }
        let mut hasher = CacheKeyHasher::new();
        assigned_attributes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        base_classes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        Ok(Self { imported, referenced, assigned_attributes, base_classes, digest: hasher.finish() })
    }

    // A file's entry is looked up once, so rules asking about many names do not normalize and hash its path each time.
    pub fn file(&self, path: &Path) -> FileIndex<'_> {
        let path = normalized(path);
        FileIndex { project: self, imported: self.imported.get(&path), referenced: self.referenced.get(&path) }
    }
}

// What the index knows about one file: the names other files use from it, and the attribute and base class names of
// the whole run.
#[derive(Clone, Copy)]
pub struct FileIndex<'a> {
    project: &'a Project,
    imported: Option<&'a HashSet<String>>,
    referenced: Option<&'a HashSet<String>>,
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged.
impl CacheKey for FileIndex<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.project.digest.cache_key(state);
        for names in [self.imported, self.referenced] {
            names.into_iter().flatten().sorted().for_each(|name| name.cache_key(state));
            0u8.cache_key(state);
        }
    }
}

impl FileIndex<'_> {
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported.is_some_and(|names| names.contains(name))
    }

    pub fn is_referenced(&self, name: &str) -> bool {
        self.is_imported(name) || self.referenced.is_some_and(|names| names.contains(name))
    }

    pub fn is_attribute_assigned(&self, name: &str) -> bool {
        self.project.assigned_attributes.contains(name)
    }

    pub fn is_base_class(&self, name: &str) -> bool {
        self.project.base_classes.contains(name)
    }
}

fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// The files a file's names resolve to, with the names it uses from each.
struct ModuleFacts<'f> {
    imports: Vec<(PathBuf, &'f [String])>,
    references: Vec<(PathBuf, &'f [String])>,
}

impl<'f> ModuleFacts<'f> {
    fn resolve(db: &ModuleDb, environment: ResolverEnvironment<'_>, path: &Path, facts: &'f FileFacts) -> Self {
        let own = normalized(path);
        let importing =
            SystemPathBuf::from_path_buf(own.clone()).ok().and_then(|path| system_path_to_file(db, path).ok());
        let resolve_all = |modules: &'f [ModuleNames]| -> Vec<(PathBuf, &'f [String])> {
            modules
                .iter()
                .filter_map(|(module, names)| Some((resolve(db, environment, importing, module)?, names.as_slice())))
                .filter(|(file, _)| *file != own)
                .collect()
        };
        let mut imports = resolve_all(&facts.imports);
        let references = resolve_all(&facts.references);
        if !facts.declared.is_empty() {
            imports.push((own.with_extension("py"), &facts.declared));
        }
        Self { imports, references }
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
