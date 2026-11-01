// Ports of the `ruff` crate's informational subcommands, which it keeps private, over simplint's rules and options.
use std::io::{self, BufWriter, Write};

use anyhow::Result;
use clap::ArgAction;
use colored::Colorize;
use ruff::args::HelpFormat;
use ruff_cache::CACHE_DIR_NAME;
use ruff_linter::FixAvailability;
use ruff_linter::fs::relativize_path;
use ruff_linter::logging::LogLevel;
use ruff_options_metadata::{OptionEntry, OptionSet, OptionsMetadata};
use serde::Serialize;
use walkdir::WalkDir;

use crate::lint::registry::{GROUPS, Rule};
use crate::project::cache;
use crate::settings::SimplintOptions;

pub fn version(format: HelpFormat) -> Result<()> {
    print(&VersionInfo { version: env!("CARGO_PKG_VERSION"), commit_info: None }, format)
}

pub fn rule(rule: Rule, format: HelpFormat) -> Result<()> {
    print(&Explanation::from_rule(rule), format)
}

pub fn rules(format: HelpFormat) -> Result<()> {
    print(&Rule::ALL.iter().copied().map(Explanation::from_rule).collect::<Vec<_>>(), format)
}

// simplint's own listing of its rules, shorter than `rule --all`.
pub fn rule_list(format: HelpFormat) -> Result<()> {
    print(&Rule::ALL.iter().copied().map(RuleEntry::from_rule).collect::<Vec<_>>(), format)
}

// An option, or every option when `key` names none.
pub fn config(key: Option<&str>, format: HelpFormat) -> Result<()> {
    let metadata = SimplintOptions::metadata();
    print(&key.and_then(|key| metadata.find(key)).map_or(ConfigOutput::All(metadata), ConfigOutput::Entry), format)
}

pub fn linter(format: HelpFormat) -> Result<()> {
    print(&GROUPS.iter().map(|&(prefix, name)| LinterInfo { prefix, name }).collect::<Vec<_>>(), format)
}

// The documentation rebuilds this command line as an argparse parser for sphinx-argparse-cli, so its reference is
// generated from the definition the binary parses with.
pub fn cli_schema(command: &clap::Command) -> Result<()> {
    let mut stdout = BufWriter::new(std::io::stdout().lock());
    writeln!(stdout, "{}", serde_json::to_string_pretty(&command_schema(command))?)?;
    stdout.flush()?;
    Ok(())
}

// An informational subcommand's output, which Ruff prints as text or as pretty JSON.
trait HelpOutput: Serialize {
    // Ruff ends some JSON outputs with a newline and others not.
    const JSON_NEWLINE: bool;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()>;
}

fn print<T: HelpOutput>(output: &T, format: HelpFormat) -> Result<()> {
    let mut stdout = BufWriter::new(std::io::stdout().lock());
    match format {
        HelpFormat::Text => output.write_text(&mut stdout)?,
        HelpFormat::Json => {
            serde_json::to_writer_pretty(&mut stdout, output)?;
            if T::JSON_NEWLINE {
                writeln!(stdout)?;
            }
        }
    }
    stdout.flush()?;
    Ok(())
}

// Ruff's `VersionInfo`. simplint records no commit, which Ruff reports as `null` when built outside a git checkout.
#[derive(Serialize)]
struct VersionInfo {
    version: &'static str,
    commit_info: Option<()>,
}

impl HelpOutput for VersionInfo {
    const JSON_NEWLINE: bool = false;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        writeln!(writer, "simplint {}", self.version)
    }
}

// Ruff's `Explanation`, without the fields simplint does not track: message formats, status, category and source
// location.
#[derive(Serialize)]
#[expect(clippy::struct_field_names, reason = "the fields are Ruff's JSON keys")]
struct Explanation {
    name: &'static str,
    code: &'static str,
    linter: &'static str,
    summary: &'static str,
    fix: String,
    fix_availability: FixAvailability,
    explanation: &'static str,
    preview: bool,
}

impl Explanation {
    fn from_rule(rule: Rule) -> Self {
        let fix_availability = rule.fix_availability();
        Self {
            name: rule.name(),
            code: rule.code(),
            linter: rule.group(),
            summary: rule.summary(),
            fix: fix_availability.to_string(),
            fix_availability,
            explanation: rule.explanation(),
            preview: false,
        }
    }

