use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects};
use clap::builder::{PossibleValue, PossibleValuesParser, Styles, TypedValueParser};
use clap::error::{ContextKind, ContextValue};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use ruff::args::{ConfigArguments, FormatCommand, HelpFormat, LogLevelArgs, SingleConfigArgument};
use ruff_linter::logging::LogLevel;
use ruff_linter::settings::types::{OutputFormat, PatternPrefixPair, PythonVersion};
use ruff_linter::{RuleSelector, SuppressionKind};
use ruff_options_metadata::{OptionField, OptionSet, OptionsMetadata, Visit};
use serde::Deserialize;

use crate::lint::registry::Rule;
use crate::settings::{Configuration, Overrides, SimplintOptions, Source, ruff_selector};

// Ruff's help colors.
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    name = "simplint",
    version,
    about = "Ruff companion linter with extra checks that make Python easier to read and review",
    after_help = "For help with a specific command, see: `simplint help <command>`.",
    styles = STYLES
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
    #[command(flatten)]
    pub global_options: GlobalConfigArgs,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Run simplint on the given files or directories.
    Check(Box<CheckCommand>),
    /// Explain a rule (or all rules).
    #[command(group = ArgGroup::new("selector").multiple(false).required(true))]
    Rule {
        /// Rule to explain
        #[arg(value_parser = RuleParser, group = "selector", hide_possible_values = true)]
        rule: Option<Rule>,
        /// Explain all rules
        #[arg(long, conflicts_with = "rule", group = "selector")]
        all: bool,
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output_format: HelpFormat,
    },
    /// List every rule with its fix availability
    Rules {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output_format: HelpFormat,
    },
    /// List or describe the available configuration options.
    Config {
        /// Config key to show
        #[arg(value_parser = OptionParser, hide_possible_values = true)]
        option: Option<String>,
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output_format: HelpFormat,
    },
    /// List the rule groups.
    Linter {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        output_format: HelpFormat,
    },
    /// Remove simplint's caches from `.ruff_cache` directories in or below the current directory.
    Clean,
    /// Generate shell completion.
    #[command(hide = true)]
    GenerateShellCompletion { shell: clap_complete_command::Shell },
    /// Display simplint's version
    Version {
        #[arg(long, value_enum, default_value = "text")]
        output_format: HelpFormat,
    },
    /// Describe the command line as JSON, for the documentation build
    #[command(hide = true)]
    CliSchema,
    /// Print the JSON Schema of the `[tool.simplint]` table, for editors and `SchemaStore`
    #[command(hide = true)]
    ConfigSchema,
}

// Ruff's `GlobalConfigArgs`, except that `--config` also takes `[tool.simplint]` options.
#[derive(Args)]
pub struct GlobalConfigArgs {
    #[command(flatten)]
    log_level_args: LogLevelArgs,
    /// Read a TOML configuration file, or override one setting with a TOML `KEY = VALUE` pair. For example,
    /// `--config "fix = true"` sets a simplint option and `--config "line-length = 100"` sets a Ruff option.
    /// Inline settings override configuration files, including a file given with `--config`.
    #[arg(
        long,
        action = clap::ArgAction::Append,
        value_name = "CONFIG_OPTION",
        value_parser = ConfigArgumentParser,
        global = true,
        help_heading = "Global options",
    )]
    pub config: Vec<ConfigArgument>,
    /// Ignore Ruff and simplint configuration files.
    #[arg(long, help_heading = "Global options", global = true)]
    pub(crate) isolated: bool,
    /// Control when colored output is used.
    #[arg(long, value_name = "WHEN", help_heading = "Global options", global = true)]
    pub(crate) color: Option<TerminalColor>,
}

impl GlobalConfigArgs {
    pub(crate) fn log_level(&self) -> LogLevel {
        LogLevel::from(&self.log_level_args)
    }
}

/// Control when colored output is used.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum TerminalColor {
    /// Display colors if the output goes to an interactive terminal.
    Auto,
    /// Always display colors.
    Always,
    /// Never display colors.
    Never,
}

