use test_case::test_case;

use crate::common;

#[test_case("section-comment", "section_comment.py" ; "section_comment")]
#[test_case("long-paragraph", "long_paragraph.py" ; "long_paragraph")]
#[test_case("suppression-without-reason", "suppression_without_reason.py" ; "suppression_without_reason")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}
