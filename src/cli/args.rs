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
use ruff_linter::SuppressionKind;
use ruff_linter::logging::LogLevel;
use ruff_linter::settings::types::{OutputFormat, PatternPrefixPair, PythonVersion};
use ruff_options_metadata::OptionsMetadata;
use serde::Deserialize;

use crate::lint::registry::Rule;
use crate::settings::{Configuration, Overrides, SimplintOptions, Source};

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
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    #[command(flatten)]
    pub global_options: GlobalConfigArgs,
}

// The subcommands mirror `ruff`'s, with the same names, flags and help text.
#[derive(Subcommand)]
pub enum Command {
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
    /// Clear simplint's caches from the Ruff cache directories in the current directory and any subdirectories.
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
}

// Ruff's `GlobalConfigArgs`, except that `--config` also takes `[tool.simplint]` options.
#[derive(Args)]
pub struct GlobalConfigArgs {
    #[command(flatten)]
    pub log_level_args: LogLevelArgs,
    /// Either a path to a TOML configuration file (`pyproject.toml` or `ruff.toml`), or a TOML `<KEY> = <VALUE>` pair
    /// (such as you might find in a `ruff.toml` configuration file or a `[tool.simplint]` table) overriding a specific
    /// configuration option (e.g., `--config "fix = true"` or `--config "line-length = 100"`).
    /// Overrides of individual settings using this option always take precedence over all configuration files,
    /// including configuration files that were also specified using `--config`.
    #[arg(
        long,
        action = clap::ArgAction::Append,
        value_name = "CONFIG_OPTION",
        value_parser = ConfigArgumentParser,
        global = true,
        help_heading = "Global options",
    )]
    pub config: Vec<ConfigArgument>,
    /// Ignore all configuration files.
    #[arg(long, help_heading = "Global options", global = true)]
    pub isolated: bool,
    /// Control when colored output is used.
    #[arg(long, value_name = "WHEN", help_heading = "Global options", global = true)]
    pub color: Option<TerminalColor>,
}

impl GlobalConfigArgs {
    pub fn log_level(&self) -> LogLevel {
        LogLevel::from(&self.log_level_args)
    }
}

/// Control when colored output is used.
#[derive(Clone, Copy, ValueEnum)]
pub enum TerminalColor {
    /// Display colors if the output goes to an interactive terminal.
    Auto,
    /// Always display colors.
    Always,
    /// Never display colors.
    Never,
}

