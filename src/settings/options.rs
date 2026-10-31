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
use serde::Deserialize;

use crate::lint::registry::RuleSet;

// Ruff's `Configuration` for simplint's options: one layer of settings, before it is resolved, so the command line can
// go on top of a configuration file the way Ruff combines them.
#[derive(Clone, Default)]
pub struct Configuration {
    pub(super) rule_selections: Vec<RuleSelection>,
    pub(super) per_file_ignores: Option<Vec<PerFileIgnoreSource>>,
    pub(super) extend_per_file_ignores: Vec<PerFileIgnoreSource>,
    pub(super) extend_safe_fixes: Vec<Selector>,
    pub(super) extend_unsafe_fixes: Vec<Selector>,
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
            if RuleSelector::from_str(text).is_ok() || ruff_linter::registry::Rule::from_name(text).is_ok() {
                anyhow!("{unknown}; it selects Ruff rules, so select it in Ruff's configuration instead")
            } else {
                anyhow!(unknown)
            }
        })
    }
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
#[derive(Deserialize, Default, Clone, OptionsMetadata)]
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

    /// A list of rule codes, prefixes or names to enable. Prefixes can specify exact rules (like `SMP101`), entire
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
    pub extend_select: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to ignore. Prefixes can specify exact rules (like `SMP101`), entire
    /// groups (like `SMP1`), or anything in between.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Skip the missing `Final` rule (`SMP101`).
            ignore = ["SMP101"]
        "#
    )]
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
    pub extend_fixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider non-fixable.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Disable fix for the dataclass rules (`SMP111`, `SMP114`).
            unfixable = ["SMP111", "SMP114"]
        "#
    )]
    pub unfixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names to consider non-fixable, in addition to those specified by
    /// [`unfixable`](#unfixable).
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Also disable fix for the walrus rule (`SMP100`).
            extend-unfixable = ["SMP100"]
        "#
    )]
    pub extend_unfixable: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names for which unsafe fixes should be considered safe.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Allow applying the unsafe fixes of `SMP101` without the `--unsafe-fixes` flag.
            extend-safe-fixes = ["SMP101"]
        "#
    )]
    pub extend_safe_fixes: Option<Vec<String>>,

    /// A list of rule codes, prefixes or names for which safe fixes should be considered unsafe.
    #[option(
        default = "[]",
        value_type = "list[RuleSelector]",
        example = r#"
            # Require the `--unsafe-fixes` flag when fixing `SMP111`.
            extend-unsafe-fixes = ["SMP111"]
        "#
    )]
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
            # Ignore `SMP101` (missing `Final`) in all `__init__.py` files, and in `path/to/file.py`.
            "__init__.py" = ["SMP101"]
            "path/to/file.py" = ["SMP101"]
            # Ignore the test rules everywhere except for the `tests/` directory.
            "!tests/**.py" = ["SMP2"]
        "#
    )]
    pub per_file_ignores: Option<BTreeMap<String, Vec<String>>>,

    /// A list of mappings from file pattern to rule codes, prefixes or names to exclude, in addition to any rules
    /// excluded by [`per-file-ignores`](#per-file-ignores).
    #[option(
        default = "{}",
        value_type = "dict[str, list[RuleSelector]]",
        scope = "extend-per-file-ignores",
        example = r#"
            # Also ignore `SMP101` in all `__init__.py` files.
            "__init__.py" = ["SMP101"]
        "#
    )]
    pub extend_per_file_ignores: Option<BTreeMap<String, Vec<String>>>,
}
