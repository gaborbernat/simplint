use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use itertools::Itertools;
use ruff_linter::RuleSelector;
use ruff_linter::settings::types::{OutputFormat, UnsafeFixes};
use ruff_macros::OptionsMetadata;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::lint::registry::RuleSet;
use crate::settings::api::{DocstringStyle, ProjectKindOption};
use crate::settings::schema::RuleSelectorSchema;

// Ruff's `Configuration` for simplint's options: one layer of settings, before it is resolved, so the command line can
// go on top of a configuration file the way Ruff combines them.
#[derive(Clone, Default)]
pub struct Configuration {
    pub(super) rule_selections: Vec<RuleSelection>,
    pub(super) per_file_ignores: Option<Vec<PerFileIgnoreSource>>,
    pub(super) extend_per_file_ignores: Vec<PerFileIgnoreSource>,
    pub(super) extend_safe_fixes: Vec<Selector>,
    pub(super) extend_unsafe_fixes: Vec<Selector>,
    pub(super) project: ProjectOptions,
    pub(super) run: RunOptions,
}

// The rules a configuration enables and those it lets fix, each chosen from selectors the same way.
#[derive(Clone)]
pub(super) struct RuleSelection {
    pub(super) enabled: SelectorLayer,
    pub(super) fixable: SelectorLayer,
}

#[derive(Clone)]
pub(super) struct SelectorLayer {
    pub(super) replace: Option<Vec<Selector>>,
    pub(super) extend: Vec<Selector>,
    pub(super) remove: Vec<Selector>,
}

// A selector as written, with the setting and the source it came from, which Ruff's `RuleResolutionError` names.
#[derive(Clone)]
pub(super) struct Selector {
    text: String,
    setting: &'static str,
    source: Source,
}

impl Selector {
    pub(super) fn resolve(&self) -> Result<(RuleSet, usize)> {
        RuleSet::from_selector(&self.text).ok_or_else(|| {
            let Self { text, setting, source } = self;
            let unknown = format!("Unknown rule selector `{text}` in `{setting}` from {source}");
            if ruff_selector(text).is_some() {
                anyhow!("{unknown}; it selects Ruff rules, so select it in Ruff's configuration instead")
            } else {
                anyhow!(unknown)
            }
        })
    }
}

// The Ruff rules a selector names, by code, prefix or name; `None` when it names none.
pub fn ruff_selector(text: &str) -> Option<RuleSelector> {
    RuleSelector::from_str(text).ok().or_else(|| {
        let rule = ruff_linter::registry::Rule::from_name(text).ok()?;
        Some(RuleSelector::Rule { rule, redirected_from: None })
    })
}

// Ruff's `ValueSource`: the configuration file or the command line a setting comes from.
#[derive(Clone)]
pub enum Source {
    Cli,
    File(Arc<Path>),
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli => write!(f, "the CLI"),
            Self::File(path) => write!(f, "`{}`", path.display()),
        }
    }
}

#[derive(Clone)]
pub(super) struct PerFileIgnoreSource {
    pub(super) pattern: String,
    pub(super) base: PathBuf,
    pub(super) selectors: Vec<Selector>,
}

