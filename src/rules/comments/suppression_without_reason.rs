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

fn lines_with_reason(checker: &Checker) -> FxHashMap<TextSize, bool> {
    let source = checker.source;
    let mut reasons = FxHashMap::default();
    for run in own_line_comment_runs(checker) {
        // The runs of own-line comments are never empty.
        let last = run[run.len() - 1];
        let gives = run.iter().any(|reason| gives_reason(checker.text(*reason)));
        *reasons.entry(source.full_line_end(last.end())).or_default() |= gives;
    }
    reasons
}

// A directive alone gives no reason, but a sentence on a line of its own does.
fn gives_reason(comment: &str) -> bool {
    !is_tool_directive(comment) && !comment.trim_start_matches('#').trim().is_empty()
}

// A suppression on a line of its own applies to no line of code, so it takes no reason from above.
fn reason_above(checker: &Checker, reasons: &FxHashMap<TextSize, bool>, comment: TextRange) -> bool {
    let source = checker.source;
    if !has_leading_content(comment.start(), source) {
        return false;
    }
    reasons.get(&source.line_start(comment.start())).copied().unwrap_or_default()
}
