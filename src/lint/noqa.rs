use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, BTreeSet};
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
    lines: FxHashMap<TextSize, Line>,
    file: Vec<Directive>,
}

pub struct UnusedIgnore {
    pub start: TextSize,
    pub count: usize,
    pub edits: Vec<(TextRange, String)>,
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
        let mut lines: FxHashMap<TextSize, Line> = FxHashMap::default();
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

    pub fn ignore_candidates(&self, diagnostics: &[Diagnostic]) -> Vec<(TextSize, String)> {
        let mut located = diagnostics.to_vec();
        let rows = self.locate(&mut located);
        let mut used = FxHashSet::default();
        for (diagnostic, line) in located.iter().zip(rows) {
            let code = diagnostic.secondary_code().expect("a rule diagnostic has a code").as_str();
            if self.file.iter().any(|directive| directive.suppresses(code)) {
                continue;
            }
            let Some(directives) = self.lines.get(&line) else { continue };
            if directives.noqa.as_ref().is_some_and(|(_, directive)| directive.suppresses(code)) {
                continue;
            }
            let Some((start, codes)) = &directives.ignore else { continue };
            let mut matches = codes.iter().filter(|candidate| matches_ignore(candidate, code));
            if let Some(candidate) = matches.next()
                && matches.next().is_none()
            {
                used.insert((*start, candidate.clone()));
            }
        }
        self.indexer
            .comment_ranges()
            .iter()
            .filter_map(|range| {
                let (offset, codes) = parse_ignore(&self.source[*range])?;
                Some(codes.into_iter().map(move |code| (range.start() + offset, code)))
            })
            .flatten()
            .filter(|candidate| !used.contains(candidate))
            .collect()
    }

    pub fn unused_ignores(
        &self,
        diagnostics: Vec<Diagnostic>,
        relint: impl Fn(&str) -> Vec<Diagnostic>,
    ) -> Option<UnusedIgnore> {
        let candidates = self.ignore_candidates(&diagnostics);
        if candidates.is_empty() {
            return None;
        }
        let mut baseline: Vec<Diagnostic> =
            self.pending(diagnostics).into_iter().map(|(diagnostic, _)| diagnostic).collect();
        let mut groups: BTreeMap<TextSize, Vec<String>> = BTreeMap::new();
        for (start, selector) in candidates {
            groups.entry(start).or_default().push(selector);
        }
        let mut current = self.source.to_owned();
        let mut edits = Vec::new();
        let mut count = 0;
        let mut first = TextSize::default();
        for (start, selectors) in groups.into_iter().rev() {
            let before = current.clone();
            for selector in selectors.into_iter().rev() {
                let (edit, replacement) = remove_ignore_selector(&current, start, &selector)
                    .expect("a parsed selector exists in its comment");
                let mut proposed = current.clone();
                proposed.replace_range(usize::from(edit.start())..usize::from(edit.end()), &replacement);
                let next = relint(&proposed);
                if !same_or_fewer_reports(&baseline, &next, edit, replacement.len()) {
                    continue;
                }
                current = proposed;
                baseline = next;
                count += 1;
                first = start;
                if replacement.is_empty() {
                    break;
                }
            }
            if current != before {
                edits.push(changed_region(&before, &current));
            }
        }
        (count > 0).then_some(UnusedIgnore { start: first, count, edits })
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
            let directives = self.lines.get(line);
            if directives.is_some_and(|directives| directives.noqa.as_ref().is_some_and(|(_, noqa)| noqa.is_blanket()))
            {
                continue;
            }
            let code = diagnostic.secondary_code().expect("a rule diagnostic has a code").as_str();
            let line_codes = codes.entry(*line).or_default();
            let name = match kind {
                SuppressionKind::Noqa => code.to_owned(),
                // Merging into the line's ignore comment renames its numeric codes, which settles SMP008 there.
                SuppressionKind::Ignore
                    if code == Rule::NumericIgnore.code()
                        && directives.and_then(|directives| directives.ignore.as_ref()).is_some_and(
                            |(start, _)| {
                                diagnostic.range().is_some_and(|range| {
                                    TextRange::new(*start, self.source.line_end(*start)).contains(range.start())
                                })
                            },
                        ) =>
                {
                    continue;
                }
                SuppressionKind::Ignore => {
                    Rule::from_code(code).map_or_else(|| code.to_owned(), |rule| rule.name().to_owned())
                }
            };
            line_codes.insert(name);
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
            let (edit_range, blank_line, suffix) = if let Some((start, existing_codes)) = existing {
                line_codes.extend(existing_codes.into_iter().map(|code| {
                    if matches!(kind, SuppressionKind::Ignore) {
                        Rule::from_code(&code).map_or(code.clone(), |rule| rule.name().to_owned())
                    } else {
                        code
                    }
                }));
                let prefix = &self.source[TextRange::new(line_range.start(), start)];
                let blank = prefix.trim().is_empty();
                let edit_start = if blank { start } else { line_range.start() + TextSize::of(prefix.trim_end()) };
                let line_end = self.source.line_end(start);
                let after = &self.source[TextRange::new(self.codes_end(start, kind), line_end)];
                (TextRange::new(edit_start, line_end), blank, with_reason(after, reason))
            } else {
                let trimmed = self.source[line_range].trim_end();
                let start = line_range.start() + TextSize::of(trimmed);
                let reason = reason.map(|reason| format!(" {reason}")).unwrap_or_default();
                (
                    TextRange::new(start, line_range.end()),
                    trimmed.trim_start().is_empty(),
                    [reason.as_str(), line_ending.as_str()].concat(),
                )
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
            output.push_str(&comment);
            output.push_str(&suffix);
            last = edit_range.end();
        }
        output.push_str(&self.source[usize::from(last)..]);
        (count, output)
    }