impl Configuration {
    // Relative patterns are anchored at `base`: the project root for a file, the working directory for the command line.
    pub fn from_options(options: SimplintOptions, base: &Path, source: &Source) -> Self {
        let tag = |setting: &'static str, selectors: Vec<String>| {
            selectors.into_iter().map(|text| Selector { text, setting, source: source.clone() }).collect_vec()
        };
        let per_file = |setting: &'static str, map: BTreeMap<String, Vec<String>>| {
            map.into_iter()
                .map(|(pattern, selectors)| PerFileIgnoreSource {
                    pattern,
                    base: base.to_path_buf(),
                    selectors: tag(setting, selectors),
                })
                .collect_vec()
        };
        let logic = options.logic.unwrap_or_default();
        let naming = options.naming.unwrap_or_default();
        Self {
            rule_selections: vec![RuleSelection {
                enabled: SelectorLayer {
                    replace: options.select.map(|selectors| tag("select", selectors)),
                    extend: tag("extend-select", options.extend_select.unwrap_or_default()),
                    remove: tag("ignore", options.ignore.unwrap_or_default()),
                },
                fixable: SelectorLayer {
                    replace: options.fixable.map(|selectors| tag("fixable", selectors)),
                    extend: tag("extend-fixable", options.extend_fixable.unwrap_or_default()),
                    remove: [
                        tag("unfixable", options.unfixable.unwrap_or_default()),
                        tag("extend-unfixable", options.extend_unfixable.unwrap_or_default()),
                    ]
                    .concat(),
                },
            }],
            per_file_ignores: options.per_file_ignores.map(|map| per_file("per-file-ignores", map)),
            extend_per_file_ignores: options
                .extend_per_file_ignores
                .map(|map| per_file("extend-per-file-ignores", map))
                .unwrap_or_default(),
            extend_safe_fixes: tag("extend-safe-fixes", options.extend_safe_fixes.unwrap_or_default()),
            extend_unsafe_fixes: tag("extend-unsafe-fixes", options.extend_unsafe_fixes.unwrap_or_default()),
            project: ProjectOptions {
                max_paragraph_words: options.max_paragraph_words,
                naming,
                imports: options.imports.unwrap_or_default(),
                logic,
                project_kind: options.project_kind,
                api_packages: options.api_packages,
                docstring_style: options.docstring_style,
            },
            run: RunOptions {
                fix: options.fix,
                fix_only: options.fix_only,
                unsafe_fixes: options.unsafe_fixes,
                show_fixes: options.show_fixes,
                output_format: options.output_format,
            },
        }
    }

    // As Ruff's `Configuration::combine`: `self` wins, and lists that extend go after those of `other`.
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        Self {
            rule_selections: other.rule_selections.into_iter().chain(self.rule_selections).collect(),
            per_file_ignores: self.per_file_ignores.or(other.per_file_ignores),
            extend_per_file_ignores: other
                .extend_per_file_ignores
                .into_iter()
                .chain(self.extend_per_file_ignores)
                .collect(),
            extend_safe_fixes: other.extend_safe_fixes.into_iter().chain(self.extend_safe_fixes).collect(),
            extend_unsafe_fixes: other.extend_unsafe_fixes.into_iter().chain(self.extend_unsafe_fixes).collect(),
            project: self.project.combine(other.project),
            run: self.run.combine(other.run),
        }
    }
}

// The command line's say over `[tool.simplint]`, as Ruff's `ConfigArguments`: inline `--config` options win over every
// configuration file, and the dedicated flags win over both.
#[derive(Default)]
pub struct Overrides {
    pub inline: Configuration,
    pub flags: Configuration,
}

impl Overrides {
    pub(super) fn transform(&self, configuration: Configuration) -> Configuration {
        self.flags.clone().combine(self.inline.clone().combine(configuration))
    }
}

// The options that describe the project and its API, each taken from the last layer that sets it.
#[derive(Clone, Default)]
pub(super) struct ProjectOptions {
    pub(super) max_paragraph_words: Option<usize>,
    pub(super) naming: NamingOptions,
    pub(super) imports: ImportsOptions,
    pub(super) logic: LogicOptions,
    pub(super) project_kind: Option<ProjectKindOption>,
    pub(super) api_packages: Option<Vec<String>>,
    pub(super) docstring_style: Option<DocstringStyle>,
}

impl ProjectOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            max_paragraph_words: self.max_paragraph_words.or(other.max_paragraph_words),
            naming: self.naming.combine(other.naming),
            imports: self.imports.combine(other.imports),
            logic: self.logic.combine(other.logic),
            project_kind: self.project_kind.or(other.project_kind),
            api_packages: self.api_packages.or(other.api_packages),
            docstring_style: self.docstring_style.or(other.docstring_style),
        }
    }
}

// The options that shape a whole run, which Ruff reads from the root configuration only.
#[derive(Clone, Copy, Default)]
pub struct RunOptions {
    pub fix: Option<bool>,
    pub fix_only: Option<bool>,
    pub unsafe_fixes: Option<bool>,
    pub show_fixes: Option<bool>,
    pub output_format: Option<OutputFormat>,
}

impl RunOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            fix: self.fix.or(other.fix),
            fix_only: self.fix_only.or(other.fix_only),
            unsafe_fixes: self.unsafe_fixes.or(other.unsafe_fixes),
            show_fixes: self.show_fixes.or(other.show_fixes),
            output_format: self.output_format.or(other.output_format),
        }
    }

    pub fn unsafe_fixes(self) -> UnsafeFixes {
        self.unsafe_fixes.map_or(UnsafeFixes::Hint, UnsafeFixes::from)
    }
}

