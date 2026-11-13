mod api;
mod file;
mod metadata;
mod options;
mod pytest;
mod schema;

use std::collections::BTreeMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use globset::{Candidate, Glob, GlobMatcher};
use itertools::Itertools;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_diagnostics::{Applicability, Fix};
use ruff_linter::settings::types::GlobPath;
use ruff_linter::warn_user_once_by_message;
use ruff_macros::CacheKey;
use ruff_python_ast::PythonVersion;
use rustc_hash::FxHashMap;
use serde::Deserialize;

pub use api::{ApiScope, DocstringStyle, ModuleRole, ProjectKind};
pub use file::{FileSettings, FromRuff};
pub use options::{Configuration, Overrides, RunOptions, SimplintOptions, Source, ruff_selector};
pub use schema::config_schema;

use crate::lint::registry::{Rule, RuleSet};
use crate::settings::metadata::{
    BuildConfiguration, CoveragePlugins, ProjectMetadata, VersionReaders, build_hooks, declared_python, hatch_hooks,
    tool_value,
};
pub(crate) use crate::settings::metadata::{canonical_name, may_provide};
use crate::settings::options::{PerFileIgnoreSource, ProjectOptions, Selector, SelectorLayer};
pub(crate) use crate::settings::pytest::PytestConfig;

// Where `[tool.simplint]` and the project metadata come from: nowhere with `--isolated`, the `--config` file when it is
// a `pyproject.toml`, and otherwise the `pyproject.toml`s closest to each file.
pub(crate) struct SettingsResolver {
    pub(crate) root: Arc<Settings>,
    overrides: Overrides,
    discovery: Discovery,
}

impl SettingsResolver {
    pub(crate) fn new(cwd: &Path, config_file: Option<&Path>, isolated: bool, overrides: Overrides) -> Result<Self> {
        // A relative `--config` would anchor per-file ignore patterns at an empty directory.
        let config_file = config_file.map(|config| {
            std::path::absolute(cwd.join(config)).expect("a path under the working directory is not empty")
        });
        let (root, pyproject) = match config_file.as_deref() {
            _ if isolated => (cwd, None),
            Some(config) => (
                config.parent().unwrap_or(cwd),
                Some(config).filter(|config| config.file_name().is_some_and(|name| name == "pyproject.toml")),
            ),
            None => {
                let discovery = Discovery::Hierarchical { run_root: cwd.to_path_buf(), projects: Mutex::default() };
                let root = discovery.settings(cwd, &overrides)?;
                discovery.set_main(&root.root);
                return Ok(Self { root, overrides, discovery });
            }
        };
        let pyproject = pyproject.map(|path| read_table(path).map(|table| (path, table))).transpose()?;
        // The one file is both: a `--config` run takes its metadata from the same file, as it takes its settings.
        let table = pyproject.as_ref().map(|(path, table)| (*path, table));
        let fixed = Arc::new(Settings::resolve(root, None, table, table, &overrides, isolated)?);
        let mut projects = Projects { main: Some(fixed.root.clone()), ..Projects::default() };
        projects.resolved.insert((fixed.root.clone(), fixed.metadata_path.clone()), Arc::clone(&fixed));
        // A nested project reads its settings from the same file, which needs no second read.
        projects.pyprojects.extend(pyproject.map(|(path, table)| (path.to_path_buf(), table)));
        let discovery = Discovery::Fixed { settings: Arc::clone(&fixed), projects: Mutex::new(projects), isolated };
        Ok(Self { root: fixed, overrides, discovery })
    }

    pub(crate) fn for_file(&self, path: &Path) -> Result<Arc<Settings>> {
        self.discovery.settings(path, &self.overrides)
    }
}

fn read_table(path: &Path) -> Result<toml::Table> {
    let text = std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("Failed to parse {}", path.display()))
}

// Ruff's `PyprojectDiscoveryStrategy`: one configuration for every file, or the one each file's location points at,
// read once per project. A file under a nested project resolves to that project's directory as its root.
enum Discovery {
    // With `--config`, the settings of the run's project, which a file of a nested project overrides. With
    // `--isolated`, no `pyproject.toml` is read, nested projects' included.
    Fixed { settings: Arc<Settings>, projects: Mutex<Projects>, isolated: bool },
    // `run_root` is the directory the run starts from, which roots a project without a `pyproject.toml`.
    Hierarchical { run_root: PathBuf, projects: Mutex<Projects> },
}

