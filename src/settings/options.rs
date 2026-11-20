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

// `None` when `text` is neither a Ruff selector nor a Ruff rule name.
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
    // Relative patterns are anchored at `base`: the project root for a file, the working directory for the command
    // line.
    pub(crate) fn from_options(options: SimplintOptions, base: &Path, source: &Source) -> Self {
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
                naming: options.naming.unwrap_or_default(),
                imports: options.imports.unwrap_or_default(),
                logic: options.logic.unwrap_or_default(),
                classes: options.classes.unwrap_or_default(),
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
    pub(crate) fn combine(self, other: Self) -> Self {
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
    pub(crate) inline: Configuration,
    pub(crate) flags: Configuration,
}

impl Overrides {
    pub(super) fn transform(&self, configuration: Configuration) -> Configuration {
        self.flags.clone().combine(self.inline.clone().combine(configuration))
    }
}

#[derive(Clone, Default)]
pub(super) struct ProjectOptions {
    pub(super) max_paragraph_words: Option<usize>,
    pub(super) naming: NamingOptions,
    pub(super) imports: ImportsOptions,
    pub(super) logic: LogicOptions,
    pub(super) classes: ClassesOptions,
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
            classes: self.classes.combine(other.classes),
            project_kind: self.project_kind.or(other.project_kind),
            api_packages: self.api_packages.or(other.api_packages),
            docstring_style: self.docstring_style.or(other.docstring_style),
        }
    }
}

// The options that shape a whole run, which Ruff reads from the root configuration only.
#[derive(Clone, Copy, Default)]
pub struct RunOptions {
    pub(crate) fix: Option<bool>,
    pub(crate) fix_only: Option<bool>,
    pub(crate) unsafe_fixes: Option<bool>,
    pub(crate) show_fixes: Option<bool>,
    pub(crate) output_format: Option<OutputFormat>,
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

    pub(crate) fn unsafe_fixes(self) -> UnsafeFixes {
        self.unsafe_fixes.map_or(UnsafeFixes::Hint, UnsafeFixes::from)
    }
}

