// A port of Ruff's `printer.rs`, which the `ruff` crate keeps private, so the summary lines, statistics and fix summary
// read the same as `ruff check`'s.
use std::cmp::Reverse;
use std::io::Write;

use anyhow::{Result, bail};
use bitflags::bitflags;
use colored::Colorize;
use itertools::{Itertools, iterate};
use ruff_db::diagnostic::render::{Renderer, SarifRenderer};
use ruff_db::diagnostic::{
    Diagnostic, DiagnosticStylesheet, DisplayDiagnosticConfig, LintName, RuleMetadata, SecondaryCode, Severity,
    fmt_with_hyperlink,
};
use ruff_linter::fs::relativize_path;
use ruff_linter::linter::FixTable;
use ruff_linter::logging::LogLevel;
use ruff_linter::message::{RenderContext, render_diagnostics};
use ruff_linter::notify_user;
use ruff_linter::settings::flags;
use ruff_linter::settings::types::{OutputFormat, UnsafeFixes};
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::cli::diagnostics::{Diagnostics, FixMap, Mode};
use crate::lint::registry::Rule;

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct Flags: u8 {
        const SHOW_VIOLATIONS = 1 << 0;
        const SHOW_FIX_SUMMARY = 1 << 1;
    }
}

#[derive(Serialize)]
struct ExpandedStatistics<'a> {
    code: Option<&'a SecondaryCode>,
    #[serde(skip)]
    url: Option<&'a str>,
    name: &'static str,
    count: usize,
    #[serde(rename = "fixable")]
    all_fixable: bool,
    fixable_count: usize,
}

impl ExpandedStatistics<'_> {
    fn any_fixable(&self) -> bool {
        self.fixable_count > 0
    }
}

pub struct Printer<'a> {
    pub format: OutputFormat,
    pub log_level: LogLevel,
    pub mode: &'a Mode,
    pub flags: Flags,
    pub preview: bool,
    // Ruff's `output-prefer-rule-codes`: in preview, rules show by name unless it is set.
    pub prefer_rule_codes: bool,
}

