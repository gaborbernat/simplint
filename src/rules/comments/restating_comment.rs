use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::own_line_comment_runs;
use crate::syntax::identifier_words;

// Punctuation can mark a label, directive or code example. Compare plain prose as a block because a short line
// such as `# status.` can complete the preceding sentence.
pub fn restating_comment(checker: &Checker) {
    for block in own_line_comment_runs(checker) {
        let texts: Vec<&str> = block.iter().map(|comment| checker.text(*comment).trim_start_matches('#')).collect();
        if !texts.iter().all(|text| text.chars().all(|char| char.is_alphanumeric() || " \t_.,;!?'-".contains(char))) {
            continue;
        }
        let words: Vec<String> = texts
            .iter()
            .flat_map(|text| identifier_words(text).filter(|word| !word.is_empty()).map(str::to_lowercase))
            .collect();
        // The runs of own-line comments are never empty.
        let last = block[block.len() - 1];
        let source = checker.source;
        let next = source.full_line_end(last.end());
        if words.is_empty() || next.to_usize() >= source.len() {
            continue;
        }
        let tokens = checker.parsed.tokens();
        // Use the entire logical line so wrapping a statement cannot change which words match.
        let code: FxHashSet<String> = tokens[tokens.partition_point(|token| token.start() < next)..]
            .iter()
            .take_while(|token| token.kind() != TokenKind::Newline)
            .filter(|token| token.kind() == TokenKind::Identifier || token.kind().is_keyword())
            .map(|token| checker.text(token.range()).to_lowercase())
            .collect();
        if code.is_empty() || !words.iter().all(|word| code.contains(word.as_str())) {
            continue;
        }
        let first = block[0];
        let lines = TextRange::new(source.line_start(first.start()), source.full_line_end(last.end()));
        checker.report_fix(
            Rule::RestatingComment,
            TextRange::new(first.start(), last.end()),
            "Comment repeats the code below; the code already says it",
            Some(Fix::safe_edit(Edit::range_deletion(lines))),
            "Remove the comment",
        );
    }
}