/// The `[tool.simplint]` table rejects unknown keys.
#[derive(Deserialize, Default, Clone, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct SimplintOptions {
    /// Format for reported violations. The default, `"full"`, shows the source. Use `"concise"` for one line per
    /// violation or `"grouped"` to group violations by file. Other formats include JSON (`"json"`, `"json-lines"`),
    /// CI reports (`"junit"`, `"github"`, `"gitlab"`, `"azure"`, `"sarif"`), `"pylint"` and `"rdjson"`.
    #[option(
        default = r#""full""#,
        value_type = r#""full" | "concise" | "grouped" | "json" | "json-lines" | "junit" | "github" | "gitlab" | "pylint" | "rdjson" | "azure" | "sarif""#,
        example = r#"
            # Group violations by containing file.
            output-format = "grouped"
        "#
    )]
    pub(crate) output_format: Option<OutputFormat>,

    /// Apply safe fixes by default with `simplint check`. Enable `unsafe-fixes` to include unsafe fixes.
    /// The `--fix` and `--no-fix` flags override this setting.
    #[option(default = "false", value_type = "bool", example = "fix = true")]
    pub(crate) fix: Option<bool>,

    /// Allow unsafe fixes. When unset, the text summary shows a hint if unsafe fixes are available.
    /// Set this to `false` to hide the hint.
    #[option(default = r#"null"#, value_type = "bool", example = "unsafe-fixes = true")]
    pub(crate) unsafe_fixes: Option<bool>,

    /// Apply fixes and omit remaining violations from the report and exit status. Implies [`fix`](#fix).
    #[option(default = "false", value_type = "bool", example = "fix-only = true")]
    pub(crate) fix_only: Option<bool>,

    /// In text output, list the fixes applied to each file with a count per rule. `--show-fixes` and `--no-show-fixes`
    /// override this setting.
    #[option(
        default = "false",
        value_type = "bool",
        example = r#"
            # Enumerate all fixed violations.
            show-fixes = true
        "#
    )]
    pub(crate) show_fixes: Option<bool>,

    /// Select rules by code (`SMP601`), prefix (`SMP1`) or name (`missing-final`). An explicit `ALL` includes
    /// `unused-ignore` (`SMP009`), which is off when `select` is unset. Opt-in rules such as `init-module-content`
    /// (`SMP407`) require their full code or name.
    ///
    /// More specific selectors take precedence within a configuration layer. `ignore` wins a tie with `select`.
    #[option(
        default = "all rules except unused-ignore and opt-in rules",
        value_type = "list[RuleSelector]",
        example = r#"
            # Only enable the test rules.
            select = ["SMP8"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) select: Option<Vec<String>>,

    /// Enable additional rules using the same selectors as [`select`](#select).
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # On top of `select`, enable the test rules.
            extend-select = ["SMP8"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) extend_select: Option<Vec<String>>,

    /// Disable rules by code, code prefix or name.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Skip the missing `Final` rule (`SMP601`).
            ignore = ["SMP601"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) ignore: Option<Vec<String>>,

    /// Allow fixes for the selected rules. By default, simplint can offer any fix a rule provides.
    #[option(
        default = r#"["ALL"]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # Only allow fix behavior for the test rules.
            fixable = ["SMP8"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) fixable: Option<Vec<String>>,

    /// Allow fixes for additional rules beyond [`fixable`](#fixable).
    #[option(
        default = r#"[]"#,
        value_type = "list[RuleSelector]",
        example = r#"
            # Enable fix for the test rules, on top of any rules specified by `fixable`.
            extend-fixable = ["SMP8"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) extend_fixable: Option<Vec<String>>,

    /// Hide and disable fixes for rules matching these codes, code prefixes or names.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Disable fix for the dataclass rules (`SMP701`, `SMP703`).
            unfixable = ["SMP701", "SMP703"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) unfixable: Option<Vec<String>>,

    /// Disable fixes for additional rules beyond [`unfixable`](#unfixable).
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Also disable fix for the walrus rule (`SMP502`).
            extend-unfixable = ["SMP502"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) extend_unfixable: Option<Vec<String>>,

    /// Treat unsafe fixes as safe for rules matching these codes, code prefixes or names.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Allow applying the unsafe fixes of `SMP601` without the `--unsafe-fixes` flag.
            extend-safe-fixes = ["SMP601"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) extend_safe_fixes: Option<Vec<String>>,

    /// Require unsafe fixes to be enabled for rules matching these codes, code prefixes or names.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Require the `--unsafe-fixes` flag when fixing `SMP701`.
            extend-unsafe-fixes = ["SMP701"]
        "#
    )]
    #[schemars(with = "Vec<RuleSelectorSchema>")]
    pub(crate) extend_unsafe_fixes: Option<Vec<String>>,

    /// Map file patterns to rule codes, prefixes or names to ignore. Match a filename or a path relative to the
    /// project root. A leading `!` applies the ignore to files outside the pattern. See the
    /// [`globset` syntax](https://docs.rs/globset/latest/globset/#syntax).
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
    pub(crate) per_file_ignores: Option<BTreeMap<String, Vec<String>>>,

    /// Add patterns and rule selectors to [`per-file-ignores`](#per-file-ignores).
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
    pub(crate) extend_per_file_ignores: Option<BTreeMap<String, Vec<String>>>,

    /// Treat the project as a `"library"` with a public Python API or an `"application"` whose modules are internal.
    /// The default, `"auto"`, selects `"library"` if the project has `[build-system]`, `setup.py` or `setup.cfg`.
    /// Setting `tool.uv.package = false` or `tool.maturin.bindings = "bin"` selects `"application"` in auto mode.
    #[option(
        default = r#""auto""#,
        value_type = r#""auto" | "library" | "application""#,
        example = r#"
            # A command-line tool whose modules nobody imports.
            project-kind = "application"
        "#
    )]
    pub project_kind: Option<ProjectKindOption>,

    /// Packages and modules that can expose a library's public API. When unset, simplint tries the build backend's
    /// package list, then packages with `py.typed`, then a package or module named for the project. If none match,
    /// public import paths outside `doc`, `docs` and the tests qualify. An explicit empty list selects no packages.
    #[option(
        default = "null",
        value_type = "list[str]",
        example = r#"
            api-packages = ["my_package"]
        "#
    )]
    pub(crate) api_packages: Option<Vec<String>>,

    /// Set to `"rst"` to check reStructuredText docstrings with `SMP910` through `SMP925`. These rules check inline
    /// markup and fields such as `:param name:`, `:returns:` and `:raises Error:`. They skip files when this is unset.
    #[option(
        default = "null",
        value_type = r#""rst""#,
        example = r#"
            docstring-style = "rst"
        "#
    )]
    pub(crate) docstring_style: Option<DocstringStyle>,

    /// Maximum words in a comment paragraph (`long-paragraph`, `SMP002`) or docstring paragraph
    /// (`long-docstring-paragraph`, `SMP904`).
    #[option(
        default = "100",
        value_type = "int",
        example = r#"
            max-paragraph-words = 80
        "#
    )]
    pub(crate) max_paragraph_words: Option<usize>,

    /// Options for the naming rules.
    #[option_group]
    pub naming: Option<NamingOptions>,

    /// Options for the import rules.
    #[option_group]
    pub imports: Option<ImportsOptions>,

    /// Options for the logic rules.
    #[option_group]
    pub logic: Option<LogicOptions>,

    /// Options for the class rules.
    #[option_group]
    pub classes: Option<ClassesOptions>,
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
    pub(crate) keep_names: Option<Vec<String>>,

    /// Variable, parameter and attribute names for `meaningless-name` (`SMP103`) to flag. A name matches the
    /// configured spelling or its uppercase form after stripping leading and trailing underscores.
    #[option(
        default = r#"["foo", "baz", "qux", "quux"]"#,
        value_type = "list[str]",
        example = r#"
            # A team that also bans its own placeholder words.
            meaningless-names = ["foo", "baz", "tmp", "stuff"]
        "#
    )]
    pub(crate) meaningless_names: Option<Vec<String>>,

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
    pub(crate) catch_all_modules: Option<Vec<String>>,
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
    pub(crate) module_attributes: Option<Vec<String>>,

    /// Modules to check with `module-attribute-import` (`SMP304`). Use `"project"` for modules in the run, `"stdlib"`
    /// for standard-library modules or `"third-party"` for other external imports. A dotted module name includes its
    /// submodules. For external imports, the rule respects Ruff's `flake8-import-conventions` aliases and `banned-from`.
    #[option(
        default = r#"["project"]"#,
        value_type = "list[str]",
        example = r#"
            # Also import the names of the standard library and of `requests`.
            module-attribute-scope = ["project", "stdlib", "requests"]
        "#
    )]
    pub(crate) module_attribute_scope: Option<Vec<String>>,

    /// Check eligible imports with `missing-lazy-module` (`SMP314`) even in modules without `__lazy_modules__`.
    /// By default the rule requires an existing list. It skips Python 3.15 and later targets.
    #[option(
        default = "false",
        value_type = "bool",
        example = r#"
            # Ask for `__lazy_modules__` in every module.
            require-lazy-modules = true
        "#
    )]
    pub(crate) require_lazy_modules: Option<bool>,
}

