mod api;
mod file;
mod metadata;
mod options;
mod schema;

use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use globset::{Candidate, Glob, GlobMatcher};
use itertools::Itertools;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_diagnostics::{Applicability, Fix};
use ruff_linter::settings::types::GlobPath;
use ruff_macros::CacheKey;
use serde::Deserialize;

pub use api::{ApiScope, DocstringStyle, ModuleRole, ProjectKind};
pub use file::{FileSettings, FromRuff};
pub use options::{Configuration, Overrides, RunOptions, SimplintOptions, Source, ruff_selector};
pub use schema::config_schema;

use crate::lint::registry::{Rule, RuleSet};
use crate::settings::metadata::{BuildConfiguration, ProjectMetadata};
use crate::settings::options::{
    ImportsOptions, LogicOptions, NamingOptions, PerFileIgnoreSource, ProjectOptions, Selector, SelectorLayer,
};

// The scalar and list settings the rules tune themselves with, which a cached result depends on.
#[derive(CacheKey)]
pub struct Knobs {
    pub max_paragraph_words: usize,
    pub keep_names: Vec<String>,
    pub meaningless_names: Vec<String>,
    pub catch_all_modules: Vec<String>,
    pub module_attributes: Vec<String>,
    pub max_helper_statements: usize,
    pub max_caller_statements: usize,
    pub max_tuple_elements: usize,
    pub max_cognitive_complexity: usize,
    pub min_duplicate_statements: usize,
}

impl Knobs {
    fn resolve(
        max_paragraph_words: Option<usize>,
        naming: NamingOptions,
        imports: ImportsOptions,
        logic: LogicOptions,
    ) -> Self {
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
            max_helper_statements: logic.max_helper_statements.unwrap_or(3),
            max_caller_statements: logic.max_caller_statements.unwrap_or(20),
            max_tuple_elements: logic.max_tuple_elements.unwrap_or(2),
            max_cognitive_complexity: logic.max_cognitive_complexity.unwrap_or(20),
            // A window needs one statement to match, so a smaller setting counts as one.
            min_duplicate_statements: logic.min_duplicate_statements.unwrap_or(6).max(1),
        }
    }
}

// simplint's own options and the project metadata its rules read. The Python version, the files to check and what to
// exclude come from Ruff's configuration instead, resolved the way `ruff check` resolves them.
pub struct Settings {
    pub root: PathBuf,
    pub path: Option<PathBuf>,
    pub metadata: ProjectMetadata,
    pub run: RunOptions,
    pub knobs: Knobs,
    pub api: ApiScope,
    pub docstring_style: Option<DocstringStyle>,
    rules: RuleSet,
    fixable: RuleSet,
    fix_safety: FixSafetyTable,
    per_file_ignores: Vec<PerFileIgnore>,
}