    // Where the codes of the directive whose `#` is at `start` end; its reason and any later comment follow.
    fn codes_end(&self, start: TextSize, kind: SuppressionKind) -> TextSize {
        let comment = &self.source[TextRange::new(start, self.source.line_end(start))];
        let head = match kind {
            SuppressionKind::Noqa => {
                let after = strip_noqa(comment[1..].trim_start()).expect("the directive starts at its `#`");
                let (_, rest) = noqa_codes(after).expect("the directive's codes parsed");
                comment[..comment.len() - rest.len()].trim_end()
            }
            SuppressionKind::Ignore => {
                let (offset, codes) = ignore_contents(comment).expect("the directive starts at its `#`");
                &comment[..=usize::from(offset) + codes.len()]
            }
        };
        start + TextSize::of(head)
    }
}

// The text after a directive's codes with `reason` in place of the one it gives, keeping any comment after the reason.
fn with_reason(after: &str, reason: Option<&str>) -> String {
    let Some(reason) = reason else {
        return after.to_owned();
    };
    let (old, tail) = after.split_at(after.find('#').unwrap_or(after.len()));
    let gap = match &old[old.trim_end().len()..] {
        _ if tail.is_empty() => "",
        "" => "  ",
        gap => gap,
    };
    format!(" {reason}{gap}{tail}")
}

fn remove_ignore_selector(source: &str, start: TextSize, selector: &str) -> Option<(TextRange, String)> {
    let comment = &source[TextRange::new(start, source.line_end(start))];
    let (offset, contents) = ignore_contents(comment)?;
    let mut codes: Vec<&str> =
        contents.split(|char: char| char == ',' || char.is_whitespace()).filter(|code| !code.is_empty()).collect();
    codes.remove(codes.iter().position(|code| *code == selector)?);
    if !codes.is_empty() {
        let from = start + offset;
        return Some((TextRange::new(from, from + TextSize::of(contents)), codes.join(", ")));
    }
    let line_start = source.line_start(start);
    let prefix = &source[TextRange::new(line_start, start)];
    if prefix.trim().is_empty() {
        Some((TextRange::new(line_start, source.full_line_end(start)), String::new()))
    } else {
        let trimmed = line_start + TextSize::of(prefix.trim_end());
        Some((TextRange::new(trimmed, source.line_end(start)), String::new()))
    }
}

fn changed_region(before: &str, after: &str) -> (TextRange, String) {
    let prefix: usize = before
        .chars()
        .zip(after.chars())
        .take_while(|(left, right)| left == right)
        .map(|(char, _)| char.len_utf8())
        .sum();
    let suffix: usize = before[prefix..]
        .chars()
        .rev()
        .zip(after[prefix..].chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(char, _)| char.len_utf8())
        .sum();
    let range = TextRange::new(
        TextSize::try_from(prefix).expect("a source fits a text size"),
        TextSize::try_from(before.len() - suffix).expect("a source fits a text size"),
    );
    (range, after[prefix..after.len() - suffix].to_owned())
}

