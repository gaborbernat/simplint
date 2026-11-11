use std::path::{Path, PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rayon::prelude::*;
use ruff_linter::package::PackageRoot;
use ruff_python_ast::helpers::to_module_path;
use rustc_hash::{FxHashMap, FxHashSet};

use super::Resolved;
use super::conflicts::{self, Scope};
use super::digest::probed_layout_key;
use crate::lint::registry::{Rule, RuleSet};
use crate::project::{DocReferences, Parts, SearchRoots};
use crate::run::cache::FileIdentity;
use crate::settings::{FileSettings, ModuleRole, Settings};

// What a project of the run takes from its files: the entry points, the build hooks, the fewest statements a duplicate
// block spans, where each file sits in its library's API, and which files are test modules. A nested project is indexed
// on its own, so its files only take in the ones of their own group.
pub(super) struct ProjectInputs<'f> {
    pub(super) entry_points: Vec<(PathBuf, String)>,
    pub(super) build_hooks: Vec<(PathBuf, String)>,
    pub(super) coverage_plugins: Vec<(PathBuf, String)>,
    pub(super) coverage_paths: Vec<(PathBuf, PathBuf)>,
    pub(super) duplicate_statements: Option<usize>,
    pub(super) probed: Option<u64>,
    pub(super) roles: FxHashMap<&'f Path, ModuleRole>,
    pub(super) test_modules: FxHashSet<&'f Path>,
    pub(super) distribution_roots: FxHashMap<&'f Path, &'f Path>,
}

impl<'f> ProjectInputs<'f> {
    pub(super) fn of(files: &'f [RunFile<'f>], roots: &SearchRoots) -> Self {
        let linted: Vec<&RunFile> = files.iter().filter(|file| file.linted).collect();
        let settings: Vec<&Settings> = linted
            .iter()
            .map(|file| file.settings.project)
            .unique_by(|settings| std::ptr::from_ref(*settings))
            .collect();
        Self {
            entry_points: settings
                .iter()
                .flat_map(|settings| {
                    settings.metadata.entry_points.iter().map(|entry| (settings.project_dir.clone(), entry.clone()))
                })
                .sorted_unstable()
                .dedup()
                .collect(),
            build_hooks: settings
                .iter()
                .flat_map(|settings| settings.build_hooks.iter().cloned())
                .sorted_unstable()
                .dedup()
                .collect(),
            coverage_plugins: settings
                .iter()
                .flat_map(|settings| {
                    settings.coverage.modules.iter().map(|module| (settings.project_dir.clone(), module.clone()))
                })
                .sorted_unstable()
                .dedup()
                .collect(),
            coverage_paths: settings
                .iter()
                .flat_map(|settings| {
                    settings
                        .coverage
                        .search_paths
                        .iter()
                        .map(|directory| (settings.project_dir.clone(), directory.clone()))
                })
                .sorted_unstable()
                .dedup()
                .collect(),
            duplicate_statements: duplicate_window(&linted, rules_of(&linted)),
            probed: probed_layout_key(&linted),
            roles: module_roles(files, roots),
            test_modules: files
                .iter()
                .filter(|file| file.settings.is_test)
                .map(|file| file.absolute.as_path())
                .collect(),
            distribution_roots: files
                .iter()
                .map(|file| (file.absolute.as_path(), file.settings.project.project_dir.as_path()))
                .collect(),
        }
    }
}

// A file the run indexes, with what indexing and linting it take, worked out once: its settings and Ruff's with the
// configuration file they come from, its package, and the module it imports as from that package. Ruff sees no package
// around a file of a namespace package, yet Python resolves its relative imports, so such a file is `importable` from
// a search root instead. The run's files are indexed in groups by `nested` project, and `at` is the file's position
// among the run's paths, which `linted` tells apart from the files only indexed.
pub(super) struct RunFile<'a> {
    pub(super) path: &'a Path,
    // The path the index keys the file by. The run's paths may be relative to the working directory, so they are made
    // absolute once here rather than at every lookup.
    pub(super) absolute: PathBuf,
    pub(super) settings: FileSettings<'a>,
    pub(super) ruff: &'a ruff_workspace::Settings,
    pub(super) ruff_config: Option<&'a Path>,
    pub(super) package: Option<PackageRoot<'a>>,
    pub(super) module_path: Option<Vec<String>>,
    pub(super) importable: Option<Vec<String>>,
    pub(super) sibling_init: bool,
    pub(super) at: usize,
    pub(super) linted: bool,
    // The directory of the nested project the file is in, which the index of the run keeps apart from the rest.
    pub(super) nested: Option<PathBuf>,
}