impl Discovery {
    // The closest `pyproject.toml` with a `[tool.simplint]` table holds the settings, and the closest with `[project]`
    // or `[build-system]` holds the project's metadata. Each falls back to the closest `pyproject.toml` when nothing
    // matches, so a nested package without its own table inherits its parent's settings. Without any
    // `pyproject.toml`, the defaults apply, rooted at the run's directory or the nearest ancestor of it holding the
    // file, as Ruff roots a project without configuration at the working directory.
    //
    // A nested project is rooted at its directory, with the settings of the project around it, and the metadata of a
    // `pyproject.toml` at or below it that describes a project, or else the metadata of the project around it. Settings
    // are cached by the directory a lookup starts from, and by the root and metadata file they resolve to, since files
    // of one root can have different metadata.
    fn settings(&self, path: &Path, overrides: &Overrides) -> Result<Arc<Settings>> {
        let absolute = std::path::absolute(path).expect("a checked path is not empty");
        let start = if absolute.is_dir() { absolute.as_path() } else { absolute.parent().unwrap_or(&absolute) };
        let mut projects = self.projects().lock().expect("no worker panics while resolving settings");
        if let Some(settings) = projects.settings.get(start) {
            return Ok(Arc::clone(settings));
        }
        let (settings_file, metadata_file, root) = match self {
            Self::Fixed { settings, .. } => {
                (settings.path.clone(), settings.metadata_path.clone(), settings.root.clone())
            }
            Self::Hierarchical { run_root, .. } => {
                let [configured, closest, described] =
                    projects.nearest(start, None, [|table| configures(table, "simplint"), |_| true, describes])?;
                let settings_file = configured.or(closest);
                let metadata_file = described.or_else(|| settings_file.clone());
                let root = match settings_file.as_deref().and_then(Path::parent) {
                    Some(root) => root.to_path_buf(),
                    None => run_root.ancestors().find(|dir| start.starts_with(dir)).unwrap_or(start).to_path_buf(),
                };
                (settings_file, metadata_file, root)
            }
        };
        let nested = projects.nested(start);
        let (root, metadata_file) = match &nested {
            Some(project) => {
                let [own] =
                    if self.is_isolated() { [None] } else { projects.nearest(start, Some(project), [describes])? };
                (project.clone(), own.or(metadata_file))
            }
            None => (root, metadata_file),
        };
        let key = (root.clone(), metadata_file.clone());
        let settings = if let Some(settings) = projects.resolved.get(&key) {
            Arc::clone(settings)
        } else {
            let configuration = settings_file.as_deref().map(|file| (file, &projects.pyprojects[file]));
            let metadata = metadata_file.as_deref().map(|file| (file, &projects.pyprojects[file]));
            let mut resolved =
                Settings::resolve(&root, projects.main.as_deref(), configuration, metadata, overrides, false)?;
            resolved.nested = nested.is_some();
            Arc::new(resolved)
        };
        projects.resolved.insert(key, Arc::clone(&settings));
        projects.settings.insert(start.to_path_buf(), Arc::clone(&settings));
        Ok(settings)
    }

    // Makes `main` the run's project, which a nested project lies strictly below. The settings found so far are
    // dropped, since directories below `main` may have resolved as projects around it.
    fn set_main(&self, main: &Path) {
        let mut projects = self.projects().lock().expect("no worker panics while resolving settings");
        projects.main = Some(main.to_path_buf());
        projects.settings.clear();
    }

    fn projects(&self) -> &Mutex<Projects> {
        match self {
            Self::Fixed { projects, .. } | Self::Hierarchical { projects, .. } => projects,
        }
    }

    fn is_isolated(&self) -> bool {
        matches!(self, Self::Fixed { isolated: true, .. })
    }
}

// Whether a `pyproject.toml` has a `[tool.<name>]` table.
fn configures(table: &toml::Table, name: &str) -> bool {
    tool_value(table, &[name]).is_some()
}

// Whether a `pyproject.toml` describes a project: a `[project]` table, or a `[build-system]` that installs it.
fn describes(table: &toml::Table) -> bool {
    table.contains_key("project") || table.contains_key("build-system")
}

