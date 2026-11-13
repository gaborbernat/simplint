// A port of Ruff's `printer.rs`, which the `ruff` crate keeps private, so the output reads as `ruff check`'s does.
use std::borrow::Cow;
use std::cmp::Reverse;
use std::io::Write;
use std::iter;
use std::path::Path;

use anyhow::{Result, bail};
use bitflags::bitflags;
use colored::Colorize;
use itertools::{Itertools, iterate};
use rayon::prelude::*;
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
use ruff_source_file::LineColumn;
use rustc_hash::FxHashMap;
use serde::Serialize;
use supports_hyperlinks::supports_hyperlinks;

use crate::cli::write_flushed;
use crate::lint::registry::Rule;
use crate::run::{Diagnostics, DisplayKey, FixMap, FullDisplay, Mode};

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub(crate) struct Flags: u8 {
        const SHOW_VIOLATIONS = 1 << 0;
        const SHOW_FIX_SUMMARY = 1 << 1;
    }
}

pub(super) struct Printer<'a> {
    pub(crate) format: OutputFormat,
    pub(crate) log_level: LogLevel,
    pub(crate) mode: &'a Mode,
    pub(crate) flags: Flags,
    pub(crate) preview: bool,
    // Ruff's `output-prefer-rule-codes`: in preview, rules show by name unless it is set.
    pub(crate) prefer_rule_codes: bool,
}