#[derive(Args)]
#[expect(clippy::struct_excessive_bools, reason = "each flag is a switch, as in `ruff check`")]
pub struct CheckCommand {
    /// List of files or directories to check, or `-` to read from stdin [default: .]
    pub files: Vec<PathBuf>,
    /// Apply fixes to resolve lint violations. Use `--no-fix` to disable or `--unsafe-fixes` to include unsafe fixes.
    #[arg(long, overrides_with("no_fix"))]
    fix: bool,
    #[arg(long, overrides_with("fix"), hide = true)]
    no_fix: bool,
    /// Include fixes that may not retain the original intent of the code. Use `--no-unsafe-fixes` to disable.
    #[arg(long, overrides_with("no_unsafe_fixes"))]
    unsafe_fixes: bool,
    #[arg(long, overrides_with("unsafe_fixes"), hide = true)]
    no_unsafe_fixes: bool,
    /// In text output, list the fixes applied by file and rule. Use `--no-show-fixes` to disable.
    #[arg(long, overrides_with("no_show_fixes"))]
    show_fixes: bool,
    #[arg(long, overrides_with("show_fixes"), hide = true)]
    no_show_fixes: bool,
    /// Print a diff of the proposed fixes to stdout and leave the files unchanged. Exit 1 if a diff is present,
    /// unless `--exit-zero` is set. Implies `--fix-only`.
    #[arg(long, conflicts_with = "show_fixes")]
    pub(crate) diff: bool,
    /// Recheck when a watched Python file, TOML file or hook configuration changes.
    #[arg(short, long)]
    pub(super) watch: bool,
    /// Apply fixes and omit remaining violations from the report and exit status. Implies `--fix`.
    /// Use `--no-fix-only` to disable or `--unsafe-fixes` to include unsafe fixes.
    #[arg(long, overrides_with("no_fix_only"))]
    fix_only: bool,
    #[arg(long, overrides_with("fix_only"), hide = true)]
    no_fix_only: bool,
    /// Report violations even when `noqa` or `simplint: ignore` comments suppress them.
    #[arg(long)]
    pub(super) ignore_noqa: bool,
    /// Format for reported violations. The default is `full`, which shows the source.
    #[arg(long, value_enum, env = "SIMPLINT_OUTPUT_FORMAT")]
    output_format: Option<OutputFormat>,
    /// Write the report to this file. The default is stdout, or stderr when sending fixed source to stdout.
    #[arg(short, long, env = "SIMPLINT_OUTPUT_FILE")]
    pub(super) output_file: Option<PathBuf>,
    /// The minimum Python version that should be supported.
    #[arg(long, value_enum)]
    target_version: Option<PythonVersion>,
    /// Enable Ruff's preview rules when checking fixes. Preview output uses rule names unless Ruff's
    /// `output-prefer-rule-codes` is set. simplint has no preview rules of its own. Use `--no-preview` to disable.
    #[arg(long, overrides_with("no_preview"))]
    preview: bool,
    #[arg(long, overrides_with("preview"), hide = true)]
    no_preview: bool,
    /// Enable rules by code, code prefix or name, separated by commas. `ALL` includes `unused-ignore`.
    /// Opt-in rules such as `init-module-content` require a full code or name.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    select: Option<Vec<String>>,
    /// Disable rules by code, code prefix or name, separated by commas.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    ignore: Option<Vec<String>>,
    /// Enable additional rules by code, code prefix or name.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    extend_select: Option<Vec<String>>,
    /// Deprecated alias for `--ignore`.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection", hide = true)]
    extend_ignore: Option<Vec<String>>,
    /// Ignore rules for matching files, for example `--per-file-ignores 'tests/*.py:SMP1'`.
    #[arg(long, value_delimiter = ',', help_heading = "Rule selection")]
    per_file_ignores: Option<Vec<PatternPrefixPair>>,
    /// Add file patterns and rule selectors to `--per-file-ignores`.
    #[arg(long, value_delimiter = ',', help_heading = "Rule selection")]
    extend_per_file_ignores: Option<Vec<PatternPrefixPair>>,
    /// Exclude files and directories that match these patterns.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN", help_heading = "File selection")]
    exclude: Option<Vec<String>>,
    /// Add patterns to the current exclusions.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN", help_heading = "File selection")]
    extend_exclude: Option<Vec<String>>,
    /// Allow fixes for rules matching these codes, code prefixes or names.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    fixable: Option<Vec<String>>,
    /// Hide and disable fixes for rules matching these codes, code prefixes or names.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    unfixable: Option<Vec<String>>,
    /// Allow fixes for additional rules by code, code prefix or name.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    extend_fixable: Option<Vec<String>>,
    /// Deprecated alias for `--unfixable`.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection", hide = true)]
    extend_unfixable: Option<Vec<String>>,
    /// Respect file exclusions via `.gitignore` and other standard ignore files. Use `--no-respect-gitignore` to
    /// disable.
    #[arg(long, overrides_with("no_respect_gitignore"), help_heading = "File selection")]
    respect_gitignore: bool,
    #[arg(long, overrides_with("respect_gitignore"), hide = true)]
    no_respect_gitignore: bool,
    /// Enforce exclusions, even for paths passed to simplint directly on the command-line. Use `--no-force-exclude` to
    /// disable.
    #[arg(long, overrides_with("no_force_exclude"), help_heading = "File selection")]
    force_exclude: bool,
    #[arg(long, overrides_with("force_exclude"), hide = true)]
    no_force_exclude: bool,
    /// Disable cache reads and writes.
    #[arg(short, long, env = "SIMPLINT_NO_CACHE", help_heading = "Miscellaneous")]
    pub(crate) no_cache: bool,
    /// Path to Ruff's cache directory, which simplint keeps its cache in.
    #[arg(long, env = "RUFF_CACHE_DIR", help_heading = "Miscellaneous")]
    pub(crate) cache_dir: Option<PathBuf>,
    /// The name of the file when passing it through stdin.
    #[arg(long, help_heading = "Miscellaneous")]
    pub(super) stdin_filename: Option<PathBuf>,
    /// Exit with status code "0", even upon detecting lint violations.
    #[arg(short, long, help_heading = "Miscellaneous", conflicts_with = "exit_non_zero_on_fix")]
    pub(super) exit_zero: bool,
    /// Exit with a non-zero status code if any files were modified via fix, even if no lint violations remain.
    #[arg(long, help_heading = "Miscellaneous", conflicts_with = "exit_zero")]
    pub(super) exit_non_zero_on_fix: bool,
    /// Count violations by rule. Supports `full`, `concise` and `json` output.
    #[arg(long, conflicts_with = "diff", conflicts_with = "watch")]
    pub(super) statistics: bool,
    /// Add `noqa` directives for reported violations. Use `--add-noqa=REASON` to explain the suppression.
    #[arg(
        long,
        value_name = "REASON",
        default_missing_value = "",
        num_args = 0..=1,
        require_equals = true,
        conflicts_with = "add_ignore",
        conflicts_with = "show_files",
        conflicts_with = "show_settings",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch",
        conflicts_with = "fix",
        conflicts_with = "diff",
    )]
    add_noqa: Option<String>,
    /// Add `simplint: ignore` comments with rule names for reported violations. Use `--add-ignore=REASON` to explain
    /// the suppression.
    #[arg(
        long,
        value_name = "REASON",
        default_missing_value = "",
        num_args = 0..=1,
        require_equals = true,
        conflicts_with = "add_noqa",
        conflicts_with = "show_files",
        conflicts_with = "show_settings",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch",
        conflicts_with = "fix",
        conflicts_with = "diff",
    )]
    add_ignore: Option<String>,
    /// List the files simplint would check with the current settings.
    #[arg(
        long,
        conflicts_with = "add_noqa",
        conflicts_with = "show_settings",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch"
    )]
    pub(super) show_files: bool,
    /// Show the resolved settings for the first Python file found.
    #[arg(
        long,
        conflicts_with = "add_noqa",
        conflicts_with = "show_files",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch"
    )]
    pub(super) show_settings: bool,
}

