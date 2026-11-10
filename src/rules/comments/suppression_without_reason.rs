use ruff_python_trivia::has_leading_content;
use ruff_source_file::LineRanges;
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::{is_tool_directive, own_line_comment_runs};

pub fn suppression_without_reason(checker: &Checker) {
    let mut reasons = None;
    for (range, comment) in checker.state().comments(checker) {
        if comment.directives.is_empty() || comment.has_reason {
            continue;
        }
        let reasons = reasons.get_or_insert_with(|| lines_with_reason(checker));
        if !reason_above(checker, reasons, *range) {
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
fn reason_above(checker: &Checker, reasons: &FxHashMap<TextSize, bool>, comment: TextRange) -> bool {
    let source = checker.source;
    if !has_leading_content(comment.start(), source) {
        return false;
    }
    reasons.get(&source.line_start(comment.start())).copied().unwrap_or_default()
}

// Whether the run of own-line comments that ends on each line gives a reason, keyed by where that line ends.
fn lines_with_reason(checker: &Checker) -> FxHashMap<TextSize, bool> {
    let source = checker.source;
    let mut reasons = FxHashMap::default();
    for run in own_line_comment_runs(checker) {
        let Some(last) = run.last() else { continue };
        let gives = run.iter().any(|reason| gives_reason(checker.text(*reason)));
        *reasons.entry(source.full_line_end(last.end())).or_default() |= gives;
    }
    reasons
}

// Whether a comment line says something a directive does not, such as a sentence before a `ty: ignore` on the line.
fn gives_reason(comment: &str) -> bool {
    !is_tool_directive(comment) && !comment.trim_start_matches('#').trim().is_empty()
}
