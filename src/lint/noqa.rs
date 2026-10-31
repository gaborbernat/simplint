use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use ruff_db::diagnostic::Diagnostic;
use ruff_linter::directives::{Flags, extract_directives};
use ruff_linter::settings::types::PreviewMode;
use ruff_linter::suppression::Suppressions as RuffSuppressions;
use ruff_linter::{Locator, SuppressionKind, generate_suppression_edits};
use ruff_python_ast::ModModule;
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::Parsed;
use ruff_python_trivia::{CommentRanges, indentation_at_offset};
use ruff_source_file::{LineEnding, LineRanges};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::registry::Rule;

// The `noqa` and `simplint: ignore` comments of one file, by the line they sit on, and the file-level `ruff: noqa`
// and `ruff: file-ignore` comments that cover the whole file.
pub struct Suppressions<'a> {
    source: &'a str,
    path: &'a Path,
    parsed: &'a Parsed<ModModule>,
    indexer: &'a Indexer,
    stylist: &'a Stylist<'a>,
    lines: HashMap<TextSize, Line>,
    file: Vec<Directive>,
}

#[derive(Default)]
struct Line {
    noqa: Option<(TextSize, Directive)>,
    ignore: Option<(TextSize, Vec<String>)>,
}

impl<'a> Suppressions<'a> {
    pub fn new(
        path: &'a Path,
        source: &'a str,
        parsed: &'a Parsed<ModModule>,
        indexer: &'a Indexer,
        stylist: &'a Stylist<'a>,
    ) -> Self {
        let mut lines: HashMap<TextSize, Line> = HashMap::new();
        let mut file = Vec::new();
        for range in indexer.comment_ranges() {
            let comment = &source[range];
            file.extend(Directive::parse_file_level(comment, indentation_at_offset(range.start(), source)));
            let noqa = Directive::parse(comment).map(|(offset, directive)| (range.start() + offset, directive));
            let ignore = parse_ignore(comment).map(|(offset, codes)| (range.start() + offset, codes));
            if noqa.is_none() && ignore.is_none() {
                continue;
            }
            let line = lines.entry(source.line_start(range.start())).or_default();
            line.noqa = line.noqa.take().or(noqa);
            line.ignore = line.ignore.take().or(ignore);
        }
        Self { source, path, parsed, indexer, stylist, lines, file }
    }

    pub fn unsuppressed(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        self.pending(diagnostics).into_iter().map(|(diagnostic, _)| diagnostic).collect()
    }

    // Each diagnostic no comment suppresses, with the line its suppression comment belongs on.
    pub fn pending(&self, mut diagnostics: Vec<Diagnostic>) -> Vec<(Diagnostic, TextSize)> {
        let lines = self.locate(&mut diagnostics);
        diagnostics
            .into_iter()
            .zip(lines)
            .filter(|(diagnostic, line)| {
                let code = diagnostic.secondary_code().expect("a rule diagnostic has a code").as_str();
                !self.file.iter().any(|directive| directive.suppresses(code))
                    && self.lines.get(line).is_none_or(|directives| !directives.suppresses(code))
            })
            .collect()
    }

    // Ruff expects a `noqa` comment on the diagnostic's line, or on the last line of the multi-line string or
    // continuation it starts in. Ruff keeps that mapping private, but its suppression edits land on that line, so with
    // no comments to merge into, each edit shows where the comment goes. Each diagnostic records its line, which
    // Ruff's JSON output shows as `noqa_row`.
    pub fn locate(&self, diagnostics: &mut [Diagnostic]) -> Vec<TextSize> {
        // Most files report nothing, and those skip Ruff's pass over the tokens.
        if diagnostics.is_empty() {
            return Vec::new();
        }
        let locator = Locator::new(self.source);
        let directives = extract_directives(self.parsed.tokens(), Flags::NOQA, &locator, self.indexer);
        generate_suppression_edits(
            self.path,
            diagnostics,
            &locator,
            &CommentRanges::default(),
            &[],
            &directives.noqa_line_for,
            LineEnding::Lf,
            &RuffSuppressions::default(),
            SuppressionKind::Noqa,
            PreviewMode::Disabled,
        )
        .into_iter()
        .zip(diagnostics)
        .map(|(edit, diagnostic)| {
            let line = self.source.line_start(edit.expect("a rule diagnostic has a code").start());
            diagnostic.set_noqa_offset(line);
            line
        })
        .collect()
    }