/// The `[tool.simplint]` table. Like `[tool.ruff]`, it rejects keys it does not know, so a typo fails loudly instead of
/// being ignored.
#[derive(Deserialize, Default, Clone, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct SimplintOptions {
    /// The style in which violation messages should be formatted: `"full"` (default) (shows source), `"concise"`,
    /// `"grouped"` (group messages by file), `"json"` (machine-readable), `"json-lines"`, `"junit"` (machine-readable
    /// XML), `"github"` (GitHub Actions annotations), `"gitlab"` (GitLab CI code quality report), `"pylint"` (Pylint
    /// text format), `"rdjson"`, `"azure"` (Azure Pipeline logging commands) or `"sarif"`.
    #[option(
        default = r#""full""#,
        value_type = r#""full" | "concise" | "grouped" | "json" | "json-lines" | "junit" | "github" | "gitlab" | "pylint" | "rdjson" | "azure" | "sarif""#,
        example = r#"
            # Group violations by containing file.
            output-format = "grouped"
        "#
    )]
    pub output_format: Option<OutputFormat>,

    /// Enable fix behavior by-default when running `simplint` (overridden by the `--fix` and `--no-fix` command-line
    /// flags). Only includes automatic fixes unless `--unsafe-fixes` is provided.
    #[option(default = "false", value_type = "bool", example = "fix = true")]
    pub fix: Option<bool>,

    /// Enable application of unsafe fixes. If excluded, a hint will be displayed when unsafe fixes are available. If
    /// set to false, the hint will be hidden.
    #[option(default = r#"null"#, value_type = "bool", example = "unsafe-fixes = true")]
    pub unsafe_fixes: Option<bool>,

    /// Like [`fix`](#fix), but disables reporting on leftover violation. Implies [`fix`](#fix).
    #[option(default = "false", value_type = "bool", example = "fix-only = true")]
    pub fix_only: Option<bool>,

    /// Whether to show an enumeration of all fixed lint violations (overridden by the `--show-fixes` command-line
    /// flag).
    #[option(
        default = "false",
        value_type = "bool",
        example = r#"
            # Enumerate all fixed violations.
            show-fixes = true
        "#
    )]
    pub show_fixes: Option<bool>,

    /// A list of rule codes, prefixes or names to enable. Prefixes can specify exact rules (like `SMP601`), entire
    /// groups (like `SMP1`), or anything in between.
    ///
    /// When breaking ties between enabled and disabled rules (via `select` and `ignore`, respectively), more specific
    /// prefixes override less specific prefixes. `ignore` takes precedence over `select` if the same prefix appears in
    /// both.
    #[option(
        default = r#"["ALL"]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # Only enable the test rules.
            select = ["SMP2"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub select: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to enable, in addition to those specified by [`select`](#select).
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of `select`, enable the test rules.
            extend-select = ["SMP2"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub extend_select: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to ignore. Prefixes can specify exact rules (like `SMP601`), entire
    /// groups (like `SMP1`), or anything in between.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Skip the missing `Final` rule (`SMP601`).
            ignore = ["SMP601"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub ignore: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider fixable. By default, all rules are considered fixable.
    #[option(
        default = r#"["ALL"]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # Only allow fix behavior for the test rules.
            fixable = ["SMP2"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub fixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider fixable, in addition to those specified by
    /// [`fixable`](#fixable).
    #[option(
        default = r#"[]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # Enable fix for the test rules, on top of any rules specified by `fixable`.
            extend-fixable = ["SMP2"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub extend_fixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider non-fixable.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Disable fix for the dataclass rules (`SMP701`, `SMP703`).
            unfixable = ["SMP701", "SMP703"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub unfixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider non-fixable, in addition to those specified by
    /// [`unfixable`](#unfixable).
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Also disable fix for the walrus rule (`SMP502`).
            extend-unfixable = ["SMP502"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub extend_unfixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names for which unsafe fixes should be considered safe.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Allow applying the unsafe fixes of `SMP601` without the `--unsafe-fixes` flag.
            extend-safe-fixes = ["SMP601"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub extend_safe_fixes: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names for which safe fixes should be considered unsafe.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Require the `--unsafe-fixes` flag when fixing `SMP701`.
            extend-unsafe-fixes = ["SMP701"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub extend_unsafe_fixes: Option<Vec<String>>,

    /// A list of mappings from file pattern to rule codes, prefixes or names to exclude, when considering any matching
    /// files. A pattern matches the file's name, or its path relative to the project root. An initial '!' negates the
    /// file pattern.
    ///
    /// For more information on the glob syntax, refer to the [`globset`
    /// documentation](https://docs.rs/globset/latest/globset/#syntax).
    #[option(
        default = "{}",
        value_type = "dict[str, list[RuleSelector]]",
        scope = "per-file-ignores",
        example = r#"
            # Ignore `SMP601` (missing `Final`) in all `__init__.py` files, and in `path/to/file.py`.
            "__init__.py" = ["SMP601"]
            "path/to/file.py" = ["SMP601"]
            # Ignore the test rules everywhere except for the `tests/` directory.
            "!tests/**.py" = ["SMP8"]
        "#
    )]
    #[schemars(with = "BTreeMap<String, Vec<RuleSelectorSchema>>")]
    pub per_file_ignores: Option<BTreeMap<String, Vec<String>>>,

    /// A list of mappings from file pattern to rule codes, prefixes or names to exclude, in addition to any rules
    /// excluded by [`per-file-ignores`](#per-file-ignores).
    #[option(
        default = "{}",
        value_type = "dict[str, list[RuleSelector]]",
        scope = "extend-per-file-ignores",
        example = r#"
            # Also ignore `SMP601` in all `__init__.py` files.
            "__init__.py" = ["SMP601"]
        "#
    )]
    #[schemars(with = "BTreeMap<String, Vec<RuleSelectorSchema>>")]
    pub extend_per_file_ignores: Option<BTreeMap<String, Vec<String>>>,

    /// Whether the project is a `"library"`, whose public API other code imports, or an `"application"`, which has no
    /// Python API, so all of its code is internal. `"auto"` picks `"library"` when something installs the project: a
    /// `[build-system]` table, or a `setup.py` or `setup.cfg`, unless `[tool.uv]` sets `package = false` or maturin
    /// builds only a binary.
    #[option(
        default = r#""auto""#,
        value_type = r#""auto" | "library" | "application""#,
        example = r#"
            # A command-line tool whose modules nobody imports.
            project-kind = "application"
        "#
    )]
    pub project_kind: Option<ProjectKindOption>,

    /// The top-level packages and modules a library ships, whose modules make up its public API. By default they come
    /// from the build backend's settings, then from the packages holding a `py.typed` marker, then from the package
    /// named after the project; when none is found, every module outside `docs` and the tests counts.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"
            api-packages = ["my_package"]
        "#
    )]
    pub api_packages: Option<Vec<String>>,

    /// The markup of docstrings, which the reStructuredText rules, `SMP910` to `SMP925`, check: `"rst"` for
    /// reStructuredText field lists, such as `:param name:`, `:returns:` and `:raises Error:`. Unset, those rules do not
    /// report.
    #[option(
        default = "null",
        value_type = r#""rst""#,
        example = r#"
            docstring-style = "rst"
        "#
    )]
    pub docstring_style: Option<DocstringStyle>,

    /// The most words a paragraph may hold: a comment paragraph for `long-paragraph` (`SMP002`), and a docstring
    /// paragraph for `long-docstring-paragraph` (`SMP904`).
    #[option(
        default = "100",
        value_type = "int",
        example = r#"
            max-paragraph-words = 80
        "#
    )]
    pub max_paragraph_words: Option<usize>,

    /// Options for the naming rules.
    #[option_group]
    pub naming: Option<NamingOptions>,

    /// Options for the import rules.
    #[option_group]
    pub imports: Option<ImportsOptions>,
    /// Options for the logic rules.
    #[option_group]
    pub logic: Option<LogicOptions>,
}