// What discovery found so far: the settings by the directories they apply to, the settings each root and metadata file
// resolves to, and each `pyproject.toml` it met, so every one is read and parsed once.
#[derive(Default)]
struct Projects {
    // The run's project, which a nested project lies strictly below.
    main: Option<PathBuf>,
    settings: FxHashMap<PathBuf, Arc<Settings>>,
    resolved: FxHashMap<(PathBuf, Option<PathBuf>), Arc<Settings>>,
    pyprojects: FxHashMap<PathBuf, toml::Table>,
}

impl Projects {
    // The directory of the nested project `start` is in: the nearest directory below the run's project that
    // `is_nested_root` accepts.
    fn nested(&self, start: &Path) -> Option<PathBuf> {
        let main = self.main.as_deref()?;
        start
            .ancestors()
            .take_while(|dir| *dir != main && dir.starts_with(main))
            .find(|dir| is_nested_root(main, dir))
            .map(Path::to_path_buf)
    }

    // For each of `matches`, the closest `pyproject.toml` from `start` upwards, down to `limit` when there is one,
    // whose table it accepts. One walk answers them all, and stops once each has its file, so no file is read twice.
    fn nearest<const N: usize>(
        &mut self,
        start: &Path,
        limit: Option<&Path>,
        matches: [fn(&toml::Table) -> bool; N],
    ) -> Result<[Option<PathBuf>; N]> {
        let candidates = start
            .ancestors()
            .take_while(|dir| limit.is_none_or(|limit| dir.starts_with(limit)))
            .map(|dir| dir.join("pyproject.toml"))
            .filter(|candidate| candidate.is_file());
        let mut found: [Option<PathBuf>; N] = [const { None }; N];
        for candidate in candidates {
            let table = self.table(&candidate)?;
            for (slot, matches) in found.iter_mut().zip(matches) {
                if slot.is_none() && matches(table) {
                    *slot = Some(candidate.clone());
                }
            }
            if found.iter().all(Option::is_some) {
                break;
            }
        }
        Ok(found)
    }

    // The table of a `pyproject.toml`, read on first use, so each file is read once.
    fn table(&mut self, path: &Path) -> Result<&toml::Table> {
        match self.pyprojects.entry(path.to_path_buf()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => Ok(entry.insert(read_table(path)?)),
        }
    }
}

// Whether a directory is a nested project of the run's project `main`: a test directory below it that holds a
// `conf.py`, `pyproject.toml`, `setup.py` or `setup.cfg`. Documentation reads the same test directories, so a page
// under one belongs to its project.
pub(crate) fn is_nested_root(main: &Path, dir: &Path) -> bool {
    dir.strip_prefix(main).is_ok_and(in_nested_test_directory)
        && ["conf.py", "pyproject.toml", "setup.py", "setup.cfg"].iter().any(|marker| dir.join(marker).is_file())
}

// Whether a path, relative to a project, has a directory that marks a nested test project: `tests` or `test`, or a
// `test-*` or `test_*` directory such as the roots of Sphinx's test suite. A name that only starts with `test`, such as
// `testkit`, is a library's.
fn in_nested_test_directory(relative: &Path) -> bool {
    in_test_directory(relative)
        || relative.components().any(|part| {
            part.as_os_str().to_str().is_some_and(|name| name.starts_with("test-") || name.starts_with("test_"))
        })
}

// Whether a path, relative to a project, has a directory named `tests` or `test`, where the project keeps its tests.
fn in_test_directory(relative: &Path) -> bool {
    relative.components().any(|part| matches!(part.as_os_str().to_str(), Some("tests" | "test")))
}