impl CheckCommand {
    // Ruff's `CheckCommand::partition`: the options of Ruff's own configuration go to Ruff's `ConfigArguments`, and the
    // rest override `[tool.simplint]`. Ruff keeps that partition private and builds its arguments only from parsed
    // command lines, so the Ruff options travel as the `ruff format` flags of the same names.
    pub fn partition(&self, global_options: &GlobalConfigArgs) -> Result<(ConfigArguments, SimplintArguments)> {
        let mut ruff_global = vec![OsString::from("ruff")];
        let mut config_file = None;
        let cwd = std::env::current_dir()?;
        let mut inline = Configuration::default();
        for argument in &global_options.config {
            match argument {
                ConfigArgument::File(path) => {
                    ruff_global.extend(["--config".into(), path.clone().into()]);
                    config_file.get_or_insert_with(|| path.clone());
                }
                ConfigArgument::Override { simplint, ruff } => {
                    ruff_global.extend(ruff.iter().flat_map(|ruff| ["--config".into(), ruff.into()]));
                    inline = inline.combine(Configuration::from_options(simplint.as_ref().clone(), &cwd, &Source::Cli));
                }
            }
        }
        if global_options.isolated {
            ruff_global.push("--isolated".into());
        }
        let mut format = vec![OsString::from("format")];
        let values = |flag: &str, values: &Option<Vec<String>>| {
            values.iter().flatten().map(|value| OsString::from(format!("--{flag}={value}"))).collect::<Vec<_>>()
        };
        format.extend(
            self.target_version
                .and_then(|version| version.to_possible_value())
                .map(|value| OsString::from(format!("--target-version={}", value.get_name()))),
        );
        format.extend(self.cache_dir.as_ref().map(|dir| {
            let mut flag = OsString::from("--cache-dir=");
            flag.push(dir);
            flag
        }));
        format.extend(values("exclude", &self.exclude));
        format.extend(values("extend-exclude", &self.extend_exclude));
        format.extend(switch("respect-gitignore", self.respect_gitignore, self.no_respect_gitignore));
        format.extend(switch("force-exclude", self.force_exclude, self.no_force_exclude));
        format.extend(switch("preview", self.preview, self.no_preview));
        // Ruff's parser checks again that a `--config` file exists, which a watched run may have removed since.
        let ruff_global = RuffGlobalConfigArgs::try_parse_from(ruff_global)?.global_options;
        // These flags passed simplint's parser, and Ruff's takes any value of theirs: a file pattern parses as a string.
        let (_, config_arguments) = FormatCommand::try_parse_from(format)
            .expect("Ruff's parser takes the flags simplint parsed")
            .partition(ruff_global)?;
        let arguments = SimplintArguments {
            config_file,
            isolated: global_options.isolated,
            overrides: Overrides {
                inline,
                flags: Configuration::from_options(self.simplint_options(), &cwd, &Source::Cli),
            },
            cwd,
        };
        Ok((config_arguments, arguments))
    }