// Whether the edit leaves no report that was not there before. A report inside the edited span, such as SMP008 on the
// codes of the comment it edits, has no position to map back, so it matches any report of the same rule and message
// that started inside the span before.
fn same_or_fewer_reports(before: &[Diagnostic], after: &[Diagnostic], edit: TextRange, new_len: usize) -> bool {
    let start = edit.start().to_usize();
    let old_len = edit.len().to_usize();
    let mut known: FxHashMap<(&str, &str, usize), usize> = FxHashMap::default();
    for report in before {
        let Some(range) = report.range() else { return false };
        let position = range.start().to_usize();
        let mapped = if (start..start + old_len).contains(&position) { start } else { position };
        *known.entry((report.secondary_code_or_id(), report.headline_message(), mapped)).or_default() += 1;
    }
    let mut observed = FxHashMap::default();
    after.iter().all(|report| {
        let Some(range) = report.range() else { return false };
        let position = range.start().to_usize();
        let mapped = if position < start {
            position
        } else if position < start + new_len {
            start
        } else {
            position + old_len - new_len
        };
        let key = (report.secondary_code_or_id(), report.headline_message(), mapped);
        let count = observed.entry(key).or_insert(0);
        *count += 1;
        *count <= *known.get(&key).unwrap_or(&0)
    })
}

impl Line {
    fn suppresses(&self, code: &str) -> bool {
        self.noqa.as_ref().is_some_and(|(_, directive)| directive.suppresses(code))
            || self
                .ignore
                .as_ref()
                .is_some_and(|(_, codes)| codes.iter().any(|candidate| matches_ignore(candidate, code)))
    }
}

// Whether a comment's `noqa` or `simplint: ignore` directive suppresses `code` on its line.
pub fn suppresses(comment: &str, code: &str) -> bool {
    Directive::parse(comment).is_some_and(|(_, directive)| directive.suppresses(code))
        || parse_ignore(comment).is_some_and(|(_, codes)| codes.iter().any(|candidate| matches_ignore(candidate, code)))
}

fn matches_ignore(candidate: &str, code: &str) -> bool {
    candidate == code || Rule::from_code(code).is_some_and(|rule| rule.name() == candidate)
}

// Whether the file has a file-level `ruff: noqa`, `flake8: noqa` or `ruff: file-ignore`, the directives `Suppressions`
// applies to every line. A fix that rewrites a docstring can shorten the lines or drop the words such a directive
// silences, which leaves its codes unused, and Ruff reports them as RUF100.
pub fn has_file_noqa(source: &str, comments: &CommentRanges) -> bool {
    comments.iter().any(|range| {
        Directive::parse_file_level(&source[*range], indentation_at_offset(range.start(), source)).is_some()
    })
}

// Whether a comment's `ruff: ignore` directive names one of `rule`, a rule's code and name, as Ruff takes either.
pub fn ruff_ignores(comment: &str, rule: [&str; 2]) -> bool {
    parse_bracket(comment, "ruff", "ignore[")
        .is_some_and(|(_, codes)| codes.iter().any(|code| rule.contains(&code.as_str())))
}

// A `noqa` directive; one without codes suppresses every rule.
struct Directive {
    codes: Vec<String>,
}

impl Directive {
    // The directive and the offset of its `#` in the comment. Ruff reads the first `# noqa` only, and when its codes do
    // not lex it applies none, rather than looking further along the comment.
    fn parse(comment: &str) -> Option<(TextSize, Self)> {
        let (index, after) = comment
            .match_indices('#')
            .find_map(|(index, _)| Some((index, strip_noqa(comment[index + 1..].trim_start())?)))?;
        let (codes, _) = noqa_codes(after)?;
        Some((TextSize::try_from(index).expect("a comment fits a text size"), Self { codes: owned(codes) }))
    }

    // Ruff's file-level exemptions: `# ruff: noqa` (or `# flake8: noqa`) on its own line, blanket or with codes, and
    // `# ruff: file-ignore[...]` at module scope. Ruff skips a `noqa` whose codes do not parse rather than reading it
    // as blanket, so the codes follow Ruff's `lex_directive`.
    fn parse_file_level(comment: &str, indentation: Option<&str>) -> Option<Self> {
        if let Some((_, codes)) = parse_bracket(comment, "ruff", "file-ignore[") {
            return (indentation == Some("") && !codes.is_empty()).then_some(Self { codes });
        }
        indentation?;
        let rest = comment.match_indices('#').find_map(|(index, _)| {
            let rest = comment[index + 1..].trim_start();
            let rest = after_tool(rest, "ruff").or_else(|| after_tool(rest, "flake8"))?;
            strip_noqa(rest)
        })?;
        Some(Self { codes: owned(noqa_codes(rest)?.0) })
    }

    fn is_blanket(&self) -> bool {
        self.codes.is_empty()
    }

    // A blanket `# noqa` is itself what SMP003 and SMP004 report, so it cannot silence those reports. A `file-ignore`
    // takes rule names as well as codes, as a line's `ruff: ignore` does.
    fn suppresses(&self, code: &str) -> bool {
        if self.is_blanket() {
            code != Rule::SuppressionWithoutReason.code() && code != Rule::BlanketIgnore.code()
        } else {
            self.codes.iter().any(|candidate| matches_ignore(candidate, code))
        }
    }
}