impl Printer<'_> {
    pub(super) fn write_once(&self, diagnostics: &Diagnostics, writer: &mut dyn Write) -> Result<()> {
        if matches!(self.log_level, LogLevel::Silent) {
            return Ok(());
        }
        let mut text = String::new();
        if self.format.is_human_readable() {
            if self.flags.intersects(Flags::SHOW_FIX_SUMMARY) && !diagnostics.fixed.is_empty() {
                text.push('\n');
                text.push_str(&fix_summary(&diagnostics.fixed, self.preview && !self.prefer_rule_codes));
                text.push('\n');
            }
            text.push_str(&self.summary_text(diagnostics));
        }
        let notebooks = FxHashMap::default();
        let context = RenderContext::new(&notebooks);
        let config = self.render_config(diagnostics);
        let violations =
            self.flags.intersects(Flags::SHOW_VIOLATIONS).then(|| self.pieces(&config, &context, diagnostics));
        write_flushed(writer, violations.into_iter().flatten().chain([Cow::Owned(text.into_bytes())]))?;
        Ok(())
    }

    // The text of a file's diagnostics in `full` output, for the cache to keep, when the output is `full`.
    pub(super) fn full_display(&self, cwd: &Path) -> Option<FullDisplay> {
        if self.format != OutputFormat::Full {
            return None;
        }
        let color = colored::control::SHOULD_COLORIZE.should_colorize();
        // Ruff picks hyperlinks from the terminal only when it colors the output.
        let key = DisplayKey {
            preview: self.preview,
            prefer_rule_codes: self.prefer_rule_codes,
            color,
            hyperlinks: color && supports_hyperlinks(),
            required: self.mode.unsafe_fixes.required_applicability(),
            cwd,
        };
        Some(FullDisplay::new(self.display_config(false), key))
    }

    fn render_config(&self, diagnostics: &Diagnostics) -> DisplayDiagnosticConfig {
        let statistics = FixableStatistics::try_from(diagnostics, self.mode.unsafe_fixes);
        self.display_config(show_fix_status(self.mode.fix, statistics.as_ref()))
    }

    // The report's bytes in the order they print. Every format renders into memory, which cannot fail; the chunked
    // formats render a window at a time as their pieces are written.
    fn pieces<'a>(
        &self,
        config: &'a DisplayDiagnosticConfig,
        context: &'a RenderContext<'a>,
        diagnostics: &'a Diagnostics,
    ) -> Box<dyn Iterator<Item = Cow<'a, [u8]>> + 'a> {
        let whole = |format| {
            let mut rendered = Vec::new();
            render_diagnostics(&mut rendered, format, config, context, &diagnostics.inner)
                .expect("rendering into memory cannot fail");
            rendered
        };
        let chunks = || render_chunks(self.format, config, context, &diagnostics.inner);
        match self.format {
            // Files whose `full` text the cache held are written as they stand, the rest render as they did.
            OutputFormat::Full if !diagnostics.rendered.is_empty() => {
                full_pieces(config, context, &diagnostics.inner, &diagnostics.rendered)
            }
            // Ruff's SARIF output describes Ruff's rules; simplint's own rules take their place.
            OutputFormat::Sarif => {
                let mut sarif = Vec::new();
                SarifRenderer::new(context, config, rule_metadata(), env!("CARGO_PKG_VERSION"))
                    .write(&mut sarif, &diagnostics.inner)
                    .expect("rendering into memory cannot fail");
                Box::new(iter::once(Cow::Owned(as_simplint(self.format, &sarif, &diagnostics.inner))))
            }
            OutputFormat::Rdjson => {
                Box::new(iter::once(Cow::Owned(as_simplint(self.format, &whole(self.format), &diagnostics.inner))))
            }
            // Renderers that take each diagnostic on its own write their chunks in order. The JSON array is the one
            // format whose chunks need brackets and commas around them.
            OutputFormat::Full
            | OutputFormat::Concise
            | OutputFormat::JsonLines
            | OutputFormat::Github
            | OutputFormat::Pylint
            | OutputFormat::Azure => Box::new(chunks().map(Cow::Owned)),
            // Ruff's JSON has no related locations, so each element that has some gets them as `related`.
            OutputFormat::Json if diagnostics.inner.iter().any(has_related) => {
                let mut elements: Vec<serde_json::Value> = chunks()
                    .flat_map(|chunk| {
                        serde_json::from_slice::<Vec<serde_json::Value>>(&chunk).expect("Ruff renders JSON")
                    })
                    .collect();
                let related =
                    elements.iter_mut().zip(&diagnostics.inner).filter(|(_, diagnostic)| has_related(diagnostic));
                for (element, diagnostic) in related {
                    element["related"] = related_entries(diagnostic)
                        .into_iter()
                        .map(|Related { name, start, end, message }| {
                            serde_json::json!({
                                "filename": name,
                                "location": {"row": start.line.get(), "column": start.column.get()},
                                "end_location": {"row": end.line.get(), "column": end.column.get()},
                                "message": message,
                            })
                        })
                        .collect::<Vec<_>>()
                        .into();
                }
                Box::new(iter::once(Cow::Owned(serde_json::to_vec_pretty(&elements).expect("JSON values serialize"))))
            }
            OutputFormat::Json if !diagnostics.inner.is_empty() => {
                let elements = chunks().enumerate().map(|(at, chunk)| {
                    let elements = chunk
                        .strip_prefix(b"[\n")
                        .and_then(|rest| rest.strip_suffix(b"\n]"))
                        .expect("ruff renders each chunk of a JSON report as an array");
                    Cow::Owned([if at > 0 { &b",\n"[..] } else { b"[\n" }, elements].concat())
                });
                Box::new(elements.chain([Cow::Borrowed(&b"\n]"[..])]))
            }
            _ => Box::new(iter::once(Cow::Owned(whole(self.format)))),
        }
    }

    fn display_config(&self, show_fix_status: bool) -> DisplayDiagnosticConfig {
        DisplayDiagnosticConfig::new("simplint")
            .preview(self.preview)
            .prefer_rule_codes(self.prefer_rule_codes)
            .hide_severity(true)
            .color(colored::control::SHOULD_COLORIZE.should_colorize())
            .with_show_fix_status(show_fix_status)
            .with_fix_applicability(self.mode.unsafe_fixes.required_applicability())
    }

    // Ruff's watch mode output: a timestamped count on standard output, then the diagnostics without a summary.
    // Silent mode prints nothing at all.
    pub(super) fn write_continuously(&self, writer: &mut dyn Write, diagnostics: &Diagnostics) -> Result<()> {
        if self.log_level >= LogLevel::Default {
            let count = diagnostics.inner.len();
            notify_user!("Found {count} error{}. Watching for file changes.", plural(count, "s"));
        }
        let notebooks = FxHashMap::default();
        let context = RenderContext::new(&notebooks);
        let config = self.render_config(diagnostics);
        let shown = !diagnostics.inner.is_empty() && self.log_level > LogLevel::Silent;
        let separator = (shown && self.log_level >= LogLevel::Default).then_some(Cow::Borrowed(&b"\n"[..]));
        let violations = shown.then(|| self.pieces(&config, &context, diagnostics));
        write_flushed(writer, separator.into_iter().chain(violations.into_iter().flatten()))?;
        Ok(())
    }

    pub(super) fn write_to_user(&self, message: &str) {
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
            let fix_prefix = format!("[{}]", "*".cyan());
            let line = match (self.mode.unsafe_fixes.is_hint(), fixables.applicable, fixables.inapplicable_unsafe) {
                (true, 0, unsafe_fixes) => {
                    format!("{unsafe_fixes} unsafe fix{unsafe_es} available with `--fix --unsafe-fixes`.")
                }
                (true, applicable, 0) | (false, applicable, _) if applicable > 0 => {
                    format!("{fix_prefix} {applicable} fixable with `--fix`.")
                }
                (true, applicable, unsafe_fixes) => format!(
                    "{fix_prefix} {applicable} fixable with `--fix` ({unsafe_fixes} unsafe fix{unsafe_es} available \
                     with `--fix --unsafe-fixes`)."
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
        let line = match (fixed, unapplied) {
            (0, 0) => return text,
            (0, _) => {
                format!("{nothing} ({unapplied} fix{} available with `--unsafe-fixes`).", plural(unapplied, "es"))
            }
            (_, 0) => format!("{fix} {errors}."),
            _ => format!(
                "{fix} {errors} ({unapplied} additional fix{} available with `--unsafe-fixes`).",
                plural(unapplied, "es")
            ),
        };
        push_line(&mut text, &line);
        text
    }

    pub(super) fn write_statistics(&self, diagnostics: &Diagnostics, writer: &mut dyn Write) -> Result<()> {
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
            OutputFormat::Json => {
                format!("{}\n", serde_json::to_string_pretty(&statistics).expect("statistics serialize to JSON"))
            }
            _ => bail!("Unsupported serialization format for statistics: {:?}", self.format),
        };
        write_flushed(writer, [text])?;
        Ok(())
    }
}