/// The `[tool.simplint.naming]` table.
#[derive(Deserialize, Default, Clone, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct NamingOptions {
    /// A list of module-scope names that `module-variable-case` (`SMP102`) and `module-variable-prefix` (`SMP404`) leave
    /// as they are, such as the variables a framework reads from a module by their exact lowercase names.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"
            # Django reads `urlpatterns` from a URLconf module by name.
            keep-names = ["urlpatterns"]
        "#
    )]
    pub keep_names: Option<Vec<String>>,

    /// Words that name a variable, parameter or attribute without saying what it holds, which `meaningless-name`
    /// (`SMP103`) reports. A word matches in lower or upper case, with or without leading or trailing underscores.
    #[option(
        default = r#"["foo", "baz", "qux", "quux"]"#,
        value_type = "list[str]",
        example = r#"
            # A team that also bans its own placeholder words.
            meaningless-names = ["foo", "baz", "tmp", "stuff"]
        "#
    )]
    pub meaningless_names: Option<Vec<String>>,

    /// Names of modules and packages that `catch-all-module` (`SMP104`) reports, since they hold whatever does not fit
    /// elsewhere. A package is reported through its `__init__.py`.
    #[option(
        default = r#"["utils", "util", "utilities", "helpers", "helper", "common", "misc"]"#,
        value_type = "list[str]",
        example = r#"
            # A project whose `types` module holds its type aliases only.
            catch-all-modules = ["utils", "helpers", "misc"]
        "#
    )]
    pub catch_all_modules: Option<Vec<String>>,
}

