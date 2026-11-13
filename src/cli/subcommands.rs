// Ports of the `ruff` crate's informational subcommands, which it keeps private, over simplint's rules and options.
use std::io::{self, BufWriter, Write};
use std::path::Path;

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

use crate::lint::examples::{Block, Example, Examples, Fence, Role, parse};
use crate::lint::registry::{GROUPS, Rule};
use crate::project::cache;
use crate::settings::SimplintOptions;

pub(crate) fn version(format: HelpFormat) -> Result<()> {
    print(&VersionInfo { version: env!("CARGO_PKG_VERSION"), commit_info: None }, format)
}

pub(crate) fn rule(rule: Rule, format: HelpFormat) -> Result<()> {
    print(&Explanation::from_rule(rule), format)
}

pub fn rules(format: HelpFormat) -> Result<()> {
    print(&Rule::ALL.iter().copied().map(Explanation::from_rule).collect::<Vec<_>>(), format)
}

// simplint's own listing of its rules, shorter than `rule --all`.
pub(crate) fn rule_list(format: HelpFormat) -> Result<()> {
    print(&Rule::ALL.iter().copied().map(RuleEntry::from_rule).collect::<Vec<_>>(), format)
}

pub(crate) fn config(key: Option<&str>, format: HelpFormat) -> Result<()> {
    let metadata = SimplintOptions::metadata();
    print(&key.and_then(|key| metadata.find(key)).map_or(ConfigOutput::All(metadata), ConfigOutput::Entry), format)
}

pub(crate) fn linter(format: HelpFormat) -> Result<()> {
    let groups = GROUPS.iter().map(|&(family, name, _)| LinterInfo {
        name,
        codes: Rule::ALL.iter().filter(|rule| rule.family() == family).map(|rule| rule.code()).collect(),
    });
    print(&groups.collect::<Vec<_>>(), format)
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

trait HelpOutput: Serialize {
    // Ruff ends some JSON outputs with a newline and others not.
    const JSON_NEWLINE: bool;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()>;
}

// The documentation rebuilds this command line as an argparse parser for sphinx-argparse-cli, so its reference is
// generated from the definition the binary parses with.
pub(crate) fn cli_schema(command: &clap::Command) -> Result<()> {
    write_json(&command_schema(command))
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
                "default": argument
                    .get_default_values()
                    .iter()
                    .map(|value| value.to_string_lossy())
                    .collect::<Vec<_>>(),
                "choices": argument
                    .get_possible_values()
                    .iter()
                    .map(|value| value.get_name().to_owned())
                    .collect::<Vec<_>>(),
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

fn write_json(value: &impl Serialize) -> Result<()> {
    let mut stdout = BufWriter::new(std::io::stdout().lock());
    writeln!(stdout, "{}", serde_json::to_string_pretty(value)?)?;
    stdout.flush()?;
    Ok(())
}

// The JSON Schema of `[tool.simplint]`, committed as `simplint.schema.json` and checked by `tests/docs_data.rs`.
pub fn config_schema() -> Result<()> {
    write_json(&crate::settings::config_schema())
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
    // simplint's own key: whether other files of the run can change what the rule reports.
    cross_file: bool,
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
            cross_file: rule.reads_other_files(),
        }
    }

    // Ruff's `format_rule_text`.
    fn text(&self) -> String {
        let mut output = format!("# {} ({})\n\n", self.name, self.code);
        if !matches!(self.fix_availability, FixAvailability::None) {
            output.push_str(&self.fix);
            output.push_str("\n\n");
        }
        if self.cross_file {
            output.push_str("Reads other files of the run.\n\n");
        }
        let examples = parse(self.explanation).expect("every rule page parses; tests/examples.rs checks each one");
        output.push_str(plain_examples(self.explanation, &examples).trim());
        output
    }
}

// Shows each example as plain Python under a label, leaving the role and attributes out of the fence. A file name
// becomes a leading comment, and context files, which only the example's other files refer to, get neither a label nor
// a place in the Example section, which already tells its flagged code from the code to use instead. A terminal cannot
// draw the documentation's mermaid diagrams, so their source is left out too.
fn plain_examples(explanation: &str, examples: &[Example]) -> String {
    let lines: Vec<&str> = explanation.lines().collect();
    let mut shown = examples.iter().flat_map(shown_fences).peekable();
    let mut kept = String::new();
    // A hidden fence and a diagram take the blank lines after them with them.
    let mut after_hidden = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(fence) = shown.next_if(|fence| fence.fence.line == index + 1) {
            index = fence.fence.end_line;
            after_hidden = fence.hidden;
            if !fence.hidden {
                push_fence(&mut kept, &lines, &fence);
            }
        } else if line.trim_end() == "```mermaid" {
            index = lines[index..].iter().position(|line| line.trim_end() == "```").expect("a diagram is closed")
                + index
                + 1;
            after_hidden = true;
        } else if after_hidden && line.trim().is_empty() {
            index += 1;
        } else {
            after_hidden = false;
            kept.push_str(line);
            kept.push('\n');
            index += 1;
        }
    }
    kept
}

