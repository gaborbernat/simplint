use test_case::test_case;

use crate::common;

#[test_case("test-class", "tests/test_class.py" ; "test_class")]
#[test_case("test-class", "multiline_module_docstring_ok.py" ; "test_class_regular_module")]
#[test_case("parametrize-without-id", "tests/test_parametrize_without_id.py" ; "parametrize_without_id")]
#[test_case("conditional-in-test", "tests/test_conditional_in_test.py" ; "conditional_in_test")]
#[test_case("conditional-in-test", "multiline_module_docstring_ok.py" ; "conditional_in_test_regular_module")]
#[test_case("constant-assert", "constant_assert.py" ; "constant_assert")]
#[test_case("mock-attribute-assignment", "tests/test_mock_attribute_assignment.py" ; "mock_attribute_assignment")]
#[test_case("mock-without-spec", "tests/test_mock_without_spec.py" ; "mock_without_spec")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}