impl<'a> RunFile<'a> {
    pub(super) fn identity(&self) -> FileIdentity<'_> {
        FileIdentity {
            settings: &self.settings,
            ruff: &self.ruff.linter,
            module_path: self.module_path.as_ref(),
            sibling_init: self.sibling_init,
        }
    }

    pub(super) fn new(
        path: &'a Path,
        settings: &'a Settings,
        ruff: &'a ruff_workspace::Settings,
        ruff_config: Option<&'a Path>,
        package: Option<PackageRoot<'a>>,
        roots: &SearchRoots,
    ) -> Self {
        let module_path = package.and_then(|package| to_module_path(package.path(), path));
        let importable = if module_path.is_none() { roots.module_path(path) } else { None };
        let settings = FileSettings::new(settings, ruff, path);
        // A namespace package has no `__init__.py`, which a fingerprint and a rule read only when a rule asks.
        let sibling_init =
            settings.rules.overlaps(RuleSet::sibling_init()) && path.with_file_name("__init__.py").is_file();
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let nested = settings.project.nested.then(|| settings.project.root.clone());
        Self {
            path,
            absolute,
            settings,
            ruff,
            ruff_config,
            package,
            module_path,
            importable,
            sibling_init,
            at: 0,
            linted: true,
            nested,
        }
    }
}

// The rules the run's files turn on, one union for every per-run check.
fn rules_of(files: &[&RunFile]) -> RuleSet {
    files.iter().fold(RuleSet::default(), |rules, file| rules.union(file.settings.rules))
}

// The facts the run's rules read, which only those rules' runs need collected.
fn wanted_parts(rules: RuleSet) -> Parts {
    rules.iter().fold(Parts::default(), |parts, rule| parts.with(rule.fact_parts()))
}

// What the linted files of a run turn on: their rules, and the facts those rules read.
pub(super) struct Plan {
    pub(super) rules: RuleSet,
    pub(super) parts: Parts,
}

impl Plan {
    pub(super) fn of(linted: &[&RunFile], resolved: &Resolved) -> Result<Self> {
        ensure_compatible(linted, resolved)?;
        let rules = rules_of(linted);
        Ok(Self { rules, parts: wanted_parts(rules) })
    }
}

// The fewest statements a duplicate block spans: each file reports the blocks its own setting allows, so the index finds
// the shortest any file asks for. `None` when no file checks for duplicate code, whose statements the facts then lack.
fn duplicate_window(files: &[&RunFile], rules: RuleSet) -> Option<usize> {
    if !rules.contains(Rule::DuplicateCode) {
        return None;
    }
    files.iter().map(|file| file.settings.project.knobs.min_duplicate_statements).min()
}

// What the documentation of each project names, by the nested project its files are in (`None` for the run's project). A
// project reads the pages under its own roots, so a name one project's pages give does not count in another.
pub(super) type Documentation = FxHashMap<Option<PathBuf>, DocReferences>;

// What the documentation of each project with a `pyproject.toml` names, read only when a rule that asks is on. The
// run's project `main` leaves the nested projects' pages to them.
pub(super) fn documentation(
    files: &[&RunFile],
    main: &Path,
    rules: RuleSet,
    cache_dir: Option<&Path>,
) -> Documentation {
    if !rules.overlaps(RuleSet::documentation()) {
        return Documentation::default();
    }
    let mut roots: FxHashMap<Option<PathBuf>, Vec<(&Path, &ruff_workspace::Settings)>> = FxHashMap::default();
    for file in files.iter().filter(|file| file.settings.project.path.is_some()) {
        roots.entry(file.nested.clone()).or_default().push((file.settings.project.root.as_path(), file.ruff));
    }
    roots
        .into_iter()
        .map(|(project, roots)| {
            let roots = roots.into_iter().unique_by(|(root, _)| *root);
            (project, DocReferences::collect(roots, main, cache_dir))
        })
        .collect()
}

fn ensure_compatible(files: &[&RunFile], resolved: &Resolved) -> Result<()> {
    let scopes = files
        .iter()
        .map(|file| Scope {
            path: file.path,
            settings: &file.settings,
            ruff: file.ruff,
            ruff_config: file.ruff_config,
            package: file.package,
        })
        .collect_vec();
    conflicts::ensure_compatible(&scopes, resolved.names_rules())
}

// Where each file sits in its library's public API, by the first of the names the file imports as, from its package or
// from a search root, that an API package holds. Unlinted files count too: an `__init__.py` Ruff's `lint.exclude` leaves
// out still re-exports public names.
fn module_roles<'s>(files: &'s [RunFile<'_>], roots: &SearchRoots) -> FxHashMap<&'s Path, ModuleRole> {
    files
        .par_iter()
        .map(|file| {
            let settings = file.settings.project;
            let module_paths = file.module_path.iter().cloned().chain(roots.module_paths(file.path)).collect_vec();
            let is_test = file.settings.is_test;
            let roles: Vec<ModuleRole> =
                module_paths.iter().map(|module_path| settings.api.role(Some(module_path), is_test)).collect();
            let role = roles.iter().copied().find(|role| role.public).or_else(|| roles.first().copied());
            (file.absolute.as_path(), role.unwrap_or_else(|| settings.api.role(None, is_test)))
        })
        .collect()
}