// simplint's own options and the project metadata its rules read. The Python version, the files to check and what to
// exclude come from Ruff's configuration instead, resolved the way `ruff check` resolves them.
pub(crate) struct Settings {
    pub(crate) root: PathBuf,
    // Whether the root is a nested project of the run, whose files are test input for the rules that judge docstrings
    // and the visibility of names.
    pub(crate) nested: bool,
    pub(crate) path: Option<PathBuf>,
    pub metadata: ProjectMetadata,
    // The `pyproject.toml` the metadata and build settings come from, which can differ from `path` in a workspace.
    metadata_path: Option<PathBuf>,
    // The directory of the metadata's `pyproject.toml`, which can sit above `root` in a workspace.
    pub(crate) project_dir: PathBuf,
    // The build backend's hooks, each with the file that defines it.
    pub(crate) build_hooks: Vec<(PathBuf, String)>,
    // The files hatchling loads its custom build and metadata hooks from, whose hook class no import reaches.
    pub(crate) hatch_hooks: Vec<PathBuf>,
    // The oldest Python the project declares where Ruff does not look, which stands in for Ruff's default target.
    pub(crate) declared_python: Option<PythonVersion>,
    // The coverage plugins and the directories they resolve from, which the index searches for them.
    pub coverage: CoveragePlugins,
    // Where the build or release tooling reads `__version__` from.
    pub(crate) version_readers: VersionReaders,
    // The markers pytest registers and the file names it collects tests from.
    pub(crate) pytest: PytestConfig,
    pub(crate) run: RunOptions,
    pub knobs: Knobs,
    pub api: ApiScope,
    pub(crate) docstring_style: Option<DocstringStyle>,
    rules: RuleSet,
    fixable: RuleSet,
    fix_safety: FixSafetyTable,
    per_file_ignores: Vec<PerFileIgnore>,
}

impl Settings {
    // Ruff's `Configuration::into_settings`: the command line goes on top of the `pyproject.toml`'s options, then the
    // rule selections resolve in order. The configuration's `[tool.simplint]` gives the options; the metadata's
    // `[project]` and build settings give the project. Without a `pyproject.toml`, the defaults apply.
    fn resolve(
        root: &Path,
        main: Option<&Path>,
        configuration: Option<(&Path, &toml::Table)>,
        metadata: Option<(&Path, &toml::Table)>,
        overrides: &Overrides,
        isolated: bool,
    ) -> Result<Self> {
        let configures_flake8_lazy =
            [metadata, configuration].into_iter().flatten().any(|(_, table)| configures(table, "flake8-lazy"));
        let pyproject = metadata.map(|(_, table)| table);
        let (metadata_path, mut metadata, build) = match metadata {
            Some((path, table)) => {
                let hatch_file = path.with_file_name("hatch.toml");
                let build = BuildConfiguration::of(
                    table,
                    hatch_file.is_file().then(|| optional(&hatch_file, read_table)).flatten().as_ref(),
                );
                let mut project: ProjectFile =
                    table.clone().try_into().with_context(|| format!("Failed to parse {}", path.display()))?;
                project.project.add_groups(&project.dependency_groups);
                project.project.add_tool_references(table);
                project.project.native = build.native;
                (Some(path), project.project, build)
            }
            None => (None, ProjectMetadata::default(), BuildConfiguration::default()),
        };
        let path = configuration.map(|(path, _)| path);
        let configuration = match configuration {
            Some((path, table)) => {
                let pyproject: Pyproject =
                    table.clone().try_into().with_context(|| format!("Failed to parse {}", path.display()))?;
                Configuration::from_options(pyproject.tool.simplint, root, &Source::File(Arc::from(path)))
            }
            None => Configuration::default(),
        };
        let configuration = overrides.transform(configuration);
        // Its `setup.py` and the packages next to it belong to the metadata's project, which may sit above `root`.
        let project_dir = metadata_path.and_then(Path::parent).unwrap_or(root);
        let pytest = PytestConfig::of(&[root, project_dir], pyproject);
        metadata.entry_points.extend(pytest.plugins.iter().cloned());
        let coverage = pyproject
            .map_or_else(CoveragePlugins::default, |table| CoveragePlugins::of(table, project_dir, &pytest.pythonpath));
        let rules = resolve_rules(
            configuration.rule_selections.iter().map(|selection| &selection.enabled),
            RuleSet::all().subtract(RuleSet::from_rules([Rule::UnusedIgnore])),
        )?;
        // flake8-lazy rebuilds each module's `__lazy_modules__` from its imports, so it and this rule's fix keep
        // rewriting each other's output; the rule stays off for the whole project. The hook configuration is read only
        // when the rule would run. Without a `pyproject.toml`, the run's root holds the project, unless `--isolated`
        // reads no files.
        let project = metadata_path.and_then(Path::parent).or_else(|| (!isolated).then_some(root));
        let rules = if rules.contains(Rule::AbsoluteSiblingImport)
            && (configures_flake8_lazy || hook_runs_flake8_lazy(project, main))
        {
            rules.subtract(RuleSet::from_rules([Rule::AbsoluteSiblingImport]))
        } else {
            rules
        };
        let fixable =
            resolve_rules(configuration.rule_selections.iter().map(|selection| &selection.fixable), RuleSet::all())?;
        let fix_safety =
            FixSafetyTable::from_selectors(&configuration.extend_safe_fixes, &configuration.extend_unsafe_fixes)?;
        let options = configuration.project;
        let api = ApiScope::resolve(
            project_dir,
            options.project_kind.unwrap_or_default(),
            &build,
            &metadata,
            options.api_packages.as_deref(),
        );
        Ok(Self {
            root: root.to_path_buf(),
            nested: false,
            path: path.map(Path::to_path_buf),
            metadata_path: metadata_path.map(Path::to_path_buf),
            project_dir: project_dir.to_path_buf(),
            // `backend-path` is relative to the `pyproject.toml` that declares the backend.
            build_hooks: build_hooks(project_dir, &build),
            hatch_hooks: hatch_hooks(project_dir, &build),
            version_readers: project
                .map_or_else(VersionReaders::default, |directory| VersionReaders::of(directory, pyproject)),
            declared_python: if isolated { None } else { declared_python(project_dir, pyproject) },
            coverage,
            pytest,
            metadata,
            api,
            docstring_style: options.docstring_style,
            run: configuration.run,
            knobs: Knobs::resolve(options),
            rules,
            fixable,
            fix_safety,
            per_file_ignores: configuration
                .per_file_ignores
                .unwrap_or_default()
                .into_iter()
                .chain(configuration.extend_per_file_ignores)
                .map(|source| PerFileIgnore::compile(&source))
                .collect::<Result<_>>()?,
        })
    }