impl ImportsOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            module_attributes: self.module_attributes.or(other.module_attributes),
            module_attribute_scope: self.module_attribute_scope.or(other.module_attribute_scope),
            require_lazy_modules: self.require_lazy_modules.or(other.require_lazy_modules),
        }
    }
}

/// The `[tool.simplint.logic]` table.
#[derive(Deserialize, Default, Clone, Copy, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct LogicOptions {
    /// Only suggest inlining a helper with fewer than this many top-level statements (`single-caller-helper`,
    /// `SMP505`). With the default of 3, up to two statements qualify. A leading docstring does not count.
    #[option(
        default = "3",
        value_type = "int",
        example = r#"
            # Only suggest inlining single-statement helpers.
            max-helper-statements = 2
        "#
    )]
    pub(crate) max_helper_statements: Option<usize>,

    /// Maximum combined number of top-level statements in the helper and its caller for `single-caller-helper`
    /// (`SMP505`). Leading docstrings do not count.
    #[option(
        default = "20",
        value_type = "int",
        example = r#"
            max-caller-statements = 30
        "#
    )]
    pub(crate) max_caller_statements: Option<usize>,

    /// Maximum number of elements in a returned tuple before `long-tuple-return` (`SMP528`) reports it.
    #[option(
        default = "2",
        value_type = "int",
        example = r#"
            # Allow three-element tuples, such as `(x, y, z)`.
            max-tuple-elements = 3
        "#
    )]
    pub(crate) max_tuple_elements: Option<usize>,

    /// Maximum cognitive complexity of a function for `cognitive-complexity` (`SMP529`). Nested `if` statements,
    /// loops and exception handlers cost more than the same constructs at the top level.
    #[option(
        default = "20",
        value_type = "int",
        example = r#"
            max-cognitive-complexity = 30
        "#
    )]
    pub(crate) max_cognitive_complexity: Option<usize>,

    /// Minimum number of consecutive statements that must match across modules for `duplicate-code` (`SMP530`) to
    /// report them. A compound statement and its body count as one statement in the surrounding block.
    #[option(
        default = "6",
        value_type = "int",
        example = r#"
            # Only report longer copies.
            min-duplicate-statements = 10
        "#
    )]
    pub(crate) min_duplicate_statements: Option<usize>,

    /// Maximum expression nesting score for `expression-complexity` (`SMP561`). Operators, calls, subscripts,
    /// conditional expressions, assignments with `:=`, lambdas and comprehensions each add a level. A flat chain of
    /// `and` or `+` counts once, as does a chain of calls and subscripts on the same receiver.
    #[option(
        default = "8",
        value_type = "int",
        example = r#"
            max-expression-complexity = 10
        "#
    )]
    pub(crate) max_expression_complexity: Option<usize>,
}