    // Ruff's `format_rule_text`. A terminal cannot draw the documentation's mermaid diagrams, so their source is left
    // out.
    fn text(&self) -> String {
        let mut output = format!("# {} ({})\n\n", self.name, self.code);
        if !matches!(self.fix_availability, FixAvailability::None) {
            output.push_str(&self.fix);
            output.push_str("\n\n");
        }
        output.push_str(without_diagrams(self.explanation).trim());
        output
    }
}

fn without_diagrams(explanation: &str) -> String {
    let mut in_diagram = false;
    let mut kept = String::new();
    for line in explanation.split_inclusive('\n') {
        if line.trim_end() == "```mermaid" {
            in_diagram = true;
        } else if in_diagram && line.trim_end() == "```" {
            in_diagram = false;
        } else if !in_diagram {
            kept.push_str(line);
        }
    }
    kept
}

impl HelpOutput for Explanation {
    const JSON_NEWLINE: bool = false;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        writeln!(writer, "{}", self.text())
    }
}

impl HelpOutput for Vec<Explanation> {
    const JSON_NEWLINE: bool = false;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        for explanation in self {
            writeln!(writer, "{}\n", explanation.text())?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct RuleEntry {
    code: &'static str,
    name: &'static str,
    summary: &'static str,
    fix: FixAvailability,
    explanation: &'static str,
}

impl RuleEntry {
    fn from_rule(rule: Rule) -> Self {
        Self {
            code: rule.code(),
            name: rule.name(),
            summary: rule.summary(),
            fix: rule.fix_availability(),
            explanation: rule.explanation(),
        }
    }
}

impl HelpOutput for Vec<RuleEntry> {
    const JSON_NEWLINE: bool = true;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        for entry in self {
            let fix = match entry.fix {
                FixAvailability::Always => " (fix)",
                FixAvailability::Sometimes => " (fix sometimes)",
                FixAvailability::None => "",
            };
            writeln!(writer, "{} {}{fix}", entry.code, entry.name)?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ConfigOutput {
    All(OptionSet),
    Entry(OptionEntry),
}

impl HelpOutput for ConfigOutput {
    const JSON_NEWLINE: bool = true;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        match self {
            Self::All(metadata) => writeln!(writer, "{metadata}"),
            Self::Entry(entry) => write!(writer, "{entry}"),
        }
    }
}

#[derive(Serialize)]
struct LinterInfo {
    prefix: &'static str,
    name: &'static str,
}

impl HelpOutput for Vec<LinterInfo> {
    const JSON_NEWLINE: bool = true;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        for LinterInfo { prefix, name } in self {
            writeln!(writer, "{prefix:>4} {name}")?;
        }
        Ok(())
    }
}

fn command_schema(command: &clap::Command) -> serde_json::Value {
    let arguments: Vec<serde_json::Value> = command
        .get_arguments()
        .filter(|argument| !matches!(argument.get_action(), ArgAction::Help | ArgAction::Version))
        .map(|argument| {
            serde_json::json!({
                "id": argument.get_id().as_str(),
                "long": argument.get_long(),
                "short": argument.get_short(),
                "help": argument.get_help().map(ToString::to_string),
                "flag": matches!(argument.get_action(), ArgAction::SetTrue),
                "multiple": argument.get_num_args().is_some_and(|range| range.max_values() > 1),
                "delimiter": argument.get_value_delimiter(),
                "default": argument.get_default_values().iter().map(|value| value.to_string_lossy()).collect::<Vec<_>>(),
                "choices": argument.get_possible_values().iter().map(|value| value.get_name().to_owned()).collect::<Vec<_>>(),
                "value_name": argument.get_value_names().and_then(|names| names.first()).map(ToString::to_string),
            })
        })
        .collect();
    let subcommands: Vec<serde_json::Value> =
        command.get_subcommands().filter(|subcommand| !subcommand.is_hide_set()).map(command_schema).collect();
    serde_json::json!({
        "name": command.get_name(),
        "about": command.get_about().map(ToString::to_string),
        "arguments": arguments,
        "subcommands": subcommands,
    })
}

// Ruff's `clean`, for simplint's directory in each Ruff cache directory, which leaves Ruff's own cache alone.
pub fn clean(level: LogLevel) -> Result<()> {
    let mut stderr = BufWriter::new(std::io::stderr().lock());
    for entry in WalkDir::new(std::env::current_dir()?)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
    {
        let cache = cache::directory(&entry.path().join(CACHE_DIR_NAME));
        if cache.is_dir() {
            if level >= LogLevel::Default {
                writeln!(stderr, "Removing cache at: {}", relativize_path(&cache).bold())?;
            }
            std::fs::remove_dir_all(&cache)?;
        }
    }
    stderr.flush()?;
    Ok(())
}