    // Ruff's `add_suppression_inner`: one comment per line naming every code reported there, merged into the line's
    // comment of the same kind. A line with a blanket `# noqa` is left alone. Returns the number of lines changed and
    // the new source.
    pub fn add(
        &self,
        pending: &[(Diagnostic, TextSize)],
        kind: SuppressionKind,
        reason: Option<&str>,
    ) -> (usize, String) {
        let mut codes: BTreeMap<TextSize, BTreeSet<String>> = BTreeMap::new();
        for (diagnostic, line) in pending {
            if self
                .lines
                .get(line)
                .is_some_and(|directives| directives.noqa.as_ref().is_some_and(|(_, noqa)| noqa.is_blanket()))
            {
                continue;
            }
            let code = diagnostic.secondary_code().expect("a rule diagnostic has a code");
            codes.entry(*line).or_default().insert(code.to_string());
        }
        let line_ending = self.stylist.line_ending();
        let mut output = String::with_capacity(self.source.len());
        let mut last = TextSize::default();
        let count = codes.len();
        for (line, mut line_codes) in codes {
            let line_range = self.source.full_line_range(line);
            let existing = self.lines.get(&line).and_then(|directives| match kind {
                SuppressionKind::Noqa => {
                    directives.noqa.as_ref().map(|(start, directive)| (*start, directive.codes.clone()))
                }
                SuppressionKind::Ignore => directives.ignore.clone(),
            });
            let (edit_range, blank_line) = if let Some((start, existing_codes)) = existing {
                line_codes.extend(existing_codes);
                let prefix = &self.source[TextRange::new(line_range.start(), start)];
                let blank = prefix.trim().is_empty();
                let edit_start = if blank { start } else { line_range.start() + TextSize::of(prefix.trim_end()) };
                (TextRange::new(edit_start, line_range.end()), blank)
            } else {
                let trimmed = self.source[line_range].trim_end();
                let start = line_range.start() + TextSize::of(trimmed);
                (TextRange::new(start, line_range.end()), trimmed.trim_start().is_empty())
            };
            output.push_str(&self.source[TextRange::new(last, edit_range.start())]);
            if !blank_line {
                output.push_str("  ");
            }
            let joined = line_codes.into_iter().collect::<Vec<_>>().join(", ");
            let comment = match kind {
                SuppressionKind::Noqa => format!("# noqa: {joined}"),
                SuppressionKind::Ignore => format!("# simplint: ignore[{joined}]"),
            };
            let reason = reason.map(|reason| format!(" {reason}")).unwrap_or_default();
            output.push_str(&[comment.as_str(), &reason, line_ending.as_str()].concat());
            last = edit_range.end();
        }
        output.push_str(&self.source[usize::from(last)..]);
        (count, output)
    }
}

impl Line {
    fn suppresses(&self, code: &str) -> bool {
        self.noqa.as_ref().is_some_and(|(_, directive)| directive.suppresses(code))
            || self.ignore.as_ref().is_some_and(|(_, codes)| codes.iter().any(|candidate| candidate == code))
    }
}

// A `noqa` directive; one without codes suppresses every rule.
struct Directive {
    codes: Vec<String>,
}

