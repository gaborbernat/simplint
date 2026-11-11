use std::iter;

use ruff_text_size::{TextLen, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::lines::indent_width;
use crate::rules::long_paragraphs;
use crate::syntax::docstring;

pub fn long_docstring_paragraph(checker: &Checker) {
    let limit = checker.settings.project.knobs.max_paragraph_words;
    let bodies = iter::once(checker.body())
        .chain(checker.function_defs().iter().map(|function| function.body.as_slice()))
        .chain(checker.class_defs().iter().map(|class| class.body.as_slice()));
    for literal in bodies.filter_map(docstring) {
        // Each part of an implicitly concatenated docstring is read as written, so a report points at its own lines.
        for part in &literal.value {
            for range in long_paragraphs_of(checker.text(part.content_range()), part.content_range().start(), limit) {
                checker.report(
                    Rule::LongDocstringParagraph,
                    range,
                    format!("Docstring paragraph exceeds {limit} words; split it at a change of topic"),
                );
            }
        }
    }
}

// Each paragraph of `text` longer than `limit` words, from its first line to the line that passes the limit. A blank
// line and a doctest line, which is code rather than prose, each end a paragraph.
fn long_paragraphs_of(text: &str, start: TextSize, limit: usize) -> Vec<TextRange> {
    let mut offset = start;
    let lines = text.split_inclusive('\n').map(|line| {
        let line_start = offset;
        offset += line.text_len();
        let content = line.trim();
        if content.is_empty() || content.starts_with(">>>") || content.starts_with("...") {
            return None;
        }
        let indent = TextSize::of(&line[..indent_width(line)]);
        let range = TextRange::new(line_start + indent, line_start + indent + content.text_len());
        Some((range, content.split_whitespace().count()))
    });
    long_paragraphs(lines, limit)
}