    fn rules_for(&self, path: &Path) -> RuleSet {
        let rules = if self.nested { self.rules.subtract(RuleSet::skipped_in_nested()) } else { self.rules };
        let absolute = std::path::absolute(path).unwrap_or_default();
        self.per_file_ignores
            .iter()
            .filter(|ignore| ignore.matches(&absolute))
            .fold(rules, |rules, ignore| rules.subtract(ignore.rules))
    }

    // Ruff drops the fix of a rule `fixable` leaves out, and `extend-safe-fixes` and `extend-unsafe-fixes` change how
    // safe the rest are.
    pub(crate) fn fix_for(&self, rule: Rule, fix: Option<Fix>) -> Option<Fix> {
        let fix = fix.filter(|_| self.fixable.contains(rule))?;
        let applicability = self.fix_safety.resolve_applicability(rule, fix.applicability());
        Some(fix.with_applicability(applicability))
    }

    pub(crate) fn is_test_module(&self, path: &Path) -> bool {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        // A nested project rooted at a test directory, such as `tools/swig/test` with its own `setup.py`, has its files
        // below that directory, which the path relative to the root no longer names.
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || name == "conftest.py"
            || self.pytest.collects_path(path)
            || in_test_directory(path.strip_prefix(&self.root).unwrap_or(path))
            || (self.nested && self.root.file_name().is_some_and(|root| in_test_directory(Path::new(root))))
    }

    // The rule tables `--show-settings` lists: enabled, fixable, forced safe and forced unsafe.
    pub(crate) fn rule_sets(&self) -> [RuleSet; 4] {
        [self.rules, self.fixable, self.fix_safety.forced_safe, self.fix_safety.forced_unsafe]
    }

    pub(crate) fn per_file_ignores(&self) -> impl Iterator<Item = (&str, RuleSet)> {
        self.per_file_ignores.iter().map(|ignore| (ignore.pattern.as_str(), ignore.rules))
    }
}

