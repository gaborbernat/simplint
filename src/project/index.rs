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
use super::facts::{ImportBinding, ModuleNames};
use crate::rules::Visibility;
use crate::settings::ModuleRole;

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
    // Modules their own package's `__init__.py` star-imports, where the package binds the submodule under its own name.
    package_star_imported: HashSet<PathBuf>,
    // Modules some file star-imports, whatever their `__all__`, so a name they export may be in use where it lands.
    star_sources: HashSet<PathBuf>,
    // Where each file sits in its library's public API.
    roles: HashMap<PathBuf, ModuleRole>,
    // The names of each file a library's users can reach: those its public module exports, and those a public module
    // re-exports, each with the file that re-exports it, if any.
    user_facing: HashMap<PathBuf, HashMap<String, Option<PathBuf>>>,
    assigned_attributes: HashSet<String>,
    module_reads: HashSet<String>,
    base_classes: HashSet<String>,
    writes: Writes,
    calls: DataclassCalls,
    // Whether any file of the run reads docstrings off objects, so any docstring may be data it reads.
    reads_docstrings: bool,
    digest: u64,
}

// The attribute writes of the run: by the qualified class of the instance written to when parsing tells, and otherwise
// by name.
#[derive(Default)]
struct Writes {
    on_class: HashMap<String, HashSet<String>>,
    untyped: HashSet<String>,
    unslottable_bases: HashSet<String>,
    stateful_bases: HashSet<String>,
    pickles_low_protocol: bool,
}

// What the index reads besides the files' facts: the entry points, where imports resolve, the files whose own
// `dataclass-kw-only` reports rewrite their positional dataclass calls, and where each file sits in its library's
// public API.
pub struct IndexInputs<'a> {
    pub entry_points: &'a [String],
    pub roots: &'a SearchRoots,
    pub rewrites_calls: &'a HashSet<&'a Path>,
    pub roles: &'a HashMap<&'a Path, ModuleRole>,
}

impl Project {
    pub fn index(files: &[(&Path, &FileFacts)], inputs: &IndexInputs) -> Result<Self> {
        let IndexInputs { entry_points, roots, rewrites_calls, roles } = *inputs;
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
        let (mut unresolved_stars, mut unreached): (HashSet<&str>, Vec<Unreached>) = (HashSet::new(), Vec::new());
        let mut constructor_uses = Vec::new();
        let mut bindings: Vec<(PathBuf, PathBuf, &ImportBinding)> = Vec::new();
        for module in modules {
            constructor_uses.push((module.path.clone(), module.rewrites_calls, module.constructors));
            bindings
                .extend(module.bindings.into_iter().map(|(source, binding)| (module.path.clone(), source, binding)));
            for (file, names) in module.imports {
                imported.entry(file).or_default().extend(names.iter().cloned());
            }
            for (file, names) in module.references {
                referenced.entry(file).or_default().extend(names.iter().cloned());
            }
            stars.extend(module.stars.into_iter().map(|source| (module.path.clone(), source)));
            unresolved_stars.extend(module.unresolved_stars);
            unreached.extend(module.unreached);
        }
        let exports: HashMap<PathBuf, Option<&[String]>> = files
            .iter()
            .filter(|(path, _)| path.extension().is_none_or(|extension| extension != "pyi"))
            .map(|(path, facts)| (normalized(path), facts.exports.as_deref()))
            .collect();
        reach_by_last_name(&mut referenced, &unreached, exports.keys());
        reexport(&mut imported, &stars, &exports);
        reexport(&mut referenced, &stars, &exports);
        let package_star_imported = stars
            .iter()
            .filter(|(importer, source)| source.parent().is_some_and(|parent| *importer == parent.join("__init__.py")))
            .map(|(_, source)| source.clone())
            .collect();
        let star_sources = stars.iter().map(|(_, source)| source.clone()).collect();
        let star_imported = stars
            .iter()
            .map(|(_, source)| source.clone())
            .chain(unresolved_candidates(exports.keys(), &unresolved_stars))
            .filter(|file| exports.get(file).is_some_and(Option::is_none))
            .collect();
        let assigned_attributes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.assigned_attributes.iter().cloned()).collect();
        let module_reads: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.module_reads.iter().cloned()).collect();
        let base_classes: HashSet<String> =
            files.iter().flat_map(|(_, facts)| facts.base_classes.iter().cloned()).collect();
        let writes = Writes::collect(files);
        let environment = ResolverEnvironment::new(&db, python_version, paths);
        for (module, attribute) in entry_points.iter().filter_map(|entry_point| entry_point.split_once(':')) {
            if let Some(file) = resolve(&db, environment, None, module.trim()) {
                let name = attribute.split('.').next().unwrap_or_default().trim();
                imported.entry(file).or_default().insert(name.to_owned());
            }
        }
        let declared = files.iter().map(|(path, facts)| (normalized(path), facts.constructors.dataclasses.as_slice()));
        let dynamic: HashSet<&str> =
            files.iter().flat_map(|(_, facts)| facts.constructors.dynamic.iter().map(String::as_str)).collect();
        let calls = DataclassCalls::index(declared, &constructor_uses, &imported, &dynamic);
        let roles: HashMap<PathBuf, ModuleRole> = roles.iter().map(|(path, role)| (normalized(path), *role)).collect();
        let user_facing = user_facing(files, &roles, &imported, &bindings, &stars);
        let reads_docstrings = files.iter().any(|(_, facts)| facts.reads_docstrings);
        let digest = digest(reads_docstrings, [&assigned_attributes, &module_reads, &base_classes], &writes);
        Ok(Self {
            imported,
            referenced,
            star_imported,
            package_star_imported,
            star_sources,
            roles,
            user_facing,
            assigned_attributes,
            module_reads,
            base_classes,
            writes,
            calls,
            reads_docstrings,
            digest,
        })
    }

    // The attribute, module read and base class names one index has and the other lacks, whose answers differ between
    // the two, and the names of the classes whose attribute writes, or subclasses that a slotted base breaks, differ,
    // which a dataclass of that name, or a namesake of it, reads. Low protocol pickling concerns every dataclass, so its
    // change names `dataclass`.
    pub fn changed_names<'p>(&'p self, other: &'p Self) -> HashSet<&'p str> {
        let (writes, others) = (&self.writes, &other.writes);
        let classes = writes
            .on_class
            .iter()
            .filter(|(class, attributes)| others.on_class.get(*class) != Some(*attributes))
            .chain(others.on_class.iter().filter(|(class, _)| !writes.on_class.contains_key(*class)))
            .map(|(class, _)| class)
            .chain(writes.unslottable_bases.symmetric_difference(&others.unslottable_bases))
            .chain(writes.stateful_bases.symmetric_difference(&others.stateful_bases))
            .map(|class| class.rsplit('.').next().unwrap_or(class));
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
    pub fn file(&self, path: &Path) -> FileIndex<'_> {
        let path = normalized(path);
        FileIndex {
            project: self,
            imported: self.imported.get(&path),
            referenced: self.referenced.get(&path),
            star_imported: self.star_imported.contains(&path),
            package_star_imported: self.package_star_imported.contains(&path),
            star_source: self.star_sources.contains(&path),
            role: self.roles.get(&path).copied().unwrap_or_default(),
            user_facing: self.user_facing.get(&path),
            calls: self.calls.file(&path),
        }
    }
}

