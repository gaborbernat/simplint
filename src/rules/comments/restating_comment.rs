use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::own_line_comment_runs;

// A comment whose every word is a name or keyword of the line below only repeats that code, so deleting it loses
// nothing. Words match whole names: `create_user` does not spell out `create`. Only plain prose can restate: a colon
// marks a directive or a label, such as `# type: ignore` or `# Note:`, and code punctuation quotes syntax, such as
// `# *args` or `# depth > limit`, which says more than the names in it. Consecutive own-line comments form one block,
// judged as a whole, since a line such as `# status.` completes the sentence above it.
pub fn restating_comment(checker: &Checker) {
    for block in own_line_comment_runs(checker) {
        restating_block(checker, &block);
    }
}

fn restating_block(checker: &Checker, block: &[TextRange]) {
    let source = checker.source;
    // The runs of own-line comments are never empty.
    let (first, last) = (block[0], block[block.len() - 1]);
    let texts: Vec<&str> = block.iter().map(|comment| checker.text(*comment).trim_start_matches('#')).collect();
    if !texts.iter().all(|text| text.chars().all(|char| char.is_alphanumeric() || " \t_.,;!?'-".contains(char))) {
        return;
    }
    let words: Vec<String> = texts.iter().flat_map(|text| words(text)).collect();
    let next = source.full_line_end(last.end());
    if words.is_empty() || next.to_usize() >= source.len() {
        return;
    }
    let tokens = checker.parsed.tokens();
    // The statement below is the code the comment restates, all of it: `ruff format` joins a statement that spans lines
    // into one, so the words of its later lines must count the same before and after the format.
    let from = tokens.partition_point(|token| token.start() < next);
    let code: FxHashSet<String> = tokens[from..]
        .iter()
        .take_while(|token| token.kind() != TokenKind::Newline)
        .filter(|token| token.kind() == TokenKind::Identifier || token.kind().is_keyword())
        .map(|token| checker.text(token.range()).to_lowercase())
        .collect();
    if code.is_empty() || !words.iter().all(|word| code.contains(word.as_str())) {
        return;
    }
    let lines = TextRange::new(source.line_start(first.start()), source.full_line_end(last.end()));
    checker.report_fix(
        Rule::RestatingComment,
        TextRange::new(first.start(), last.end()),
        "Comment repeats the code below; the code already says it",
        Some(Fix::safe_edit(Edit::range_deletion(lines))),
        "Remove the comment",
    );
}

// The lowercase words of a comment, split at anything that is not a letter, digit or underscore.
fn words(text: &str) -> Vec<String> {
    text.split(|char: char| !(char.is_alphanumeric() || char == '_'))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}