// The part of a file's settings its whole project shares that its diagnostics depend on, so a cached result is reused
// only under the same; the rules and Ruff's decisions for the file add the rest.
impl CacheKey for Settings {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.root.cache_key(state);
        self.nested.cache_key(state);
        self.metadata_path.cache_key(state);
        self.metadata.name.cache_key(state);
        self.metadata.dependencies.iter().for_each(|dependency| dependency.cache_key(state));
        self.metadata.declared.iter().for_each(|dependency| dependency.cache_key(state));
        self.metadata.runtime.iter().for_each(|dependency| dependency.cache_key(state));
        self.metadata.lists_dependencies.cache_key(state);
        for (name, modules) in &self.metadata.requirements {
            name.cache_key(state);
            modules.cache_key(state);
        }
        self.metadata.entry_points.cache_key(state);
        self.metadata.scripts.cache_key(state);
        self.metadata.native.cache_key(state);
        self.build_hooks.cache_key(state);
        self.hatch_hooks.cache_key(state);
        self.version_readers.files.cache_key(state);
        self.version_readers.any_init.cache_key(state);
        self.declared_python.cache_key(state);
        self.coverage.modules.cache_key(state);
        self.coverage.search_paths.cache_key(state);
        self.pytest.cache_key(state);
        self.fixable.cache_key(state);
        self.fix_safety.forced_safe.cache_key(state);
        self.fix_safety.forced_unsafe.cache_key(state);
        self.knobs.cache_key(state);
        self.api.cache_key(state);
        self.docstring_style.map(|style| style.to_string()).cache_key(state);
    }
}