// The names of the whole run that every file's answers depend on: what the run's attribute writes, module reads and base
// classes are, and whether it reads docstrings.
fn digest(
    reads_docstrings: bool,
    [assigned, module_reads, base_classes]: [&HashSet<String>; 3],
    writes: &Writes,
) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    reads_docstrings.cache_key(&mut hasher);
    assigned.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
    0u8.cache_key(&mut hasher);
    module_reads.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
    base_classes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
    for (class, attributes) in writes.on_class.iter().sorted_by(|left, right| left.0.cmp(right.0)) {
        class.cache_key(&mut hasher);
        attributes.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
    }
    for names in [&writes.untyped, &writes.unslottable_bases, &writes.stateful_bases] {
        0u8.cache_key(&mut hasher);
        names.iter().sorted().for_each(|name: &String| name.cache_key(&mut hasher));
    }
    writes.pickles_low_protocol.cache_key(&mut hasher);
    hasher.finish()
}

impl Writes {
    fn collect(files: &[(&Path, &FileFacts)]) -> Self {
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
        writes
    }
}

// The names a library's users reach. A public module offers what its `__all__` lists, or, without one, every name it
// defines or re-exports without a leading underscore, as the typing spec reads a module; a name another module imports
// from it counts too, since `missing-from-all` lists it there. A module that re-exports a public name, through
// `__all__`, a redundant alias or `import *`, makes it public where it is defined, through any chain of re-exports.
// Names only spread, so this settles.
fn user_facing(
    files: &[(&Path, &FileFacts)],
    roles: &HashMap<PathBuf, ModuleRole>,
    imported: &HashMap<PathBuf, HashSet<String>>,
    bindings: &[(PathBuf, PathBuf, &ImportBinding)],
    stars: &[(PathBuf, PathBuf)],
) -> HashMap<PathBuf, HashMap<String, Option<PathBuf>>> {
    let facts: HashMap<PathBuf, &FileFacts> = files
        .iter()
        .filter(|(path, _)| path.extension().is_none_or(|extension| extension != "pyi"))
        .map(|(path, facts)| (normalized(path), *facts))
        .collect();
    // What `import *` of a module takes: its `__all__`, or every name it defines without a leading underscore.
    // What `from module import *` takes from a module without `__all__`.
    let starred = |name: &&str| Visibility::of(name) == Visibility::Public;
    let star_exports = |path: &Path| -> Vec<&str> {
        facts.get(path).map_or_else(Vec::new, |facts| match &facts.exports {
            Some(listed) => listed.iter().map(String::as_str).collect(),
            None => facts.defined.iter().map(String::as_str).filter(starred).collect(),
        })
    };
    let mut names: HashMap<PathBuf, HashMap<String, Option<PathBuf>>> = HashMap::new();
    for (path, facts) in &facts {
        if !roles.get(path).is_some_and(|role| role.public) {
            continue;
        }
        let own = names.entry(path.clone()).or_default();
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
                    star_exports(source).into_iter().for_each(&mut add);
                }
            }
        }
        imported.get(path).into_iter().flatten().map(String::as_str).filter(starred).for_each(&mut add);
    }
    loop {
        let mut reached: Vec<(PathBuf, String, PathBuf)> = Vec::new();
        for (importer, source, binding) in bindings {
            if names.get(importer).is_some_and(|known| known.contains_key(&binding.bound)) {
                reached.push((source.clone(), binding.name.clone(), importer.clone()));
            }
        }
        for (importer, source) in stars {
            let Some(known) = names.get(importer) else {
                continue;
            };
            let exported = star_exports(source);
            reached.extend(
                exported
                    .into_iter()
                    .filter(|name| known.contains_key(*name))
                    .map(|name| (source.clone(), name.to_owned(), importer.clone())),
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
            return names;
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
    files.filter(|file| last_name(file).is_some_and(|name| unresolved.contains(name))).cloned().collect()
}

// A dotted path through a package that names no module, as `nx.reportviews.Name` where `networkx` re-exports its
// `networkx.classes.reportviews` submodule, may reach any module of that package with the path's last name. Those
// names count as referenced rather than imported: they are kept, yet not made part of the module's API.
fn reach_by_last_name<'p>(
    referenced: &mut HashMap<PathBuf, HashSet<String>>,
    unreached: &[Unreached],
    files: impl Iterator<Item = &'p PathBuf>,
) {
    if unreached.is_empty() {
        return;
    }
    let mut by_name: HashMap<&str, Vec<&PathBuf>> = HashMap::new();
    for file in files {
        if let Some(name) = last_name(file) {
            by_name.entry(name).or_default().push(file);
        }
    }
    for (package, name, names) in unreached {
        for file in by_name.get(name).into_iter().flatten().filter(|file| file.starts_with(package)) {
            referenced.entry((*file).clone()).or_default().extend(names.iter().cloned());
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
    imported: Option<&'a HashSet<String>>,
    referenced: Option<&'a HashSet<String>>,
    star_imported: bool,
    package_star_imported: bool,
    star_source: bool,
    role: ModuleRole,
    user_facing: Option<&'a HashMap<String, Option<PathBuf>>>,
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
            other.calls,
            other.project.reads_docstrings,
        )
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
        self.package_star_imported.cache_key(state);
        self.star_source.cache_key(state);
        self.role.cache_key(state);
        for (name, importer) in self.user_facing.into_iter().flatten().sorted() {
            name.cache_key(state);
            importer.cache_key(state);
        }
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

fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// A dotted path that resolves to no module, by the directory of the package of the run it starts from and its last
// name, with the names it uses.
type Unreached<'f> = (PathBuf, &'f str, &'f [String]);

// The files a file's names resolve to, with the names it uses from each, and the files it star-imports, or the last
// name of each star-imported module that resolves to no file.
struct ModuleFacts<'f> {
    path: PathBuf,
    rewrites_calls: bool,
    constructors: Vec<(PathBuf, &'f [NameUse])>,
    imports: Vec<(PathBuf, &'f [String])>,
    bindings: Vec<(PathBuf, &'f ImportBinding)>,
    references: Vec<(PathBuf, &'f [String])>,
    stars: Vec<PathBuf>,
    unresolved_stars: Vec<&'f str>,
    unreached: Vec<Unreached<'f>>,
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
        let unreached = facts
            .imports
            .iter()
            .chain(&facts.references)
            .filter_map(|(module, names)| {
                let (top, rest) = module.split_once('.')?;
                let package = resolve(db, environment, importing, top)?;
                let directory = package.parent().filter(|_| package.ends_with("__init__.py"))?.to_path_buf();
                let last = rest.rsplit('.').next()?;
                resolve(db, environment, importing, module).is_none().then_some((directory, last, names.as_slice()))
            })
            .collect();
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
        let bindings = facts
            .bindings
            .iter()
            .filter_map(|binding| Some((resolve(db, environment, importing, &binding.module)?, binding)))
            .filter(|(file, _)| *file != own)
            .collect();
        Self {
            path: own,
            rewrites_calls,
            constructors,
            imports,
            bindings,
            references,
            stars,
            unresolved_stars,
            unreached,
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
