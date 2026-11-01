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
use super::constructors::{DataclassCalls, FileCalls, NameUse};
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
    // Modules without a static `__all__` that an `import *` may read, so each of their public names may be in use.
    star_imported: HashSet<PathBuf>,
    assigned_attributes: HashSet<String>,
    base_classes: HashSet<String>,
    writes: Writes,
    calls: DataclassCalls,
    digest: u64,
}

// The attribute writes of the run: by the qualified class of the instance written to when parsing tells, and otherwise
// by name.
#[derive(Default)]
struct Writes {
    on_class: HashMap<String, HashSet<String>>,
    untyped: HashSet<String>,
    unslottable_bases: HashSet<String>,
}

impl Project {
    // `rewrites_calls` holds the files whose own `dataclass-kw-only` reports rewrite their positional dataclass calls.
    pub fn index(
        files: &[(&Path, &FileFacts)],
        entry_points: &[String],
        roots: &SearchRoots,
        rewrites_calls: &HashSet<&Path>,
    ) -> Result<Self> {
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
                ModuleFacts::resolve(db, environment, path, facts, rewrites_calls.contains(path))
            })
            .collect();
        let mut imported: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        let mut referenced: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        let mut stars: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut unresolved_stars: HashSet<&str> = HashSet::new();
        let mut constructor_uses = Vec::new();
        for module in modules {
            constructor_uses.push((module.path.clone(), module.rewrites_calls, module.constructors));
            for (file, names) in module.imports {
                imported.entry(file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in module.references {
                referenced.entry(file).or_default().extend(names.iter().cloned());
            }
            stars.extend(module.stars.into_iter().map(|source| (module.path.clone(), source)));
            unresolved_stars.extend(module.unresolved_stars);
        }
        let exports: HashMap<PathBuf, Option<&[String]>> = files
            .iter()
            .filter(|(path, _)| path.extension().is_none_or(|extension| extension != "pyi"))
            .map(|(path, facts)| (normalized(path), facts.exports.as_deref()))
            .collect();
        reexport(&mut imported, &stars, &exports);
        reexport(&mut referenced, &stars, &exports);
        let star_imported = stars
            .iter()
            .map(|(_, source)| source.clone())
            .chain(unresolved_candidates(exports.keys(), &unresolved_stars))
            .filter(|file| exports.get(file).is_some_and(Option::is_none))
            .collect();
        let assigned_attributes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.assigned_attributes.iter().cloned()).collect();
        let base_classes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.base_classes.iter().cloned()).collect();
        let mut writes = Writes::default();
        for (_, facts) in files {
            let facts = &facts.attribute_writes;
            for (class, attribute) in &facts.typed {
                writes.on_class.entry(class.clone()).or_default().insert(attribute.clone());
            }
            writes.untyped.extend(facts.untyped.iter().cloned());
            writes.unslottable_bases.extend(facts.unslottable_bases.iter().cloned());
        }
        let environment = ResolverEnvironment::new(&db, python_version, paths);
        for (module, attribute) in entry_points.iter().filter_map(|entry_point| entry_point.split_once(':')) {
            if let Some(file) = resolve(&db, environment, None, module.trim()) {
                let name = attribute.split('.').next().unwrap_or_default().trim();
                imported.entry(file).or_default().insert(name.to_owned());
            }
        }
        let declared = files.iter().map(|(path, facts)| (normalized(path), facts.constructors.dataclasses.as_slice()));
        let calls = DataclassCalls::index(declared, &constructor_uses, &imported);
        let mut hasher = CacheKeyHasher::new();
        assigned_attributes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        base_classes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        for (class, attributes) in writes.on_class.iter().sorted_by(|left, right| left.0.cmp(right.0)) {
            class.cache_key(&mut hasher);
            attributes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        }
        for names in [&writes.untyped, &writes.unslottable_bases] {
            0u8.cache_key(&mut hasher);
            names.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
        }
        Ok(Self {
            imported,
            referenced,
            star_imported,
            assigned_attributes,
            base_classes,
            writes,
            calls,
            digest: hasher.finish(),
        })
    }

    // The attribute and base class names one index has and the other lacks, whose answers differ between the two, and
    // the names of the classes whose attribute writes, or subclasses that a slotted base breaks, differ, which a
    // dataclass of that name, or a namesake of it, reads.
    pub fn changed_names<'p>(&'p self, other: &'p Self) -> HashSet<&'p str> {
        let (writes, others) = (&self.writes, &other.writes);
        let classes = writes
            .on_class
            .iter()
            .filter(|(class, attributes)| others.on_class.get(*class) != Some(*attributes))
            .chain(others.on_class.iter().filter(|(class, _)| !writes.on_class.contains_key(*class)))
            .map(|(class, _)| class)
            .chain(writes.unslottable_bases.symmetric_difference(&others.unslottable_bases))
            .map(|class| class.rsplit('.').next().unwrap_or(class));
        self.assigned_attributes
            .symmetric_difference(&other.assigned_attributes)
            .chain(self.base_classes.symmetric_difference(&other.base_classes))
            .chain(writes.untyped.symmetric_difference(&others.untyped))
            .map(String::as_str)
            .chain(classes)
            .collect()
    }

    // A file's entry is looked up once, so rules asking about many names do not normalize and hash its path each time.
    pub fn file(&self, path: &Path) -> FileIndex<'_> {
        let path = normalized(path);
        FileIndex {
            project: self,
            imported: self.imported.get(&path),
            referenced: self.referenced.get(&path),
            star_imported: self.star_imported.contains(&path),
            calls: self.calls.file(&path),
        }
    }
}

