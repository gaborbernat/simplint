use ruff_diagnostics::{Edit, Fix};
use ruff_source_file::LineRanges;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::follows;

const DIVIDER_CHARS: &[char] = &['-', '=', '*', '#', '~', '_', '+', '/'];

pub fn section_comment(checker: &Checker) {
    let source = checker.source;
    let comments = checker.state().own_line_comments(checker);
    for (index, range) in comments.iter().copied().enumerate() {
        let content = source[range].trim_start_matches('#').trim();
        // A sentence wrapped onto a line that ends in an operator such as `===` continues the prose above it rather
        // than labeling code.
        let continues_prose = !source[range].starts_with("##")
            && content.chars().any(char::is_alphanumeric)
            && divider_run(content.chars()) == 0
            && !content.to_ascii_lowercase().contains("region")
            && index.checked_sub(1).map(|previous| comments[previous]).is_some_and(|previous| {
                follows(source, previous, range)
                    && source[previous].chars().any(char::is_alphanumeric)
                    && !is_section_comment(&source[previous])
            });
        // A short line of a `##` paragraph, a URL or code after `##` is no heading, so it is left to Ruff too.
        let ruff_prose = left_to_ruff(&source[range])
            && (prose(&source[range]).is_some()
                || !is_title(content)
                || index
                    .checked_sub(1)
                    .map(|previous| comments[previous])
                    .is_some_and(|previous| follows(source, previous, range) && left_to_ruff(&source[previous])));
        if continues_prose || ruff_prose || !is_section_comment(&source[range]) {
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

fn divider_run(mut chars: impl Iterator<Item = char>) -> usize {
    let Some(first) = chars.next().filter(|char| DIVIDER_CHARS.contains(char)) else {
        return 0;
    };
    1 + chars.take_while(|char| *char == first).count()
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
        // Runs split by spaces rule off the columns of a table, such as `------  ------`.
        let table_rule =
            content.split_whitespace().nth(1).is_some() && content.split_whitespace().any(|run| run.len() > 1);
        return !table_rule
            && content.chars().filter(|char| !char.is_whitespace()).count() + hashes >= 4
            && content.chars().all(|char| char.is_whitespace() || DIVIDER_CHARS.contains(&char));
    }
    // Two repeated characters also end prose (`C++`), so a single run needs three; a pair on both ends frames a title.
    let (leading, trailing) = decoration(content);
    hashes >= 2 || leading.max(trailing) >= 3 || leading.min(trailing) >= 2
}

// A divider run is decoration at three or more characters, or when a space separates it from the words, so `C++` and
// `__init__` keep their own characters. Divider characters are ASCII, so the lengths count bytes too.
fn decoration(content: &str) -> (usize, usize) {
    let leading = divider_run(content.chars());
    let leading = if leading >= 3 || content[leading..].starts_with(' ') { leading } else { 0 };
    let trailing = divider_run(content[leading..].chars().rev());
    let trailing = if trailing >= 3 || content[..content.len() - trailing].ends_with(' ') { trailing } else { 0 };
    (leading, trailing)
}

// Ruff's E266 and E265 report text after more than one `#`. Their fix keeps a decorated comment's decoration, and a
// title's text where this rule removes it.
fn left_to_ruff(comment: &str) -> bool {
    let hashes = comment.chars().take_while(|char| *char == '#').count();
    let content = comment[hashes..].trim();
    hashes >= 2 && content.chars().any(char::is_alphanumeric) && decoration(content) == (0, 0)
}

fn is_title(content: &str) -> bool {
    !content.contains("://") && !content.contains(['(', ')', '[', ']', '{', '}', '=', '<', '>', '`'])
}

// Up to three words are a title that labels the code below, so the fix removes the comment with them.
fn prose(comment: &str) -> Option<&str> {
    let content = comment.trim_start_matches('#').trim();
    if content.split_whitespace().filter(|word| word.chars().any(char::is_alphanumeric)).count() <= 3
        || content.to_ascii_lowercase().starts_with("region ")
    {
        return None;
    }
    let (leading, trailing) = decoration(content);
    Some(content[leading..content.len() - trailing].trim())
}