struct FixableStatistics {
    applicable: usize,
    inapplicable_unsafe: usize,
}

impl FixableStatistics {
    fn try_from(diagnostics: &Diagnostics, unsafe_fixes: UnsafeFixes) -> Option<Self> {
        let required = unsafe_fixes.required_applicability();
        let fixes = diagnostics.inner.iter().filter_map(Diagnostic::fix);
        let applicable = fixes.clone().filter(|fix| fix.applies(required)).count();
        let inapplicable_unsafe = fixes.filter(|fix| !fix.applies(required) && fix.applicability().is_unsafe()).count();
        (applicable > 0 || inapplicable_unsafe > 0).then_some(Self { applicable, inapplicable_unsafe })
    }
}

fn fix_summary(fixed: &FixMap, by_name: bool) -> String {
    let total = fixed.values().map(|table| table.counts().sum::<usize>()).sum::<usize>();
    let width = num_digits(fixed.values().filter_map(|table| table.counts().max()).max().unwrap_or(1));
    let mut text = String::new();
    push_line(&mut text, &format!("Fixed {total} error{}:", plural(total, "s")).bold().green().to_string());
    for (filename, table) in fixed.iter().sorted_by_key(|(filename, ..)| filename.as_str()) {
        push_line(&mut text, &format!("{} {}{}", "-".cyan(), relativize_path(filename).bold(), ":".cyan()));
        for (name, code, count) in table.iter().sorted_by_key(|(.., count)| Reverse(*count)) {
            let code = code.expect("a fixed diagnostic comes from a rule").as_str();
            let (shown, detail) = if by_name { (name.as_str(), code) } else { (code, name.as_str()) };
            push_line(&mut text, &format!("    {count:>width$} × {} ({detail})", shown.red().bold()));
        }
    }
    text
}

fn num_digits(n: usize) -> usize {
    iterate(n, |&n| n / 10).take_while(|&n| n > 0).count().max(1)
}

fn push_line(text: &mut String, line: &str) {
    text.push_str(line);
    text.push('\n');
}

fn plural(count: usize, suffix: &str) -> &str {
    if count == 1 { "" } else { suffix }
}

// With fixes applied, a violation left over was not fixable after all, so no fix marker is shown.
fn show_fix_status(fix_mode: flags::FixMode, fixables: Option<&FixableStatistics>) -> bool {
    !fix_mode.is_apply() && fixables.is_some_and(|fixables| fixables.applicable > 0)
}

// The diagnostics in order. A file whose text is known is written as it is, the others render in parallel chunks, as
// they do when no text is known.
fn full_pieces<'a>(
    config: &'a DisplayDiagnosticConfig,
    context: &'a RenderContext<'a>,
    diagnostics: &'a [Diagnostic],
    rendered: &'a FxHashMap<String, Vec<u8>>,
) -> Box<dyn Iterator<Item = Cow<'a, [u8]>> + 'a> {
    // Each segment renders the files before a known text, then writes that text; the last has no text after it.
    let mut segments: Vec<(&[Diagnostic], &[u8])> = Vec::new();
    let mut pending = 0;
    let mut start = 0;
    for file in diagnostics.chunk_by(|left, right| file_name(left) == file_name(right)) {
        let end = start + file.len();
        if let Some(text) = rendered.get(file_name(&file[0])) {
            segments.push((&diagnostics[pending..start], text));
            pending = end;
        }
        start = end;
    }
    segments.push((&diagnostics[pending..], &[]));
    Box::new(segments.into_iter().flat_map(|(diagnostics, text)| {
        render_chunks(OutputFormat::Full, config, context, diagnostics).map(Cow::Owned).chain([Cow::Borrowed(text)])
    }))
}