// The flags mirror `ruff check`'s, with the same names, defaults, help text and conflicts.
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
    /// Show an enumeration of all fixed lint violations. Use `--no-show-fixes` to disable.
    #[arg(long, overrides_with("no_show_fixes"))]
    show_fixes: bool,
    #[arg(long, overrides_with("show_fixes"), hide = true)]
    no_show_fixes: bool,
    /// Avoid writing any fixed files back; instead, output a diff for each changed file to stdout, and exit 0 if there
    /// are no diffs. Implies `--fix-only`.
    #[arg(long, conflicts_with = "show_fixes")]
    pub diff: bool,
    /// Run in watch mode by re-running whenever files change.
    #[arg(short, long)]
    pub watch: bool,
    /// Apply fixes to resolve lint violations, but don't report on, or exit non-zero for, leftover violations. Implies
    /// `--fix`. Use `--no-fix-only` to disable or `--unsafe-fixes` to include unsafe fixes.
    #[arg(long, overrides_with("no_fix_only"))]
    fix_only: bool,
    #[arg(long, overrides_with("fix_only"), hide = true)]
    no_fix_only: bool,
    /// Ignore any `# noqa` comments.
    #[arg(long)]
    pub ignore_noqa: bool,
    /// Output serialization format for violations. The default serialization format is "full".
    #[arg(long, value_enum, env = "SIMPLINT_OUTPUT_FORMAT")]
    output_format: Option<OutputFormat>,
    /// Specify file to write the linter output to (default: stdout).
    #[arg(short, long, env = "SIMPLINT_OUTPUT_FILE")]
    pub output_file: Option<PathBuf>,
    /// The minimum Python version that should be supported.
    #[arg(long, value_enum)]
    target_version: Option<PythonVersion>,
    /// Enable Ruff's preview mode: reports name rules instead of codes, and simplint's fixes are checked against
    /// Ruff's preview rules too. simplint has no preview rules of its own. Use `--no-preview` to disable.
    #[arg(long, overrides_with("no_preview"))]
    preview: bool,
    #[arg(long, overrides_with("preview"), hide = true)]
    no_preview: bool,
    /// Comma-separated list of rule codes to enable (or ALL, to enable all rules).
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    select: Option<Vec<String>>,
    /// Comma-separated list of rule codes to disable.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    ignore: Option<Vec<String>>,
    /// Like --select, but adds additional rule codes on top of those already specified.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    extend_select: Option<Vec<String>>,
    /// Like --ignore. (Deprecated: You can just use --ignore instead.)
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection", hide = true)]
    extend_ignore: Option<Vec<String>>,
    /// List of mappings from file pattern to code to exclude.
    #[arg(long, value_delimiter = ',', help_heading = "Rule selection")]
    per_file_ignores: Option<Vec<PatternPrefixPair>>,
    /// Like `--per-file-ignores`, but adds additional ignores on top of those already specified.
    #[arg(long, value_delimiter = ',', help_heading = "Rule selection")]
    extend_per_file_ignores: Option<Vec<PatternPrefixPair>>,
    /// List of paths, used to omit files and/or directories from analysis.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN", help_heading = "File selection")]
    exclude: Option<Vec<String>>,
    /// Like --exclude, but adds additional files and directories on top of those already excluded.
    #[arg(long, value_delimiter = ',', value_name = "FILE_PATTERN", help_heading = "File selection")]
    extend_exclude: Option<Vec<String>>,
    /// List of rule codes to treat as eligible for fix. Only applicable when fix itself is enabled (e.g., via
    /// `--fix`).
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    fixable: Option<Vec<String>>,
    /// List of rule codes to treat as ineligible for fix. Only applicable when fix itself is enabled (e.g., via
    /// `--fix`).
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    unfixable: Option<Vec<String>>,
    /// Like --fixable, but adds additional rule codes on top of those already specified.
    #[arg(long, value_delimiter = ',', value_name = "RULE_CODE", help_heading = "Rule selection")]
    extend_fixable: Option<Vec<String>>,
    /// Like --unfixable. (Deprecated: You can just use --unfixable instead.)
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
    /// Disable cache reads.
    #[arg(short, long, env = "SIMPLINT_NO_CACHE", help_heading = "Miscellaneous")]
    pub no_cache: bool,
    /// Path to Ruff's cache directory, which simplint keeps its cache in.
    #[arg(long, env = "RUFF_CACHE_DIR", help_heading = "Miscellaneous")]
    cache_dir: Option<PathBuf>,
    /// The name of the file when passing it through stdin.
    #[arg(long, help_heading = "Miscellaneous")]
    pub stdin_filename: Option<PathBuf>,
    /// Exit with status code "0", even upon detecting lint violations.
    #[arg(short, long, help_heading = "Miscellaneous", conflicts_with = "exit_non_zero_on_fix")]
    pub exit_zero: bool,
    /// Exit with a non-zero status code if any files were modified via fix, even if no lint violations remain.
    #[arg(long, help_heading = "Miscellaneous", conflicts_with = "exit_zero")]
    pub exit_non_zero_on_fix: bool,
    /// Show counts for every rule with at least one violation.
    #[arg(long, conflicts_with = "diff", conflicts_with = "watch")]
    pub statistics: bool,
    /// Enable automatic additions of `noqa` directives to failing lines. Optionally provide a reason to append after
    /// the codes.
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
    /// Enable automatic additions of `simplint: ignore` comments to failing lines. Optionally provide a reason to
    /// append after the codes.
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
    /// See the files simplint will be run against with the current settings.
    #[arg(
        long,
        conflicts_with = "add_noqa",
        conflicts_with = "show_settings",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch"
    )]
    pub show_files: bool,
    /// See the settings simplint will use to lint a given Python file.
    #[arg(
        long,
        conflicts_with = "add_noqa",
        conflicts_with = "show_files",
        conflicts_with = "ignore_noqa",
        conflicts_with = "statistics",
        conflicts_with = "stdin_filename",
        conflicts_with = "watch"
    )]
    pub show_settings: bool,
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
        let ruff_global = RuffGlobalConfigArgs::try_parse_from(ruff_global)?.global_options;
        let (_, config_arguments) = FormatCommand::try_parse_from(format)?.partition(ruff_global)?;
        let overrides =
            Overrides { inline, flags: Configuration::from_options(self.simplint_options(), &cwd, &Source::Cli) };
        let arguments = SimplintArguments { config_file, isolated: global_options.isolated, overrides };
        Ok((config_arguments, arguments))
    }

    // The suppression comments `--add-noqa` or `--add-ignore` asks for: the reason and the kind.
    pub fn suppression(&self) -> Option<(&str, SuppressionKind)> {
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
            extend_unfixable: None,
            extend_safe_fixes: None,
            extend_unsafe_fixes: None,
            per_file_ignores: self.per_file_ignores.as_deref().map(collect_per_file_ignores),
            extend_per_file_ignores: self.extend_per_file_ignores.as_deref().map(collect_per_file_ignores),
        }
    }
}

