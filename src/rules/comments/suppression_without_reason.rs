use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn suppression_without_reason(checker: &Checker) {
    for (range, comment) in checker.state().comments(checker) {
        if !comment.directives.is_empty() && !comment.has_reason {
            checker.report(
                Rule::SuppressionWithoutReason,
                *range,
                "Suppression comment gives no reason; add one after the codes",
            );
        }
    }
}
