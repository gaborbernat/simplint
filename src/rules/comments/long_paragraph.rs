use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::long_paragraphs;

pub fn long_paragraph(checker: &Checker) {
    let source = checker.source;
    let limit = checker.settings.project.knobs.max_paragraph_words;
    let report = |range| {
        checker.report(
            Rule::LongParagraph,
            range,
            format!("Paragraph exceeds {limit} words; split it at a change of topic"),
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
            long_comment_paragraphs(source, &block, limit).into_iter().for_each(report);
            block.clear();
        }
        block.push(range);
    }
    long_comment_paragraphs(source, &block, limit).into_iter().for_each(report);
}

fn long_comment_paragraphs(source: &str, block: &[TextRange], limit: usize) -> Vec<TextRange> {
    long_paragraphs(
        block.iter().map(|range| {
            let text = source[*range].trim_start_matches('#').trim();
            (!text.is_empty()).then(|| (*range, text.split_whitespace().count()))
        }),
        limit,
    )
}