impl Printer<'_> {
    pub fn write_once(&self, diagnostics: &Diagnostics, writer: &mut dyn Write) -> Result<()> {
        if matches!(self.log_level, LogLevel::Silent) {
            return Ok(());
        }
        if self.flags.intersects(Flags::SHOW_VIOLATIONS) {
            let fixables = FixableStatistics::try_from(diagnostics, self.mode.unsafe_fixes);
            self.render(diagnostics, fixables.as_ref(), writer)?;
        }
        if self.format.is_human_readable() {
            let mut text = String::new();
            if self.flags.intersects(Flags::SHOW_FIX_SUMMARY) && !diagnostics.fixed.is_empty() {
                text.push('\n');
                text.push_str(&fix_summary(&diagnostics.fixed, self.preview && !self.prefer_rule_codes));
                text.push('\n');
            }
            text.push_str(&self.summary_text(diagnostics));
            writer.write_all(text.as_bytes())?;
        }
        writer.flush()?;
        Ok(())
    }

    fn render(
        &self,
        diagnostics: &Diagnostics,
        fixables: Option<&FixableStatistics>,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let config = DisplayDiagnosticConfig::new("simplint")
            .preview(self.preview)
            .prefer_rule_codes(self.prefer_rule_codes)
            .hide_severity(true)
            .color(colored::control::SHOULD_COLORIZE.should_colorize())
            .with_show_fix_status(show_fix_status(self.mode.fix, fixables))
            .with_fix_applicability(self.mode.unsafe_fixes.required_applicability());
        let notebooks = FxHashMap::default();
        let context = RenderContext::new(&notebooks);
        let mut rendered = Vec::new();
        match self.format {
            // Ruff's SARIF output describes Ruff's rules; simplint's own rules take their place.
            OutputFormat::Sarif => SarifRenderer::new(&context, &config, rule_metadata(), env!("CARGO_PKG_VERSION"))
                .write(&mut rendered, &diagnostics.inner)?,
            OutputFormat::Rdjson => {
                render_diagnostics(&mut rendered, self.format, &config, &context, &diagnostics.inner)?;
            }
            _ => {
                render_diagnostics(writer, self.format, &config, &context, &diagnostics.inner)?;
                return Ok(());
            }
        }
        Self::write_as_simplint(self.format, &rendered, writer)
    }

    // Ruff's SARIF and rdjson renderers name Ruff, or link to it, as the tool behind the report.
    fn write_as_simplint(format: OutputFormat, rendered: &[u8], writer: &mut dyn Write) -> Result<()> {
        let mut report: serde_json::Value = serde_json::from_slice(rendered)?;
        if format == OutputFormat::Sarif {
            report["runs"][0]["tool"]["driver"]["informationUri"] = env!("CARGO_PKG_REPOSITORY").into();
        } else {
            report["source"] = serde_json::json!({"name": "simplint", "url": "https://simplint.readthedocs.io/"});
        }
        write!(writer, "{report:#}")?;
        Ok(())
    }

    // Ruff's watch mode output: a timestamped count on standard output, then the diagnostics without a summary.
    // Silent mode prints nothing at all.
    pub fn write_continuously(&self, writer: &mut dyn Write, diagnostics: &Diagnostics) -> Result<()> {
        if self.log_level >= LogLevel::Default {
            let count = diagnostics.inner.len();
            notify_user!("Found {count} error{}. Watching for file changes.", plural(count, "s"));
        }
        if !diagnostics.inner.is_empty() && self.log_level > LogLevel::Silent {
            if self.log_level >= LogLevel::Default {
                writeln!(writer)?;
            }
            let fixables = FixableStatistics::try_from(diagnostics, self.mode.unsafe_fixes);
            self.render(diagnostics, fixables.as_ref(), writer)?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn write_to_user(&self, message: &str) {
        if self.log_level >= LogLevel::Default {
            notify_user!("{message}");
        }
    }

    fn summary_text(&self, diagnostics: &Diagnostics) -> String {
        let mut text = String::new();
        if self.log_level < LogLevel::Default {
            return text;
        }
        let fixables = FixableStatistics::try_from(diagnostics, self.mode.unsafe_fixes);
        let fixed = diagnostics.fixed.values().flat_map(FixTable::counts).sum::<usize>();
        let fix_prefix = format!("[{}]", "*".cyan());
        if self.flags.intersects(Flags::SHOW_VIOLATIONS) {
            let remaining = diagnostics.inner.len();
            let total = fixed + remaining;
            let line = if fixed > 0 {
                format!("Found {total} error{} ({fixed} fixed, {remaining} remaining).", plural(total, "s"))
            } else if remaining > 0 {
                format!("Found {remaining} error{}.", plural(remaining, "s"))
            } else {
                "All checks passed!".green().bold().to_string()
            };
            push_line(&mut text, &line);
            let Some(fixables) = fixables else {
                return text;
            };
            let unsafe_es = plural(fixables.inapplicable_unsafe, "es");
            let line = match (self.mode.unsafe_fixes.is_hint(), fixables.applicable, fixables.inapplicable_unsafe) {
                (true, 0, unsafe_fixes) => {
                    format!("{unsafe_fixes} unsafe fix{unsafe_es} available with `--fix --unsafe-fixes`.")
                }
                (true, applicable, 0) | (false, applicable, _) if applicable > 0 => {
                    format!("{fix_prefix} {applicable} fixable with `--fix`.")
                }
                (true, applicable, unsafe_fixes) => format!(
                    "{fix_prefix} {applicable} fixable with `--fix` ({unsafe_fixes} unsafe fix{unsafe_es} available with \
                     `--fix --unsafe-fixes`)."
                ),
                (false, ..) => return text,
            };
            push_line(&mut text, &line);
            return text;
        }
        let unapplied = fixables.map_or(0, |fixables| fixables.inapplicable_unsafe);
        let (fix, nothing) = if self.mode.fix.is_apply() {
            ("Fixed", "No errors fixed")
        } else {
            ("Would fix", "No errors would be fixed")
        };
        let errors = format!("{fixed} error{}", plural(fixed, "s"));
        let available = format!("{unapplied} fix{}", plural(unapplied, "es"));
        let line = match (fixed, unapplied) {
            (0, 0) => return text,
            (0, _) => format!("{nothing} ({available} available with `--unsafe-fixes`)."),
            (_, 0) => format!("{fix} {errors}."),
            _ => format!(
                "{fix} {errors} ({unapplied} additional fix{} available with `--unsafe-fixes`).",
                plural(unapplied, "es")
            ),
        };
        push_line(&mut text, &line);
        text
    }

    pub fn write_statistics(&self, diagnostics: &Diagnostics, writer: &mut dyn Write) -> Result<()> {
        let required = self.mode.unsafe_fixes.required_applicability();
        let statistics: Vec<ExpandedStatistics> = diagnostics
            .inner
            .iter()
            .sorted_by_key(|diagnostic| diagnostic.name())
            .chunk_by(|diagnostic| diagnostic.name())
            .into_iter()
            .map(|(name, group)| {
                let group: Vec<&Diagnostic> = group.collect();
                let fixable_count =
                    group.iter().filter(|diagnostic| diagnostic.fix().is_some_and(|fix| fix.applies(required))).count();
                ExpandedStatistics {
                    code: group[0].secondary_code(),
                    url: group[0].documentation_url(),
                    name,
                    count: group.len(),
                    all_fixable: fixable_count == group.len(),
                    fixable_count,
                }
            })
            .sorted_by_key(|statistic| Reverse(statistic.count))
            .collect();
        let text = match self.format {
            _ if statistics.is_empty() => String::new(),
            OutputFormat::Full | OutputFormat::Concise => {
                format!("{}{}", statistics_table(&statistics), self.summary_text(diagnostics))
            }
            OutputFormat::Json => format!("{}\n", serde_json::to_string_pretty(&statistics)?),
            _ => bail!("Unsupported serialization format for statistics: {:?}", self.format),
        };
        writer.write_all(text.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

fn plural(count: usize, suffix: &str) -> &str {
    if count == 1 { "" } else { suffix }
}

fn push_line(text: &mut String, line: &str) {
    text.push_str(line);
    text.push('\n');
}

// Flake8's `--statistics` layout, as Ruff prints it: count, code, a fixability marker and the rule name.
fn statistics_table(statistics: &[ExpandedStatistics]) -> String {
    let count_width = num_digits(statistics.iter().map(|statistic| statistic.count).max().unwrap_or(1));
    let code_width = statistics.iter().map(|statistic| statistic.code.map_or(0, |code| code.len())).max().unwrap_or(0);
    let any_fixable = statistics.iter().any(ExpandedStatistics::any_fixable);
    let stylesheet = if colored::control::SHOULD_COLORIZE.should_colorize() {
        DiagnosticStylesheet::styled()
    } else {
        DiagnosticStylesheet::plain()
    };
    let all_fixable = format!("[{}] ", "*".cyan());
    let partially_fixable = format!("[{}] ", "-".cyan());
    let mut text = String::new();
    for statistic in statistics {
        let code = statistic.code.map(SecondaryCode::as_str).unwrap_or_default();
        let marker = match (any_fixable, statistic.all_fixable, statistic.any_fixable()) {
            (false, ..) => "",
            (true, true, _) => &all_fixable,
            (true, false, true) => &partially_fixable,
            (true, false, false) => "[ ] ",
        };
        let line = format!(
            "{:>count_width$}\t{}{:padding$}\t{marker}{}",
            statistic.count.to_string().bold(),
            fmt_with_hyperlink(code.red().bold(), statistic.url, &stylesheet),
            "",
            statistic.name,
            padding = code_width - code.len(),
        );
        push_line(&mut text, &line);
    }
    text
}

fn rule_metadata() -> FxHashMap<LintName, RuleMetadata> {
    Rule::ALL
        .iter()
        .map(|rule| {
            let name = LintName::of(rule.name());
            let metadata = RuleMetadata {
                name,
                summary: rule.summary(),
                description: rule.explanation().into(),
                help_url: rule.url(),
                linter: Some("simplint"),
                default_severity: Some(Severity::Error),
                enabled_by_default: true,
            };
            (name, metadata)
        })
        .collect()
}

fn num_digits(n: usize) -> usize {
    iterate(n, |&n| n / 10).take_while(|&n| n > 0).count().max(1)
}

// With fixes applied, a violation left over was not fixable after all, so no fix marker is shown.
fn show_fix_status(fix_mode: flags::FixMode, fixables: Option<&FixableStatistics>) -> bool {
    !fix_mode.is_apply() && fixables.is_some_and(|fixables| fixables.applicable > 0)
}

fn fix_summary(fixed: &FixMap, by_name: bool) -> String {
    let total = fixed.values().map(|table| table.counts().sum::<usize>()).sum::<usize>();
    let width = num_digits(fixed.values().filter_map(|table| table.counts().max()).max().unwrap_or(1));
    let mut text = String::new();
    push_line(&mut text, &format!("Fixed {total} error{}:", plural(total, "s")).bold().green().to_string());
    for (filename, table) in fixed.iter().sorted_by_key(|(filename, ..)| filename.as_str()) {
        push_line(&mut text, &format!("{} {}{}", "-".cyan(), relativize_path(filename).bold(), ":".cyan()));
        for (name, code, count) in table.iter().sorted_by_key(|(.., count)| Reverse(*count)) {
            let code = code.map_or_else(|| name.as_str(), SecondaryCode::as_str);
            let (shown, detail) = if by_name { (name.as_str(), code) } else { (code, name.as_str()) };
            push_line(&mut text, &format!("    {count:>width$} × {} ({detail})", shown.red().bold()));
        }
    }
    text
}

struct FixableStatistics {
    applicable: usize,
    inapplicable_unsafe: usize,
}

impl FixableStatistics {
    fn try_from(diagnostics: &Diagnostics, unsafe_fixes: UnsafeFixes) -> Option<Self> {
        let mut applicable = 0;
        let mut inapplicable_unsafe = 0;
        for fix in diagnostics.inner.iter().filter_map(Diagnostic::fix) {
            if fix.applies(unsafe_fixes.required_applicability()) {
                applicable += 1;
            } else if fix.applicability().is_unsafe() {
                inapplicable_unsafe += 1;
            }
        }
        (applicable > 0 || inapplicable_unsafe > 0).then_some(Self { applicable, inapplicable_unsafe })
    }
}