// A name another module takes from a module that star-imports it, as `from pkg import Name` where `pkg/__init__.py` has
// `from ._types import *`, comes from the star-imported module, so it counts there too, through any chain of star
// re-exports, unless that module's `__all__` leaves it out of the `import *`. Names only spread, so this settles.
fn reexport(
    names: &mut HashMap<PathBuf, HashSet<String>>,
    stars: &[(PathBuf, PathBuf)],
    exports: &HashMap<PathBuf, Option<&[String]>>,
) {
    loop {
        let mut spread = false;
        for (importer, source) in stars {
            let listed = exports.get(source).copied().flatten();
            let reached: Vec<String> = names
                .get(importer)
                .into_iter()
                .flatten()
                .filter(|name| listed.is_none_or(|listed| listed.contains(name)))
                .filter(|name| !names.get(source).is_some_and(|known| known.contains(*name)))
                .cloned()
                .collect();
            spread |= !reached.is_empty();
            names.entry(source.clone()).or_default().extend(reached);
        }
        if !spread {
            return;
        }
    }
}

// An `import *` of a module that does not resolve, as one found on a `sys.path` entry a `conftest.py` adds, may read any
// file of the run Python could import under that module's last name: `name.py` or `name/__init__.py`. A module no file
// of the run is named after is outside the run, so it reads none of them.
fn unresolved_candidates<'p>(files: impl Iterator<Item = &'p PathBuf>, unresolved: &HashSet<&str>) -> Vec<PathBuf> {
    files
        .filter(|file| {
            let stem = file.file_stem().and_then(|stem| stem.to_str());
            let name = if stem == Some("__init__") {
                file.parent().and_then(Path::file_name).and_then(|name| name.to_str())
            } else {
                stem
            };
            name.is_some_and(|name| unresolved.contains(name))
        })
        .cloned()
        .collect()
}

// What the index knows about one file: the names other files use from it, and the attribute and base class names of
// the whole run.
#[derive(Clone, Copy)]
pub struct FileIndex<'a> {
    project: &'a Project,
    imported: Option<&'a HashSet<String>>,
    referenced: Option<&'a HashSet<String>>,
    star_imported: bool,
    calls: FileCalls<'a>,
}

// Two indexes give a file the same answers when the names other files use from it and its dataclass calls match; the
// attribute and base class names of the whole run are compared apart, by `Project::changed_names`.
impl PartialEq for FileIndex<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.imported, self.referenced, self.star_imported, self.calls)
            == (other.imported, other.referenced, other.star_imported, other.calls)
    }
}

// The index slice a file's diagnostics depend on; a cached result stays valid while this is unchanged.
impl CacheKey for FileIndex<'_> {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.project.digest.cache_key(state);
        for names in [self.imported, self.referenced] {
            names.into_iter().flatten().sorted().for_each(|name| name.cache_key(state));
            0u8.cache_key(state);
        }
        self.star_imported.cache_key(state);
        self.calls.cache_key(state);
    }
}

impl<'a> FileIndex<'a> {
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported.is_some_and(|names| names.contains(name))
    }

    // A public name of a module some `import *` reads may be used under that import, though no file names it.
    pub fn is_referenced(&self, name: &str) -> bool {
        self.is_imported(name)
            || self.referenced.is_some_and(|names| names.contains(name))
            || (self.star_imported && !name.starts_with('_'))
    }

    // Whether an `import *` takes every public name of the module, which adding an `__all__` would narrow.
    pub fn is_star_imported(&self) -> bool {
        self.star_imported
    }

    pub fn is_attribute_assigned(&self, name: &str) -> bool {
        self.project.assigned_attributes.contains(name)
    }

    pub fn is_base_class(&self, name: &str) -> bool {
        self.project.base_classes.contains(name)
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
        let name = class.rsplit('.').next().unwrap_or(class);
        let writes = &self.project.writes;
        writes
            .on_class
            .keys()
            .chain(&writes.unslottable_bases)
            .any(|other| other != class && other.rsplit('.').next() == Some(name))
    }

    // Whether the run sets or deletes an attribute with this name on an object of unknown class; mocks do not count.
    pub fn is_attribute_written(&self, name: &str) -> bool {
        self.project.writes.untyped.contains(name)
    }

    // Whether a class of the run that a slotted base breaks inherits from the class with this qualified name.
    pub fn has_unslottable_subclass(&self, name: &str) -> bool {
        self.project.writes.unslottable_bases.contains(name)
    }
}

fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// The files a file's names resolve to, with the names it uses from each, and the files it star-imports, or the last
// name of each star-imported module that resolves to no file.
struct ModuleFacts<'f> {
    path: PathBuf,
    rewrites_calls: bool,
    constructors: Vec<(PathBuf, &'f [NameUse])>,
    imports: Vec<(PathBuf, &'f [String])>,
    references: Vec<(PathBuf, &'f [String])>,
    stars: Vec<PathBuf>,
    unresolved_stars: Vec<&'f str>,
}

impl<'f> ModuleFacts<'f> {
    fn resolve(
        db: &ModuleDb,
        environment: ResolverEnvironment<'_>,
        path: &Path,
        facts: &'f FileFacts,
        rewrites_calls: bool,
    ) -> Self {
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
        let mut stars = Vec::new();
        let mut unresolved_stars = Vec::new();
        for module in &facts.star_imports {
            match resolve(db, environment, importing, module) {
                Some(file) => stars.extend((file != own).then_some(file)),
                None => unresolved_stars.extend(module.rsplit('.').next()),
            }
        }
        let constructors = facts
            .constructors
            .uses
            .iter()
            .filter_map(|(module, uses)| Some((resolve(db, environment, importing, module)?, uses.as_slice())))
            .filter(|(file, _)| *file != own)
            .collect();
        Self { path: own, rewrites_calls, constructors, imports, references, stars, unresolved_stars }
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
