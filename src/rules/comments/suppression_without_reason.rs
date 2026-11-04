use super::directive::Comment;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn suppression_without_reason(checker: &Checker) {
    for range in checker.comment_ranges() {
        let comment = Comment::parse(checker.text(range));
        if !comment.directives.is_empty() && !comment.has_reason {
            checker.report(
                Rule::SuppressionWithoutReason,
                range,
                "Suppression comment gives no reason; add one after the codes",
            );
        }
    }
}
