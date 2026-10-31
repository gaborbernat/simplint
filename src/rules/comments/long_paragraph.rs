use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;

const MAX_PARAGRAPH_WORDS: usize = 100;

pub fn long_paragraph(checker: &Checker) {
    let source = checker.source;
    let report = |range| {
        checker.report(
            Rule::LongParagraph,
            range,
            format!("Paragraph exceeds {MAX_PARAGRAPH_WORDS} words; split it at a change of topic"),
        );
    };
    let mut block: Vec<TextRange> = Vec::new();
    let comments = checker.comment_ranges().iter().copied().filter(|range| !has_leading_content(range.start(), source));
    for range in comments {
        let continues = block.last().is_some_and(|previous| {
            source.full_line_end(previous.end()) == source.line_start(range.start())
                && previous.start() - source.line_start(previous.start())
                    == range.start() - source.line_start(range.start())
        });
        if !continues {
            long_comment_paragraphs(source, &block).into_iter().for_each(report);
            block.clear();
        }
        block.push(range);
    }
    long_comment_paragraphs(source, &block).into_iter().for_each(report);
    let body = checker.body();
    let docstrings = std::iter::once(docstring(body))
        .chain(checker.function_defs().iter().map(|function| docstring(&function.body)))
        .chain(checker.class_defs().iter().map(|class| docstring(&class.body)))
        .flatten();
    for literal in docstrings {
        if docstring_paragraphs(literal.value.to_str()).any(|paragraph| {
            paragraph.iter().map(|line| line.split_whitespace().count()).sum::<usize>() > MAX_PARAGRAPH_WORDS
        }) {
            report(literal.range());
        }
    }
}

// A blank line ends a paragraph, and so does the start of a reST field such as `:param name:`, since each field documents
// its own topic.
fn docstring_paragraphs(text: &str) -> impl Iterator<Item = Vec<&str>> {
    let mut paragraphs: Vec<Vec<&str>> = vec![Vec::new()];
    for line in text.lines() {
        let trimmed = line.trim();
        let field = trimmed.starts_with(':') && trimmed[1..].contains(':');
        if trimmed.is_empty() || field {
            paragraphs.push(Vec::new());
        }
        if !trimmed.is_empty() {
            paragraphs.last_mut().expect("starts with one paragraph").push(line);
        }
    }
    paragraphs.into_iter()
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