// A fence as the plain output shows it. A config the Example section shares from above belongs to the example that
// opens with it, so only an example's own config is shown.
struct Shown<'a> {
    fence: &'a Fence,
    language: &'a str,
    label: Option<&'static str>,
    comment: Option<String>,
    hidden: bool,
}

fn shown_fences(example: &Example) -> impl Iterator<Item = Shown<'_>> {
    let in_example = example.section.as_deref() == Some("Example");
    let config = example.config.as_ref().filter(|config| config.line == example.line).map(|config| Shown {
        fence: config,
        language: "toml",
        label: Some("With this configuration:"),
        comment: None,
        hidden: false,
    });
    let blocks = example.blocks.iter().map(move |block| Shown {
        fence: &block.fence,
        language: block.language.as_str(),
        label: if in_example { None } else { example_label(block.role) },
        comment: block_comment(block),
        hidden: in_example && block.role == Role::Context,
    });
    config.into_iter().chain(blocks)
}

fn example_label(role: Role) -> Option<&'static str> {
    match role {
        Role::Report => Some("Reported:"),
        Role::Ok => Some("Not reported:"),
        Role::Fixed => Some("After the fix:"),
        Role::Context => None,
    }
}

fn block_comment(block: &Block) -> Option<String> {
    let mut parts = Vec::new();
    if block.named {
        parts.push(block.file.clone());
    }
    if !block.select.is_empty() {
        parts.push(format!("with {} selected", block.select.join(", ")));
    }
    if block.executable {
        parts.push("(executable)".to_owned());
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn push_fence(kept: &mut String, lines: &[&str], shown: &Shown) {
    let header = lines[shown.fence.line - 1];
    let indent = &header[..header.len() - header.trim_start().len()];
    if let Some(label) = shown.label {
        kept.extend([indent, label, "\n"]);
    }
    kept.extend([indent, "```", shown.language, "\n"]);
    if let Some(comment) = &shown.comment {
        kept.extend([indent, "# ", comment.as_str(), "\n"]);
    }
    for &line in &lines[shown.fence.line..shown.fence.end_line - 1] {
        kept.extend([line, "\n"]);
    }
    kept.extend([lines[shown.fence.end_line - 1], "\n"]);
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
    examples: Examples,
    family: &'static str,
    group: &'static str,
    short_group: &'static str,
    cross_file: bool,
}

impl RuleEntry {
    fn from_rule(rule: Rule) -> Self {
        Self {
            code: rule.code(),
            name: rule.name(),
            summary: rule.summary(),
            fix: rule.fix_availability(),
            explanation: rule.explanation(),
            examples: parse(rule.explanation()).into(),
            family: rule.family(),
            group: rule.group(),
            short_group: rule.short_group(),
            cross_file: rule.reads_other_files(),
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
    name: &'static str,
    codes: Vec<&'static str>,
}

impl HelpOutput for Vec<LinterInfo> {
    const JSON_NEWLINE: bool = true;

    fn write_text(&self, writer: &mut dyn Write) -> io::Result<()> {
        for LinterInfo { name, codes } in self {
            writeln!(writer, "{name}: {}", codes.join(", "))?;
        }
        Ok(())
    }
}

// Ruff's `clean`, for simplint's directory in each Ruff cache directory, which leaves Ruff's own cache alone.
pub(crate) fn clean(level: LogLevel) -> Result<()> {
    let mut stderr = BufWriter::new(std::io::stderr().lock());
    for entry in WalkDir::new(std::env::current_dir()?)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
    {
        let ruff_cache = entry.path().join(CACHE_DIR_NAME);
        let cache = cache::directory(&ruff_cache);
        if is_real_dir(&ruff_cache) && is_real_dir(&cache) {
            if level >= LogLevel::Default {
                writeln!(stderr, "Removing cache at: {}", relativize_path(&cache).bold())?;
            }
            std::fs::remove_dir_all(&cache)?;
        }
    }
    stderr.flush()?;
    Ok(())
}

// A link is not a directory to remove: it may point outside the project, and removing through it would delete there.
fn is_real_dir(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|metadata| metadata.is_dir())
}
