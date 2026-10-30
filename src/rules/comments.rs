use ruff_diagnostics::{Edit, Fix};
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::noqa::is_code;
use crate::registry::Rule;
use crate::rules::{class_defs, docstring, function_defs};

const MAX_PARAGRAPH_WORDS: usize = 100;
const DIVIDER_CHARS: &[char] = &['-', '=', '*', '#', '~', '_', '+', '/', '<', '>'];

pub fn section_comment(checker: &mut Checker) {
    let source = checker.source;
    let comments: Vec<TextRange> =
        checker.comments().filter(|range| !has_leading_content(range.start(), source)).collect();
    for range in comments {
        if is_section_comment(&source[range]) {
            checker.report(
                Rule::SectionComment,
                range,
                "Section comment groups code; extract a named function or class instead",
                Some(Fix::safe_edit(Edit::range_deletion(source.full_line_range(range.start())))),
            );
        }
    }
}

fn is_section_comment(comment: &str) -> bool {
    let hashes = comment.chars().take_while(|char| *char == '#').count();
    let content = comment[hashes..].trim();
    if content.starts_with('!') {
        return false;
    }
    let lower = content.to_ascii_lowercase();
    if lower == "region" || lower == "endregion" || lower.starts_with("region ") {
        return true;
    }
    if !content.chars().any(char::is_alphanumeric) {
        return content.chars().filter(|char| !char.is_whitespace()).count() + hashes >= 4
            && content.chars().all(|char| char.is_whitespace() || DIVIDER_CHARS.contains(&char));
    }
    hashes >= 2 && !content.is_empty() || divider_run(content.chars()) || divider_run(content.chars().rev())
}

fn divider_run(mut chars: impl Iterator<Item = char>) -> bool {
    chars.next().filter(|char| DIVIDER_CHARS.contains(char)).is_some_and(|first| chars.next() == Some(first))
}

pub fn long_paragraph(checker: &mut Checker) {
    let source = checker.source;
    let mut long = Vec::new();
    let mut block: Vec<TextRange> = Vec::new();
    let comments: Vec<TextRange> =
        checker.comments().filter(|range| !has_leading_content(range.start(), source)).collect();
    for range in comments {
        let continues = block.last().is_some_and(|previous| {
            source.full_line_end(previous.end()) == source.line_start(range.start())
                && previous.start() - source.line_start(previous.start())
                    == range.start() - source.line_start(range.start())
        });
        if !continues {
            long.extend(long_comment_paragraphs(source, &block));
            block.clear();
        }
        block.push(range);
    }
    long.extend(long_comment_paragraphs(source, &block));
    let body = checker.body();
    let docstrings = std::iter::once(docstring(body))
        .chain(function_defs(body).into_iter().map(|function| docstring(&function.body)))
        .chain(class_defs(body).into_iter().map(|class| docstring(&class.body)))
        .flatten();
    for literal in docstrings {
        if literal
            .value
            .to_str()
            .split("\n\n")
            .any(|paragraph| paragraph.split_whitespace().count() > MAX_PARAGRAPH_WORDS)
        {
            long.push(literal.range());
        }
    }
    for range in long {
        checker.report(
            Rule::LongParagraph,
            range,
            format!("Paragraph exceeds {MAX_PARAGRAPH_WORDS} words; split it at a change of topic"),
            None,
        );
    }
}

fn long_comment_paragraphs(source: &str, block: &[TextRange]) -> Vec<TextRange> {
    let mut long = Vec::new();
    let mut words = 0;
    let mut start = None;
    for range in block {
        let text = source[*range].trim_start_matches('#').trim();
        if text.is_empty() {
            words = 0;
            start = None;
            continue;
        }
        let first = *start.get_or_insert(range.start());
        words += text.split_whitespace().count();
        if words > MAX_PARAGRAPH_WORDS && long.last().is_none_or(|last: &TextRange| last.start() != first) {
            long.push(TextRange::new(first, range.end()));
        }
    }
    long
}

pub fn suppression_without_reason(checker: &mut Checker) {
    let comments: Vec<TextRange> = checker.comments().collect();
    for range in comments {
        let segments: Vec<Segment> = checker.text(range).split('#').map(Segment::parse).collect();
        let directives: Vec<&Segment> = segments.iter().filter(|segment| segment.directive.is_some()).collect();
        if directives.is_empty() {
            continue;
        }
        if let Some(blanket) = directives.iter().find(|segment| segment.missing_codes) {
            let message = format!("`{}` names no error code", blanket.directive.unwrap_or_default());
            checker.report(Rule::SuppressionWithoutReason, range, message, None);
        } else if !segments.iter().any(|segment| segment.has_reason) {
            checker.report(
                Rule::SuppressionWithoutReason,
                range,
                "Suppression comment gives no reason; add one after the codes",
                None,
            );
        }
    }
}

struct Segment {
    directive: Option<&'static str>,
    missing_codes: bool,
    has_reason: bool,
}

impl Segment {
    fn parse(segment: &str) -> Self {
        let text = segment.trim();
        let lower = text.to_ascii_lowercase();
        for (prefix, codes) in [
            ("ruff: noqa", Codes::Colon),
            ("noqa", Codes::Colon),
            ("type: ignore", Codes::Bracket),
            ("ty: ignore", Codes::Bracket),
            ("pyright: ignore", Codes::Bracket),
            ("pragma: no cover", Codes::None),
            ("pragma: no branch", Codes::None),
        ] {
            if let Some(rest) = lower.strip_prefix(prefix) {
                let rest = &text[text.len() - rest.len()..];
                let (named, reason) = codes.split(rest);
                return Self {
                    directive: Some(prefix),
                    missing_codes: !named && matches!(prefix, "ty: ignore" | "pyright: ignore"),
                    has_reason: !reason.trim().is_empty(),
                };
            }
        }
        Self { directive: None, missing_codes: false, has_reason: !text.is_empty() }
    }
}

enum Codes {
    Colon,
    Bracket,
    None,
}

impl Codes {
    fn split(self, rest: &str) -> (bool, &str) {
        match self {
            Self::Colon => {
                let Some(codes) = rest.trim_start().strip_prefix(':') else {
                    return (false, rest);
                };
                let mut rest = codes;
                let mut named = false;
                loop {
                    let trimmed = rest.trim_start_matches(|char: char| char == ',' || char.is_whitespace());
                    let end = trimmed.find(|char: char| char == ',' || char.is_whitespace()).unwrap_or(trimmed.len());
                    if end == 0 || !is_code(&trimmed[..end]) {
                        return (named, trimmed);
                    }
                    named = true;
                    rest = &trimmed[end..];
                }
            }
            Self::Bracket => match rest.strip_prefix('[').and_then(|inner| inner.split_once(']')) {
                Some((codes, reason)) => (!codes.trim().is_empty(), reason),
                None => (false, rest),
            },
            Self::None => (true, rest),
        }
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::SectionComment, "SMP001.py" ; "section_comment")]
    #[test_case(Rule::LongParagraph, "SMP005.py" ; "long_paragraph")]
    #[test_case(Rule::SuppressionWithoutReason, "SMP060.py" ; "suppression_without_reason")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