    pub(crate) fn suppression(&self) -> Option<(&str, SuppressionKind)> {
        let noqa = self.add_noqa.as_deref().map(|reason| (reason, SuppressionKind::Noqa));
        noqa.or_else(|| self.add_ignore.as_deref().map(|reason| (reason, SuppressionKind::Ignore)))
    }

    fn simplint_options(&self) -> SimplintOptions {
        let chained = |first: &Option<Vec<String>>, second: &Option<Vec<String>>| {
            (first.is_some() || second.is_some())
                .then(|| first.iter().chain(second).flatten().cloned().collect::<Vec<_>>())
        };
        SimplintOptions {
            output_format: self.output_format,
            fix: resolve_bool_arg(self.fix, self.no_fix),
            unsafe_fixes: resolve_bool_arg(self.unsafe_fixes, self.no_unsafe_fixes),
            fix_only: resolve_bool_arg(self.fix_only, self.no_fix_only),
            show_fixes: resolve_bool_arg(self.show_fixes, self.no_show_fixes),
            select: self.select.clone(),
            extend_select: self.extend_select.clone(),
            ignore: chained(&self.ignore, &self.extend_ignore),
            fixable: self.fixable.clone(),
            extend_fixable: self.extend_fixable.clone(),
            unfixable: chained(&self.unfixable, &self.extend_unfixable),
            per_file_ignores: self.per_file_ignores.as_deref().map(collect_per_file_ignores),
            extend_per_file_ignores: self.extend_per_file_ignores.as_deref().map(collect_per_file_ignores),
            ..SimplintOptions::default()
        }
    }
}