impl NamingOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            keep_names: self.keep_names.or(other.keep_names),
            meaningless_names: self.meaningless_names.or(other.meaningless_names),
            catch_all_modules: self.catch_all_modules.or(other.catch_all_modules),
        }
    }
}

/// The `[tool.simplint.imports]` table.
#[derive(Deserialize, Default, Clone, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImportsOptions {
    /// Absolute names of modules whose attributes the code may reach as `module.Name`, such as a module that tests patch
    /// attribute by attribute. `module-attribute-import` (`SMP304`) leaves them alone.
    #[option(
        default = "[]",
        value_type = "list[str]",
        example = r#"
            # Tests patch `settings` by its module attribute.
            module-attributes = ["app.settings"]
        "#
    )]
    pub module_attributes: Option<Vec<String>>,
}

impl ImportsOptions {
    fn combine(self, other: Self) -> Self {
        Self { module_attributes: self.module_attributes.or(other.module_attributes) }
    }
}

/// The `[tool.simplint.logic]` table.
#[derive(Deserialize, Default, Clone, Copy, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct LogicOptions {
    /// The most statements a private helper may have for `single-caller-helper` (`SMP505`) to suggest inlining it into
    /// its one caller. A docstring does not count.
    #[option(
        default = "3",
        value_type = "int",
        example = r#"
            # Only one-line helpers are inlined.
            max-helper-statements = 2
        "#
    )]
    pub max_helper_statements: Option<usize>,

    /// The most statements the caller of such a helper may have once the helper's statements are added to its own.
    #[option(
        default = "20",
        value_type = "int",
        example = r#"
            max-caller-statements = 30
        "#
    )]
    pub max_caller_statements: Option<usize>,

    /// The most elements a tuple returned from a function may hold before `long-tuple-return` (`SMP528`) reports it. A
    /// longer tuple is a record whose fields are known only by position; a dataclass or a named tuple names them.
    #[option(
        default = "2",
        value_type = "int",
        example = r#"
            # Three-element tuples are records too.
            max-tuple-elements = 2
        "#
    )]
    pub max_tuple_elements: Option<usize>,

    /// The highest cognitive complexity a function may have before `cognitive-complexity` (`SMP529`) reports it. The
    /// score counts each branch, loop and handler more the deeper it is nested, as `SonarSource` defines it.
    #[option(
        default = "20",
        value_type = "int",
        example = r#"
            max-cognitive-complexity = 30
        "#
    )]
    pub max_cognitive_complexity: Option<usize>,
    /// The fewest statements in a row another module must repeat for `duplicate-code` (`SMP530`) to report them.
    /// Statements count as those of one body, each nested block counting as one.
    #[option(
        default = "6",
        value_type = "int",
        example = r#"
            # Only report longer copies.
            min-duplicate-statements = 10
        "#
    )]
    pub min_duplicate_statements: Option<usize>,
}

impl LogicOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            max_helper_statements: self.max_helper_statements.or(other.max_helper_statements),
            max_caller_statements: self.max_caller_statements.or(other.max_caller_statements),
            max_tuple_elements: self.max_tuple_elements.or(other.max_tuple_elements),
            max_cognitive_complexity: self.max_cognitive_complexity.or(other.max_cognitive_complexity),
            min_duplicate_statements: self.min_duplicate_statements.or(other.min_duplicate_statements),
        }
    }
}