impl Directive {
    // The directive and the offset of its `#` in the comment.
    fn parse(comment: &str) -> Option<(TextSize, Self)> {
        let lower = comment.to_ascii_lowercase();
        let (hash, start) = lower.match_indices('#').find_map(|(index, _)| {
            let rest = lower[index + 1..].trim_start();
            rest.starts_with("noqa").then(|| (index, comment.len() - rest.len() + "noqa".len()))
        })?;
        let codes = comment[start..].trim_start().strip_prefix(':').unwrap_or_default();
        let codes = code_list(codes).take_while(|part| is_code(part)).map(str::to_owned).collect();
        Some((TextSize::try_from(hash).expect("a comment fits a text size"), Self { codes }))
    }

    // Ruff's file-level exemptions: `# ruff: noqa` (or `# flake8: noqa`) on its own line, blanket or with codes, and
    // `# ruff: file-ignore[...]` at module scope. Ruff skips a `noqa` whose codes do not parse rather than reading it
    // as blanket, so the codes follow Ruff's `lex_directive`.
    fn parse_file_level(comment: &str, indentation: Option<&str>) -> Option<Self> {
        if let Some((_, codes)) = parse_bracket(comment, "ruff:", "file-ignore[") {
            return (indentation == Some("") && !codes.is_empty()).then_some(Self { codes });
        }
        indentation?;
        let rest = comment.match_indices('#').find_map(|(index, _)| {
            let rest = comment[index + 1..].trim_start();
            let rest = rest.strip_prefix("ruff").or_else(|| rest.strip_prefix("flake8"))?;
            let rest = rest.trim_start().strip_prefix(':')?.trim_start();
            rest.get(..4).filter(|noqa| noqa.eq_ignore_ascii_case("noqa")).map(|_| &rest[4..])
        })?;
        let codes = match rest.trim_start().strip_prefix(':') {
            Some(codes) => codes,
            None if rest.is_empty() || rest.starts_with(|char: char| char == '#' || char.is_whitespace()) => {
                return Some(Self { codes: Vec::new() });
            }
            None => return None,
        };
        let codes: Vec<String> = code_list(codes).take_while(|part| is_code(part)).map(str::to_owned).collect();
        (!codes.is_empty()).then_some(Self { codes })
    }

    fn is_blanket(&self) -> bool {
        self.codes.is_empty()
    }

    // A blanket `# noqa` is itself what SMP060 reports, so it cannot silence that report.
    fn suppresses(&self, code: &str) -> bool {
        if self.is_blanket() {
            code != Rule::SuppressionWithoutReason.code()
        } else {
            self.codes.iter().any(|candidate| candidate == code)
        }
    }
}

fn code_list(codes: &str) -> impl Iterator<Item = &str> {
    codes.split(|char: char| char == ',' || char.is_whitespace()).filter(|part| !part.is_empty())
}

// A `# simplint: ignore[SMP001, SMP002]` comment, simplint's counterpart of Ruff's `# ruff: ignore[...]`: the codes
// and the offset of its `#` in the comment.
fn parse_ignore(comment: &str) -> Option<(TextSize, Vec<String>)> {
    parse_bracket(comment, "simplint:", "ignore[")
}

// A `# tool: action[CODES]` comment: the codes and the offset of its `#` in the comment.
fn parse_bracket(comment: &str, tool: &str, action: &str) -> Option<(TextSize, Vec<String>)> {
    comment.match_indices('#').find_map(|(index, _)| {
        let rest = comment[index + 1..].trim_start().strip_prefix(tool)?.trim_start();
        let (codes, _) = rest.strip_prefix(action)?.split_once(']')?;
        let codes = code_list(codes).map(str::to_owned).collect();
        Some((TextSize::try_from(index).expect("a comment fits a text size"), codes))
    })
}

pub fn is_code(part: &str) -> bool {
    let letters = part.chars().take_while(char::is_ascii_uppercase).count();
    letters > 0 && part.len() > letters && part[letters..].chars().all(|char| char.is_ascii_digit())
}
