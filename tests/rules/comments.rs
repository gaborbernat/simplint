use test_case::test_case;

use crate::common;

#[test_case("section-comment", "section_comment.py" ; "section_comment")]
#[test_case("long-paragraph", "long_paragraph.py" ; "long_paragraph")]
#[test_case("suppression-without-reason", "suppression_without_reason.py" ; "suppression_without_reason")]
#[test_case("blanket-ignore", "blanket_ignore.py" ; "blanket_ignore")]
#[test_case("restating-comment", "restating_comment.py" ; "restating_comment")]
#[test_case("string-statement-comment", "string_statement_comment.py" ; "string_statement_comment")]
#[test_case("long-comment-line", "long_comment_line.py" ; "long_comment_line")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}