fn switch(flag: &str, yes: bool, no: bool) -> Option<OsString> {
    resolve_bool_arg(yes, no)
        .map(|value| OsString::from(if value { format!("--{flag}") } else { format!("--no-{flag}") }))
}

fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

// Ruff's `collect_per_file_ignores`.
fn collect_per_file_ignores(pairs: &[PatternPrefixPair]) -> BTreeMap<String, Vec<String>> {
    let mut per_file_ignores: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for pair in pairs {
        // A selector serializes as the text it was given.
        let selector = serde_json::to_value(&pair.prefix).expect("a selector serializes as a string");
        per_file_ignores
            .entry(pair.pattern.clone())
            .or_default()
            .push(selector.as_str().unwrap_or_default().to_owned());
    }
    per_file_ignores
}

pub(super) fn suppression_flag(kind: SuppressionKind) -> &'static str {
    match kind {
        SuppressionKind::Noqa => "--add-noqa",
        SuppressionKind::Ignore => "--add-ignore",
    }
}

pub struct SimplintArguments {
    pub(crate) config_file: Option<PathBuf>,
    pub(crate) isolated: bool,
    pub(crate) overrides: Overrides,
    pub(crate) cwd: PathBuf,
}

#[derive(Parser)]
struct RuffGlobalConfigArgs {
    #[command(flatten)]
    global_options: ruff::args::GlobalConfigArgs,
}

#[derive(Clone)]
pub enum ConfigArgument {
    File(PathBuf),
    Override { simplint: Box<SimplintOptions>, ruff: Option<String> },
}

// Ruff's parser distinguishes file paths from inline TOML. Remove simplint-only keys before Ruff validates the rest.
// Shared keys such as `fix` configure simplint, while Ruff keeps keys such as `line-length`.
#[derive(Clone)]
struct ConfigArgumentParser;

impl TypedValueParser for ConfigArgumentParser {
    type Value = ConfigArgument;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let mut tables = toml::Table::new();
        let mut ruff_value = value.to_owned();
        if let Some(mut table) = value.to_str().and_then(|text| toml::Table::from_str(text).ok()) {
            // Ruff validates the rest, so it gets neither simplint's tables nor the options it has no namesake for.
            let ruff_options = ruff_workspace::options::Options::metadata();
            tables = option_names()
                .into_iter()
                .filter(|option| {
                    option.table || !option.name.contains('.') && ruff_options.find(&option.name).is_none()
                })
                .filter_map(|option| table.remove_entry(&option.name))
                .collect();
            if !tables.is_empty() {
                ruff_value = OsString::from(table.to_string());
            }
        }
        if let SingleConfigArgument::FilePath(path) =
            ruff::args::ConfigArgumentParser.parse_ref(cmd, arg, &ruff_value)?
        {
            return Ok(ConfigArgument::File(path));
        }
        let text = ruff_value.to_str().expect("Ruff accepts inline TOML only as UTF-8");
        let mut ruff = toml::Table::from_str(text).expect("Ruff accepted the value as TOML");
        let simplint: toml::Table = SimplintOptions::metadata()
            .collect_fields()
            .iter()
            .filter_map(|(key, _)| ruff.remove_entry(key))
            .chain(tables)
            .collect();
        // Ruff checked its own keys, but not simplint's tables.
        let simplint = SimplintOptions::deserialize(simplint).map_err(|error| {
            let mut invalid = clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(cmd);
            invalid.insert(
                ContextKind::InvalidArg,
                ContextValue::String(arg.map(ToString::to_string).unwrap_or_default()),
            );
            invalid.insert(ContextKind::InvalidValue, ContextValue::String(value.to_string_lossy().into_owned()));
            invalid.insert(ContextKind::Suggested, ContextValue::StyledStrs(vec![error.message().to_owned().into()]));
            invalid
        })?;
        Ok(ConfigArgument::Override {
            simplint: Box::new(simplint),
            ruff: (!ruff.is_empty()).then(|| ruff.to_string()),
        })
    }
}

