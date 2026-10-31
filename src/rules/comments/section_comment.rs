use ruff_diagnostics::{Edit, Fix};
use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

const DIVIDER_CHARS: &[char] = &['-', '=', '*', '#', '~', '_', '+', '/'];

pub fn section_comment(checker: &Checker) {
    let source = checker.source;
    let comments: Vec<TextRange> =
        checker.comment_ranges().iter().copied().filter(|range| !has_leading_content(range.start(), source)).collect();
    // A sentence wrapped onto a line that ends in an operator such as `===` continues the prose above it rather than
    // labeling code.
    let is_prose =
        |range: TextRange| source[range].chars().any(char::is_alphanumeric) && !is_section_comment(&source[range]);
    for (index, range) in comments.iter().copied().enumerate() {
        let content = source[range].trim_start_matches('#').trim();
        let continues_prose = !source[range].starts_with("##")
            && content.chars().any(char::is_alphanumeric)
            && divider_run(content.chars()) == 0
            && !content.to_ascii_lowercase().contains("region")
            && index.checked_sub(1).map(|previous| comments[previous]).is_some_and(|previous| {
                source.full_line_end(previous.end()) == source.line_start(range.start()) && is_prose(previous)
            });
        if !continues_prose && is_section_comment(&source[range]) {
            checker.report_fix(
                Rule::SectionComment,
                range,
                "Section comment groups code; extract a named function or class instead",
                Some(Fix::safe_edit(Edit::range_deletion(source.full_line_range(range.start())))),
                "Remove the section comment",
            );
        }
    }
}

fn is_section_comment(comment: &str) -> bool {
    let hashes = comment.chars().take_while(|char| *char == '#').count();
    let content = comment[hashes..].trim();
    // `#!` is a shebang and `# ///` opens or closes PEP 723 inline script metadata.
    if content.starts_with('!') || content == "///" || content.starts_with("/// ") {
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
    // Two repeated characters also end prose (`C++`), so a single run needs three; a pair on both ends frames a title.
    let (leading, trailing) = (divider_run(content.chars()), divider_run(content.chars().rev()));
    hashes >= 2 && !content.is_empty() || leading.max(trailing) >= 3 || leading.min(trailing) >= 2
}

fn divider_run(mut chars: impl Iterator<Item = char>) -> usize {
    let Some(first) = chars.next().filter(|char| DIVIDER_CHARS.contains(char)) else {
        return 0;
    };
    1 + chars.take_while(|char| *char == first).count()
}