// Ruff's `as_rule_table`: each layer applies its selectors from the least to the most specific, an ignore beating a
// select of the same specificity, and a layer that sets `select` (or `fixable`) starts from nothing.
#[derive(Deserialize, Default)]
#[serde(default)]
struct ProjectFile {
    project: ProjectMetadata,
    #[serde(rename = "dependency-groups")]
    dependency_groups: BTreeMap<String, Vec<toml::Value>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Pyproject {
    tool: Tool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Tool {
    simplint: SimplintOptions,
}

fn resolve_rules<'a>(layers: impl IntoIterator<Item = &'a SelectorLayer>, default: RuleSet) -> Result<RuleSet> {
    layers.into_iter().try_fold(default, |rules, layer| {
        let enable = layer.replace.iter().flatten().chain(&layer.extend).map(|selector| (selector, true));
        let updates: Vec<(usize, RuleSet, bool)> = enable
            .chain(layer.remove.iter().map(|selector| (selector, false)))
            .map(|(selector, enabled)| selector.resolve().map(|(set, specificity)| (specificity, set, enabled)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .sorted_by_key(|(specificity, ..)| *specificity)
            .collect();
        let base = if layer.replace.is_some() { RuleSet::default() } else { rules };
        Ok(updates
            .into_iter()
            .fold(base, |rules, (_, set, enabled)| if enabled { rules.union(set) } else { rules.subtract(set) }))
    })
}

// Whether the nearest hook configuration from `project` upwards has a flake8-lazy hook. pre-commit and prek read it at
// the repository root, so the search stops there. Outside a repository it stops at the run's project, `main`, since a
// configuration further up, such as one in the home directory, belongs to no hook run of this project.
fn hook_runs_flake8_lazy(project: Option<&Path>, main: Option<&Path>) -> bool {
    let Some(project) = project else {
        return false;
    };
    let top = project
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .or_else(|| main.filter(|main| project.starts_with(main)))
        .unwrap_or(project);
    for dir in project.ancestors() {
        let configs = HOOK_CONFIGS.iter().map(|name| dir.join(name)).filter(|path| path.is_file()).collect_vec();
        if !configs.is_empty() {
            return configs.iter().any(|config| optional(config, config_has_flake8_lazy).unwrap_or(false));
        }
        if dir == top {
            break;
        }
    }
    false
}

// The hook configuration files pre-commit and prek read from a project, which the nearest one upwards decides.
pub(crate) const HOOK_CONFIGS: [&str; 3] = [".pre-commit-config.yaml", ".pre-commit-config.yml", "prek.toml"];

fn config_has_flake8_lazy(path: &Path) -> Result<bool> {
    let bytes = std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    if path.extension().is_some_and(|extension| extension == "toml") {
        let table: toml::Table =
            toml::from_str(&text).with_context(|| format!("Failed to parse {}", path.display()))?;
        return Ok(table
            .get("repos")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|repo| repo.get("hooks")?.as_array())
            .flatten()
            .any(|hook| hook.get("id").and_then(toml::Value::as_str) == Some("flake8-lazy")));
    }
    Ok(text.lines().filter_map(hook_id).any(|id| id == "flake8-lazy"))
}

// The id a line of a pre-commit configuration gives a hook, as in `  - id: flake8-lazy  # lazy imports`. The line scan
// does not read flow-style hooks, such as `hooks: [{id: flake8-lazy}]`.
fn hook_id(line: &str) -> Option<&str> {
    let entry = line.trim_start().trim_start_matches('-').trim_start();
    let value = entry.strip_prefix("id:")?;
    let value = value
        .char_indices()
        .find(|(index, character)| *character == '#' && value[..*index].ends_with([' ', '\t']))
        .map_or(value, |(index, _)| &value[..index]);
    Some(value.trim().trim_matches(['"', '\'']))
}

// A file another tool owns, which simplint only reads for hints, so one it cannot read or parse counts as absent.
fn optional<T>(path: &Path, read: impl FnOnce(&Path) -> Result<T>) -> Option<T> {
    read(path)
        .inspect_err(|error| {
            warn_user_once_by_message!("Ignoring {}: {error:#}", path.file_name().unwrap_or_default().display());
        })
        .ok()
}

// Ruff's `FixSafetyTable` for simplint's rules: when both name a rule, the more specific selector wins, and the unsafe
// one on a tie.
#[derive(Clone, Copy)]
struct FixSafetyTable {
    forced_safe: RuleSet,
    forced_unsafe: RuleSet,
}

impl FixSafetyTable {
    fn from_selectors(safe: &[Selector], unsafe_: &[Selector]) -> Result<Self> {
        let mut overrides: BTreeMap<Rule, (usize, bool)> = BTreeMap::new();
        for (selector, is_safe) in
            safe.iter().map(|selector| (selector, true)).chain(unsafe_.iter().map(|selector| (selector, false)))
        {
            let (rules, specificity) = selector.resolve()?;
            for rule in rules.iter() {
                let entry = overrides.entry(rule).or_insert((specificity, is_safe));
                if specificity >= entry.0 {
                    *entry = (specificity, is_safe);
                }
            }
        }
        let forced = |safe: bool| {
            RuleSet::from_rules(overrides.iter().filter(|(_, (_, is_safe))| *is_safe == safe).map(|(rule, _)| *rule))
        };
        Ok(Self { forced_safe: forced(true), forced_unsafe: forced(false) })
    }

    // A display-only fix never applies, so its applicability stays.
    fn resolve_applicability(self, rule: Rule, applicability: Applicability) -> Applicability {
        let forced = if self.forced_unsafe.contains(rule) {
            Some(Applicability::Unsafe)
        } else {
            self.forced_safe.contains(rule).then_some(Applicability::Safe)
        };
        forced.filter(|_| applicability != Applicability::DisplayOnly).unwrap_or(applicability)
    }
}

// The scalar and list settings the rules tune themselves with, which a cached result depends on.
#[derive(CacheKey)]
pub struct Knobs {
    pub(crate) max_paragraph_words: usize,
    pub(crate) keep_names: Vec<String>,
    pub(crate) meaningless_names: Vec<String>,
    pub(crate) catch_all_modules: Vec<String>,
    pub(crate) module_attributes: Vec<String>,
    pub(crate) module_attribute_scope: Vec<String>,
    pub(crate) max_helper_statements: usize,
    pub(crate) max_caller_statements: usize,
    pub(crate) max_tuple_elements: usize,
    pub(crate) max_cognitive_complexity: usize,
    pub(crate) min_duplicate_statements: usize,
    pub(crate) max_expression_complexity: usize,
    pub(crate) max_instance_attributes: usize,
}

impl Knobs {
    fn resolve(options: ProjectOptions) -> Self {
        let ProjectOptions { max_paragraph_words, naming, imports, logic, classes, .. } = options;
        Self {
            max_paragraph_words: max_paragraph_words.unwrap_or(100),
            keep_names: naming.keep_names.unwrap_or_default(),
            meaningless_names: naming
                .meaningless_names
                .unwrap_or_else(|| ["foo", "baz", "qux", "quux"].map(str::to_owned).into()),
            catch_all_modules: naming.catch_all_modules.unwrap_or_else(|| {
                ["utils", "util", "utilities", "helpers", "helper", "common", "misc"].map(str::to_owned).into()
            }),
            module_attributes: imports.module_attributes.unwrap_or_default(),
            module_attribute_scope: imports.module_attribute_scope.unwrap_or_else(|| vec!["project".to_owned()]),
            max_helper_statements: logic.max_helper_statements.unwrap_or(3),
            max_caller_statements: logic.max_caller_statements.unwrap_or(20),
            max_tuple_elements: logic.max_tuple_elements.unwrap_or(2),
            max_cognitive_complexity: logic.max_cognitive_complexity.unwrap_or(20),
            // A window needs one statement to match, so a smaller setting counts as one.
            min_duplicate_statements: logic.min_duplicate_statements.unwrap_or(6).max(1),
            max_expression_complexity: logic.max_expression_complexity.unwrap_or(8),
            max_instance_attributes: classes.max_instance_attributes.unwrap_or(12),
        }
    }
}

// Ruff's `CompiledPerFile`: a pattern matches a file's name, or its path once the pattern is anchored at the directory
// of the configuration that set it; a leading `!` inverts the match.
struct PerFileIgnore {
    pattern: String,
    absolute: GlobMatcher,
    basename: GlobMatcher,
    negated: bool,
    rules: RuleSet,
}

impl PerFileIgnore {
    fn compile(source: &PerFileIgnoreSource) -> Result<Self> {
        let negated = source.pattern.starts_with('!');
        let basename = source.pattern.trim_start_matches('!');
        let anchored = GlobPath::normalize(basename, &source.base);
        Ok(Self {
            basename: Glob::new(basename)?.compile_matcher(),
            // Anchoring resolves `..`, which can cut an alternation such as `{a,b/../c}.py` in two.
            absolute: Glob::new(&anchored.to_string_lossy())?.compile_matcher(),
            pattern: format!("{}{}", if negated { "!" } else { "" }, anchored.to_string_lossy()),
            negated,
            rules: source
                .selectors
                .iter()
                .map(|selector| selector.resolve().map(|(rules, _)| rules))
                .fold_ok(RuleSet::default(), RuleSet::union)?,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        let name = Candidate::new(path.file_name().unwrap_or_default());
        let matched =
            self.basename.is_match_candidate(&name) || self.absolute.is_match_candidate(&Candidate::new(path));
        matched != self.negated
    }
}

#[cfg(test)]
mod tests {
    use ruff_options_metadata::{OptionEntry, OptionsMetadata};

    use super::*;

    #[test]
    fn documented_defaults_are_the_resolved_ones() {
        let knobs = Knobs::resolve(ProjectOptions::default());
        let resolved = [
            ("max-paragraph-words", serde_json::json!(knobs.max_paragraph_words)),
            ("naming.keep-names", serde_json::json!(knobs.keep_names)),
            ("naming.meaningless-names", serde_json::json!(knobs.meaningless_names)),
            ("naming.catch-all-modules", serde_json::json!(knobs.catch_all_modules)),
            ("imports.module-attributes", serde_json::json!(knobs.module_attributes)),
            ("imports.module-attribute-scope", serde_json::json!(knobs.module_attribute_scope)),
            ("logic.max-helper-statements", serde_json::json!(knobs.max_helper_statements)),
            ("logic.max-caller-statements", serde_json::json!(knobs.max_caller_statements)),
            ("logic.max-tuple-elements", serde_json::json!(knobs.max_tuple_elements)),
            ("logic.max-cognitive-complexity", serde_json::json!(knobs.max_cognitive_complexity)),
            ("logic.min-duplicate-statements", serde_json::json!(knobs.min_duplicate_statements)),
            ("logic.max-expression-complexity", serde_json::json!(knobs.max_expression_complexity)),
            ("classes.max-instance-attributes", serde_json::json!(knobs.max_instance_attributes)),
        ];
        for (name, value) in resolved {
            let field = SimplintOptions::metadata().find(name).and_then(OptionEntry::into_field);
            let documented = field.expect("every knob is documented").default;
            assert_eq!(serde_json::from_str::<serde_json::Value>(documented).expect("a JSON default"), value, "{name}");
        }
    }
}