// Whether a comment is a `noqa` that names no code, or only `code`, so that once `code` is not reported the comment
// silences nothing and Ruff reports it as unused.
pub fn is_noqa_for_only(comment: &str, code: &str) -> bool {
    let rest = comment.trim().trim_start_matches('#').trim_start();
    let Some((codes, _)) = strip_noqa(rest).and_then(noqa_codes) else {
        return false;
    };
    codes.into_iter().all(|candidate| candidate == code)
}

// The text after a `noqa` keyword, which Ruff matches in any case.
fn strip_noqa(text: &str) -> Option<&str> {
    strip_ignore_case(text, "noqa")
}

// The codes a `noqa` directive names after its keyword and the text after them, as Ruff's `NoqaLexer` reads them; no
// codes is a blanket directive. `None` where Ruff rejects the whole directive: a character other than a space, `#` or
// `:` follows the keyword, a colon names no code, or a code runs into a character no code holds.
pub fn noqa_codes(after: &str) -> Option<(Vec<&str>, &str)> {
    let mut rest = match after.chars().next() {
        None | Some('#') => return Some((Vec::new(), after)),
        Some(':') => &after[1..],
        Some(char) if char.is_whitespace() => match after.trim_start().strip_prefix(':') {
            Some(list) => list,
            None => return Some((Vec::new(), after)),
        },
        Some(_) => return None,
    };
    let mut codes = Vec::new();
    // Ruff splits codes written together, as in `F401F841`, but then rejects a word after them that is no code.
    let mut joined = false;
    loop {
        rest = rest.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        let letters = rest.find(|char: char| !char.is_ascii_uppercase()).unwrap_or(rest.len());
        let digits = rest[letters..].find(|char: char| !char.is_ascii_digit()).unwrap_or(rest.len() - letters);
        if letters == 0 || digits == 0 {
            // The first word that is no code ends the list, as in `# noqa: F401 kept for its side effect`.
            return (!joined && !codes.is_empty()).then_some((codes, rest));
        }
        let (code, tail) = rest.split_at(letters + digits);
        codes.push(code);
        rest = tail;
        joined = match tail.chars().next() {
            None | Some(',' | '#') => false,
            Some(char) if char.is_whitespace() => false,
            Some(char) if char.is_ascii_uppercase() => true,
            Some(_) => return None,
        };
    }
}

fn owned(codes: Vec<&str>) -> Vec<String> {
    codes.into_iter().map(str::to_owned).collect()
}

fn strip_ignore_case<'t>(text: &'t str, prefix: &str) -> Option<&'t str> {
    text.get(..prefix.len()).filter(|head| head.eq_ignore_ascii_case(prefix)).map(|_| &text[prefix.len()..])
}

fn code_list(codes: &str) -> impl Iterator<Item = &str> {
    codes.split(|char: char| char == ',' || char.is_whitespace()).filter(|part| !part.is_empty())
}

// A `# simplint: ignore[SMP001, SMP901]` comment, simplint's counterpart of Ruff's `# ruff: ignore[...]`: the codes
// and the offset of its `#` in the comment.
fn parse_ignore(comment: &str) -> Option<(TextSize, Vec<String>)> {
    parse_bracket(comment, "simplint", "ignore[")
}

pub(crate) fn ignore_contents(comment: &str) -> Option<(TextSize, &str)> {
    comment.match_indices('#').find_map(|(index, _)| {
        let rest = after_tool(comment[index + 1..].trim_start(), "simplint")?.strip_prefix("ignore[")?;
        let (codes, _) = rest.split_once(']')?;
        Some((TextSize::try_from(comment.len() - rest.len()).expect("a comment fits a text size"), codes))
    })
}

// A `# tool: action[CODES]` comment: the codes and the offset of its `#` in the comment.
fn parse_bracket(comment: &str, tool: &str, action: &str) -> Option<(TextSize, Vec<String>)> {
    comment.match_indices('#').find_map(|(index, _)| {
        let rest = after_tool(comment[index + 1..].trim_start(), tool)?;
        let (codes, _) = rest.strip_prefix(action)?.split_once(']')?;
        let codes = code_list(codes).map(str::to_owned).collect();
        Some((TextSize::try_from(index).expect("a comment fits a text size"), codes))
    })
}

// What follows `tool:` at the start of a directive, with any space around the colon, as Ruff reads `# ruff : noqa`. The
// tool name is matched in lowercase only, as Ruff does.
pub fn after_tool<'t>(text: &'t str, tool: &str) -> Option<&'t str> {
    Some(text.strip_prefix(tool)?.trim_start().strip_prefix(':')?.trim_start())
}