// The flag that asks for a kind of suppression comment.
pub fn suppression_flag(kind: SuppressionKind) -> &'static str {
    match kind {
        SuppressionKind::Noqa => "--add-noqa",
        SuppressionKind::Ignore => "--add-ignore",
    }
}

// What the command line says about `[tool.simplint]`: where to read it from, and what overrides it.
pub struct SimplintArguments {
    pub config_file: Option<PathBuf>,
    pub isolated: bool,
    pub overrides: Overrides,
}

#[derive(Parser)]
struct RuffGlobalConfigArgs {
    #[command(flatten)]
    global_options: ruff::args::GlobalConfigArgs,
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

// One `--config` value: a configuration file, or inline options split into simplint's and Ruff's.
#[derive(Clone)]
pub enum ConfigArgument {
    File(PathBuf),
    Override { simplint: Box<SimplintOptions>, ruff: Option<String> },
}

// Ruff's parser decides between a file and inline TOML and reports invalid values with its own messages. Every
// `[tool.simplint]` key is also a (deprecated, top-level) `ruff.toml` key of the same type, so what Ruff accepts, the
// `[tool.simplint]` table accepts too; those keys go to simplint, and only the rest to Ruff.
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
        if let SingleConfigArgument::FilePath(path) = ruff::args::ConfigArgumentParser.parse_ref(cmd, arg, value)? {
            return Ok(ConfigArgument::File(path));
        }
        let text = value.to_str().expect("Ruff accepts inline TOML only as UTF-8");
        let mut ruff = toml::Table::from_str(text).expect("Ruff accepted the value as TOML");
        let simplint: toml::Table =
            SimplintOptions::metadata().collect_fields().iter().filter_map(|(key, _)| ruff.remove_entry(key)).collect();
        let simplint = SimplintOptions::deserialize(simplint).expect("Ruff accepted these keys with the same types");
        Ok(ConfigArgument::Override {
            simplint: Box::new(simplint),
            ruff: (!ruff.is_empty()).then(|| ruff.to_string()),
        })
    }
}

// Ruff's `RuleParser`: a rule by code or name, without listing every choice when the value is not one.
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
            if ruff_linter::registry::Rule::from_code(&value).is_ok()
                || ruff_linter::registry::Rule::from_name(&value).is_ok()
            {
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
        let fields = SimplintOptions::metadata().collect_fields();
        let values = fields
            .iter()
            .map(|(name, field)| PossibleValue::new(name).help(field.doc.lines().next().unwrap_or_default()));
        without_choices(PossibleValuesParser::new(values).parse_ref(cmd, arg, value))
    }
}

fn without_choices<T>(result: Result<T, clap::Error>) -> Result<T, clap::Error> {
    result.map_err(|mut error| {
        error.remove(ContextKind::ValidValue);
        error
    })
}