// Ruff's SARIF and rdjson renderers name Ruff, or link to it, as the tool behind the report. SARIF also gets the
// related locations of the results, which Ruff's renderer leaves out.
fn as_simplint(format: OutputFormat, rendered: &[u8], diagnostics: &[Diagnostic]) -> Vec<u8> {
    let mut report: serde_json::Value = serde_json::from_slice(rendered).expect("Ruff renders JSON");
    if format == OutputFormat::Sarif {
        if diagnostics.iter().any(has_related) {
            let results = report["runs"][0]["results"].as_array_mut().expect("Ruff's SARIF report lists its results");
            let related = results.iter_mut().zip(diagnostics).filter(|(_, diagnostic)| has_related(diagnostic));
            for (result, diagnostic) in related {
                result["relatedLocations"] = related_entries(diagnostic)
                    .into_iter()
                    .map(|Related { name, start, end, message }| {
                        serde_json::json!({
                            "physicalLocation": {
                                "artifactLocation": {"uri": file_uri(&name)},
                                "region": {
                                    "startLine": start.line.get(),
                                    "startColumn": start.column.get(),
                                    "endLine": end.line.get(),
                                    "endColumn": end.column.get(),
                                },
                            },
                            "message": {"text": message},
                        })
                    })
                    .collect::<Vec<_>>()
                    .into();
            }
        }
        report["runs"][0]["tool"]["driver"]["informationUri"] = env!("CARGO_PKG_REPOSITORY").into();
    } else {
        report["source"] = serde_json::json!({"name": "simplint", "url": "https://simplint.readthedocs.io/"});
    }
    format!("{report:#}").into_bytes()
}

fn file_name(diagnostic: &Diagnostic) -> &str {
    diagnostic
        .primary_annotation()
        .and_then(|annotation| annotation.get_span().as_ruff_file())
        .expect("simplint reports a diagnostic on a Ruff source file")
        .name()
}

// Diagnostics render in parallel chunks, which come back in order, a window of them at a time: a window bounds the
// rendered output held in memory, and each window is written before the next one renders.
fn render_chunks<'a>(
    format: OutputFormat,
    config: &'a DisplayDiagnosticConfig,
    context: &'a RenderContext<'a>,
    diagnostics: &'a [Diagnostic],
) -> impl Iterator<Item = Vec<u8>> + 'a {
    const FULL_CHUNK: usize = 64;
    diagnostics.chunks(FULL_CHUNK * 256).flat_map(move |window| {
        window
            .par_chunks(FULL_CHUNK)
            .map(|chunk| {
                let mut buffer = Vec::new();
                render_diagnostics(&mut buffer, format, config, context, chunk)
                    .expect("rendering into memory cannot fail");
                buffer
            })
            .collect::<Vec<_>>()
            .into_iter()
    })
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

fn has_related(diagnostic: &Diagnostic) -> bool {
    diagnostic.secondary_annotations().next().is_some()
}

// The locations a diagnostic's secondary annotations name, each as its file's name, its start and end, and its message.
// simplint annotates only a range of a Ruff source file.
fn related_entries(diagnostic: &Diagnostic) -> Vec<Related> {
    diagnostic
        .secondary_annotations()
        .map(|annotation| {
            let span = annotation.get_span();
            let file = span.as_ruff_file().expect("simplint annotates Ruff source files");
            let range = span.range().expect("simplint annotates a range");
            let code = file.to_source_code();
            Related {
                name: file.name().to_owned(),
                start: code.line_column(range.start()),
                end: code.line_column(range.end()),
                message: annotation.get_message().unwrap_or_default().to_owned(),
            }
        })
        .collect()
}

struct Related {
    name: String,
    start: LineColumn,
    end: LineColumn,
    message: String,
}

// A file's URI as Ruff writes a result's primary location, so the locations of one result agree. Related locations
// name files of the project index, which holds absolute paths, so Ruff's fallback for a relative one never applies.
fn file_uri(name: &str) -> String {
    url::Url::from_file_path(name).expect("the project index holds absolute paths").to_string()
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