// Ruff's `RuleParser`: a rule's code or name, checked without listing every choice when the value is not one.
#[derive(Clone)]
struct RuleParser;

impl TypedValueParser for RuleParser {
    type Value = Rule;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let values = Rule::ALL.iter().flat_map(|rule| {
            [PossibleValue::new(rule.code()).help(rule.name()), PossibleValue::new(rule.name()).help(rule.code())]
        });
        let parsed = PossibleValuesParser::new(values)
            .map(|value| Rule::from_code_or_name(&value).expect("a choice"))
            .parse_ref(cmd, arg, value);
        without_choices(parsed).map_err(|mut error| {
            let value = value.to_string_lossy();
            if let Some(RuleSelector::Rule { .. }) = ruff_selector(&value) {
                error.remove(ContextKind::SuggestedValue);
                let tip = format!("`{value}` is a Ruff rule; run `ruff rule {value}`");
                error.insert(ContextKind::Suggested, ContextValue::StyledStrs(vec![tip.into()]));
            }
            error
        })
    }
}

// Ruff's `OptionStringParser`: the name of a `[tool.simplint]` option.
#[derive(Clone)]
struct OptionParser;

impl TypedValueParser for OptionParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let values = option_names().into_iter().map(|option| PossibleValue::new(option.name).help(option.summary));
        without_choices(PossibleValuesParser::new(values).parse_ref(cmd, arg, value))
    }
}

// Every `[tool.simplint]` option and table, by its dotted name, with the first line of its documentation.
fn option_names() -> Vec<OptionName> {
    let mut names = OptionNames { prefix: String::new(), names: Vec::new() };
    SimplintOptions::metadata().record(&mut names);
    names.names
}

struct OptionName {
    name: String,
    summary: String,
    table: bool,
}

struct OptionNames {
    prefix: String,
    names: Vec<OptionName>,
}

impl Visit for OptionNames {
    fn record_field(&mut self, name: &str, field: OptionField) {
        let summary = field.doc.lines().next().unwrap_or_default().to_owned();
        self.names.push(OptionName { name: format!("{}{name}", self.prefix), summary, table: false });
    }

    fn record_set(&mut self, name: &str, set: OptionSet) {
        let name = format!("{}{name}", self.prefix);
        let summary = set.documentation().and_then(|doc| doc.lines().next()).unwrap_or_default().to_owned();
        let mut nested = Self { prefix: format!("{name}."), names: Vec::new() };
        set.record(&mut nested);
        self.names.push(OptionName { name, summary, table: true });
        self.names.extend(nested.names);
    }
}

fn without_choices<T>(result: Result<T, clap::Error>) -> Result<T, clap::Error> {
    result.map_err(|mut error| {
        error.remove(ContextKind::ValidValue);
        error
    })
}

#[cfg(test)]
mod tests {
    use clap::{Args, FromArgMatches};

    use super::{CheckCommand, GlobalConfigArgs};

    // A watched run partitions the arguments again after each change, by when the `--config` file may be gone.
    #[test]
    fn configuration_file_removed_since_parsing_fails_the_partition() {
        let directory = tempfile::TempDir::new().expect("temporary directory");
        let config = directory.path().join("ruff.toml");
        std::fs::write(&config, "line-length = 100\n").expect("write the configuration");
        let matches = GlobalConfigArgs::augment_args(CheckCommand::augment_args(clap::Command::new("check")))
            .get_matches_from(["check".as_ref(), "--config".as_ref(), config.as_os_str()]);
        let check = CheckCommand::from_arg_matches(&matches).expect("the check arguments parse");
        let global_options = GlobalConfigArgs::from_arg_matches(&matches).expect("the global arguments parse");
        std::fs::remove_file(&config).expect("remove the configuration");

        assert!(check.partition(&global_options).is_err());
    }
}