impl LogicOptions {
    fn combine(self, other: Self) -> Self {
        Self {
            max_helper_statements: self.max_helper_statements.or(other.max_helper_statements),
            max_caller_statements: self.max_caller_statements.or(other.max_caller_statements),
            max_tuple_elements: self.max_tuple_elements.or(other.max_tuple_elements),
            max_cognitive_complexity: self.max_cognitive_complexity.or(other.max_cognitive_complexity),
            min_duplicate_statements: self.min_duplicate_statements.or(other.min_duplicate_statements),
            max_expression_complexity: self.max_expression_complexity.or(other.max_expression_complexity),
        }
    }
}

/// The `[tool.simplint.classes]` table.
#[derive(Deserialize, Default, Clone, Copy, JsonSchema, OptionsMetadata)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct ClassesOptions {
    /// Maximum distinct instance attributes per class for `too-many-attributes` (`SMP711`). The rule counts
    /// attributes assigned by instance methods and fields declared by dataclasses.
    #[option(
        default = "12",
        value_type = "int",
        example = r#"
            max-instance-attributes = 7
        "#
    )]
    pub(crate) max_instance_attributes: Option<usize>,
}

impl ClassesOptions {
    fn combine(self, other: Self) -> Self {
        Self { max_instance_attributes: self.max_instance_attributes.or(other.max_instance_attributes) }
    }
}
