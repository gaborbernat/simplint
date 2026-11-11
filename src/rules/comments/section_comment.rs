use ruff_diagnostics::{Edit, Fix};
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::follows;

const DIVIDER_CHARS: &[char] = &['-', '=', '*', '#', '~', '_', '+', '/'];

pub fn section_comment(checker: &Checker) {
    let source = checker.source;
    let comments = checker.state().own_line_comments(checker);
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
            && index
                .checked_sub(1)
                .map(|previous| comments[previous])
                .is_some_and(|previous| follows(source, previous, range) && is_prose(previous));
        if continues_prose || !is_section_comment(&source[range]) {
            continue;
        }
        match prose(&source[range]) {
            Some(text) => checker.report_fix(
                Rule::SectionComment,
                range,
                "Decoration makes the comment read as a section divider; keep its text alone",
                Some(Fix::safe_edit(Edit::range_replacement(format!("# {text}"), range))),
                "Remove the decoration",
            ),
            None => checker.report_fix(
                Rule::SectionComment,
                range,
                "Section comment groups code; extract a named function or class instead",
                Some(Fix::safe_edit(Edit::range_deletion(source.full_line_range(range.start())))),
                "Remove the section comment",
            ),
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
    let (leading, trailing) = decoration(content);
    hashes >= 2 && !content.is_empty() || leading.max(trailing) >= 3 || leading.min(trailing) >= 2
}

// The text of a section comment with more words than a title, such as an instruction, with the decoration around it
// stripped. Up to three words only label the code below, as in `# === helpers ===`, and go with the comment.
fn prose(comment: &str) -> Option<&str> {
    let content = comment.trim_start_matches('#').trim();
    let words = content.split_whitespace().filter(|word| word.chars().any(char::is_alphanumeric)).count();
    if words <= 3 || content.to_ascii_lowercase().starts_with("region ") {
        return None;
    }
    let (leading, trailing) = decoration(content);
    Some(content[leading..content.len() - trailing].trim())
}

// The divider runs that open and close a comment's text as decoration: a run of three or more, or one set apart from
// the words by a space, so `C++` and the underscores of `__init__` belong to the words. Divider characters are ASCII,
// so the lengths count bytes too.
fn decoration(content: &str) -> (usize, usize) {
    let leading = divider_run(content.chars());
    let leading = if leading >= 3 || content[leading..].starts_with(' ') { leading } else { 0 };
    let trailing = divider_run(content[leading..].chars().rev());
    let trailing = if trailing >= 3 || content[..content.len() - trailing].ends_with(' ') { trailing } else { 0 };
    (leading, trailing)
}

fn divider_run(mut chars: impl Iterator<Item = char>) -> usize {
    let Some(first) = chars.next().filter(|char| DIVIDER_CHARS.contains(char)) else {
        return 0;
    };
    1 + chars.take_while(|char| *char == first).count()
}
