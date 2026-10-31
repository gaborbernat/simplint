use test_case::test_case;

use crate::common;

#[test_case("string-concatenation", "string_concatenation.py", "py314" ; "string_concatenation")]
#[test_case("string-concatenation", "string_concatenation_old.py", "py310" ; "string_concatenation_py310")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
