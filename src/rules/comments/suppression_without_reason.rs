use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::{is_tool_directive, own_line_comment_runs};

pub fn suppression_without_reason(checker: &Checker) {
    let mut runs = None;
    for (range, comment) in checker.state().comments(checker) {
        if comment.directives.is_empty() || comment.has_reason {
            continue;
        }
        let runs = runs.get_or_insert_with(|| own_line_comment_runs(checker));
        if !reason_above(checker, runs, *range) {
            checker.report(
                Rule::SuppressionWithoutReason,
                *range,
                "Suppression comment gives no reason; add one after the codes",
            );
        }
    }
}

// Whether a run of comment lines that ends on the line above a trailing suppression gives its reason. A suppression on
// a line of its own applies to no line of code, so it has no line to take a reason from.
fn reason_above(checker: &Checker, runs: &[Vec<TextRange>], comment: TextRange) -> bool {
    let source = checker.source;
    if !has_leading_content(comment.start(), source) {
        return false;
    }
    let line = source.line_start(comment.start());
    runs.iter()
        .filter(|run| run.last().is_some_and(|last| source.full_line_end(last.end()) == line))
        .any(|run| run.iter().any(|reason| gives_reason(checker.text(*reason))))
}

// Whether a comment line says something a directive does not, such as a sentence before a `ty: ignore` on the line.
fn gives_reason(comment: &str) -> bool {
    !is_tool_directive(comment) && !comment.trim_start_matches('#').trim().is_empty()
}