impl Settings {
    // Ruff's `Configuration::into_settings`: the command line goes on top of the `pyproject.toml`'s options, then the
    // rule selections resolve in order. Without a `pyproject.toml`, the defaults apply; one comes with its parsed table.
    fn resolve(root: &Path, pyproject: Option<(&Path, toml::Table)>, overrides: &Overrides) -> Result<Self> {
        let path = pyproject.as_ref().map(|(path, _)| *path);
        let (metadata, build, configuration) = match pyproject {
            Some((path, table)) => {
                let build = BuildConfiguration::of(&table);
                let mut pyproject: Pyproject =
                    table.try_into().with_context(|| format!("Failed to parse {}", path.display()))?;
                pyproject.project.add_groups(&pyproject.dependency_groups);
                let source = Source::File(Arc::from(path));
                (pyproject.project, build, Configuration::from_options(pyproject.tool.simplint, root, &source))
            }
            None => (ProjectMetadata::default(), BuildConfiguration::default(), Configuration::default()),
        };
        let configuration = overrides.transform(configuration);
        let rules = resolve_rules(configuration.rule_selections.iter().map(|selection| &selection.enabled))?;
        let fixable = resolve_rules(configuration.rule_selections.iter().map(|selection| &selection.fixable))?;
        let fix_safety =
            FixSafetyTable::from_selectors(&configuration.extend_safe_fixes, &configuration.extend_unsafe_fixes)?;
        let ProjectOptions {
            max_paragraph_words,
            naming,
            imports,
            logic,
            project_kind,
            api_packages,
            api_modules,
            docstring_style,
        } = configuration.project;
        let knobs = Knobs::resolve(max_paragraph_words, naming, imports, logic);
        let api = ApiScope::resolve(
            root,
            project_kind.unwrap_or_default(),
            &build,
            &metadata,
            (api_packages.as_deref(), api_modules.as_deref()),
        );
        Ok(Self {
            root: root.to_path_buf(),
            path: path.map(Path::to_path_buf),
            metadata,
            api,
            docstring_style,
            run: configuration.run,
            knobs,
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

    pub fn rules_for(&self, path: &Path) -> RuleSet {
        let absolute = std::path::absolute(path).unwrap_or_default();
        self.per_file_ignores
            .iter()
            .filter(|ignore| ignore.matches(&absolute))
            .fold(self.rules, |rules, ignore| rules.subtract(ignore.rules))
    }

    // Ruff drops the fix of a rule `fixable` leaves out, and `extend-safe-fixes` and `extend-unsafe-fixes` change how
    // safe the rest are.
    pub fn fix_for(&self, rule: Rule, fix: Option<Fix>) -> Option<Fix> {
        let fix = fix.filter(|_| self.is_fixable(rule))?;
        let applicability = self.fix_safety.resolve_applicability(rule, fix.applicability());
        Some(fix.with_applicability(applicability))
    }

    pub fn is_fixable(&self, rule: Rule) -> bool {
        self.fixable.contains(rule)
    }

    pub fn is_test_module(&self, path: &Path) -> bool {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || name == "conftest.py"
            || path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .components()
                .any(|part| matches!(part.as_os_str().to_str(), Some("tests" | "test")))
    }

    // The rule tables `--show-settings` lists: enabled, fixable, forced safe and forced unsafe.
    pub fn rule_sets(&self) -> [RuleSet; 4] {
        [self.rules, self.fixable, self.fix_safety.forced_safe, self.fix_safety.forced_unsafe]
    }

    // Each per-file ignore's pattern, anchored at the configuration that set it, and the rules it turns off.
    pub fn per_file_ignores(&self) -> impl Iterator<Item = (&str, RuleSet)> {
        self.per_file_ignores.iter().map(|ignore| (ignore.pattern.as_str(), ignore.rules))
    }

    // Where a file sits in its library's public API; `module_path` is the module Python imports it as.
    pub fn role(&self, path: &Path, module_path: Option<&[String]>) -> ModuleRole {
        self.api.role(module_path, self.is_test_module(path))
    }
}

// The part of a file's settings its whole project shares that its diagnostics depend on, so a cached result is reused
// only under the same; the rules and Ruff's decisions for the file add the rest.
impl CacheKey for Settings {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.root.cache_key(state);
        self.metadata.dependencies.iter().for_each(|dependency| dependency.cache_key(state));
        self.metadata.declared.iter().for_each(|dependency| dependency.cache_key(state));
        self.metadata.entry_points.cache_key(state);
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
fn resolve_rules<'a>(layers: impl IntoIterator<Item = &'a SelectorLayer>) -> Result<RuleSet> {
    layers.into_iter().try_fold(RuleSet::all(), |rules, layer| {
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
            absolute: Glob::new(&anchored.to_string_lossy())?.compile_matcher(),
            basename: Glob::new(basename)?.compile_matcher(),
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

// Where `[tool.simplint]` comes from: nowhere with `--isolated`, the `--config` file when it is a `pyproject.toml`, and
// otherwise the `pyproject.toml` closest to each file.
pub struct SettingsResolver {
    pub root: Arc<Settings>,
    overrides: Overrides,
    discovery: Discovery,
}

impl SettingsResolver {
    pub fn new(config_file: Option<&Path>, isolated: bool, overrides: Overrides) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        // A relative `--config` would anchor per-file ignore patterns at an empty directory.
        let config_file = config_file.map(std::path::absolute).transpose()?;
        let (root, pyproject) = match config_file.as_deref() {
            _ if isolated => (cwd.as_path(), None),
            Some(config) => (
                config.parent().unwrap_or(&cwd),
                Some(config).filter(|config| config.file_name().is_some_and(|name| name == "pyproject.toml")),
            ),
            None => {
                let discovery = Discovery::Hierarchical { run_root: cwd.clone(), projects: Mutex::default() };
                return Ok(Self { root: discovery.settings(&cwd, &overrides)?, overrides, discovery });
            }
        };
        let pyproject = match pyproject {
            Some(path) => Some((path, read_table(path)?)),
            None => None,
        };
        let fixed = Arc::new(Settings::resolve(root, pyproject, &overrides)?);
        Ok(Self { root: Arc::clone(&fixed), overrides, discovery: Discovery::Fixed(fixed) })
    }

    pub fn for_file(&self, path: &Path) -> Result<Arc<Settings>> {
        self.discovery.settings(path, &self.overrides)
    }
}

// Ruff's `PyprojectDiscoveryStrategy`: one configuration for every file, or the one each file's location points at,
// read once per project.
enum Discovery {
    Fixed(Arc<Settings>),
    // `run_root` is the directory the run starts from, which roots a project without a `pyproject.toml`.
    Hierarchical { run_root: PathBuf, projects: Mutex<Projects> },
}

impl Discovery {
    // The closest `pyproject.toml` with a `[tool.simplint]` table holds the settings and the project's metadata;
    // without one, the defaults apply, rooted at the run's directory or the nearest ancestor of it holding the file,
    // as Ruff roots a project without configuration at the working directory. Projects are cached by both the
    // directory a lookup starts from and their root.
    fn settings(&self, path: &Path, overrides: &Overrides) -> Result<Arc<Settings>> {
        let (run_root, projects) = match self {
            Self::Fixed(settings) => return Ok(Arc::clone(settings)),
            Self::Hierarchical { run_root, projects } => (run_root, projects),
        };
        let absolute = std::path::absolute(path)?;
        let start = if absolute.is_dir() { absolute.as_path() } else { absolute.parent().unwrap_or(&absolute) };
        let mut projects = projects.lock().expect("no worker panics while resolving settings");
        if let Some(settings) = projects.settings.get(start) {
            return Ok(Arc::clone(settings));
        }
        let pyproject = projects.find_pyproject(start)?;
        let root = match pyproject.as_deref().and_then(Path::parent) {
            Some(root) => root,
            None => run_root.ancestors().find(|dir| start.starts_with(dir)).unwrap_or(start),
        };
        let settings = if let Some(settings) = projects.settings.get(root) {
            Arc::clone(settings)
        } else {
            let parsed = pyproject.as_deref().map(|path| (path, projects.take_table(path)));
            Arc::new(Settings::resolve(root, parsed, overrides)?)
        };
        projects.settings.insert(root.to_path_buf(), Arc::clone(&settings));
        projects.settings.insert(start.to_path_buf(), Arc::clone(&settings));
        Ok(settings)
    }
}

// What hierarchical discovery found so far: the settings by the directories they apply to, and each `pyproject.toml`
// it met, so every one is read and parsed once.
#[derive(Default)]
struct Projects {
    settings: FxHashMap<PathBuf, Arc<Settings>>,
    pyprojects: FxHashMap<PathBuf, PyprojectFile>,
}

// A `pyproject.toml` met: whether it configures simplint, and its table until the project rooted next to it takes it.
struct PyprojectFile {
    configures: bool,
    table: Option<toml::Table>,
}

impl Projects {
    // Like Ruff's `settings_toml`, which passes over a `pyproject.toml` without `[tool.ruff]`, so that a nested package
    // inherits its parent's settings. When no ancestor configures simplint, the nearest `pyproject.toml` still gives
    // the project's metadata.
    fn find_pyproject(&mut self, start: &Path) -> Result<Option<PathBuf>> {
        let candidates =
            start.ancestors().map(|dir| dir.join("pyproject.toml")).filter(|candidate| candidate.is_file());
        let mut nearest = None;
        for candidate in candidates {
            if !self.pyprojects.contains_key(&candidate) {
                let table = read_table(&candidate)?;
                let configures = table.get("tool").and_then(|tool| tool.get("simplint")).is_some();
                self.pyprojects.insert(candidate.clone(), PyprojectFile { configures, table: Some(table) });
            }
            if self.pyprojects[&candidate].configures {
                return Ok(Some(candidate));
            }
            nearest.get_or_insert(candidate);
        }
        Ok(nearest)
    }

    // A project resolves once, from the table `find_pyproject` parsed.
    fn take_table(&mut self, path: &Path) -> toml::Table {
        self.pyprojects.get_mut(path).and_then(|file| file.table.take()).expect("a project resolves once")
    }
}

fn read_table(path: &Path) -> Result<toml::Table> {
    let text = std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("Failed to parse {}", path.display()))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Pyproject {
    project: ProjectMetadata,
    #[serde(rename = "dependency-groups")]
    dependency_groups: BTreeMap<String, Vec<toml::Value>>,
    tool: Tool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Tool {
    simplint: SimplintOptions,
}

#[cfg(test)]
mod tests {
    use ruff_options_metadata::{OptionEntry, OptionsMetadata};

    use super::*;

    // Each option's documented default parses to the value `Knobs::resolve` gives it with no option set.
    #[test]
    fn documented_defaults_are_the_resolved_ones() {
        let knobs = Knobs::resolve(None, NamingOptions::default(), ImportsOptions::default(), LogicOptions::default());
        let resolved = [
            ("max-paragraph-words", serde_json::json!(knobs.max_paragraph_words)),
            ("naming.keep-names", serde_json::json!(knobs.keep_names)),
            ("naming.meaningless-names", serde_json::json!(knobs.meaningless_names)),
            ("naming.catch-all-modules", serde_json::json!(knobs.catch_all_modules)),
            ("imports.module-attributes", serde_json::json!(knobs.module_attributes)),
            ("logic.max-helper-statements", serde_json::json!(knobs.max_helper_statements)),
            ("logic.max-caller-statements", serde_json::json!(knobs.max_caller_statements)),
            ("logic.max-tuple-elements", serde_json::json!(knobs.max_tuple_elements)),
            ("logic.max-cognitive-complexity", serde_json::json!(knobs.max_cognitive_complexity)),
            ("logic.min-duplicate-statements", serde_json::json!(knobs.min_duplicate_statements)),
        ];
        for (name, value) in resolved {
            let field = SimplintOptions::metadata().find(name).and_then(OptionEntry::into_field);
            let documented = field.expect("every knob is documented").default;
            assert_eq!(serde_json::from_str::<serde_json::Value>(documented).expect("a JSON default"), value, "{name}");
        }
    }
}
