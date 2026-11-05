use test_case::test_case;

use crate::common;

#[test_case("test-class", "tests/test_class.py" ; "test_class")]
#[test_case("test-class", "multiline_module_docstring_ok.py" ; "test_class_regular_module")]
#[test_case("test-class", "tests/test_class_generic.py" ; "test_class_generic")]
#[test_case("parametrize-without-id", "tests/test_parametrize_without_id.py" ; "parametrize_without_id")]
#[test_case("conditional-in-test", "tests/test_conditional_in_test.py" ; "conditional_in_test")]
#[test_case("conditional-in-test", "multiline_module_docstring_ok.py" ; "conditional_in_test_regular_module")]
#[test_case("constant-assert", "constant_assert.py" ; "constant_assert")]
#[test_case("mock-attribute-assignment", "tests/test_mock_attribute_assignment.py" ; "mock_attribute_assignment")]
#[test_case("mock-without-spec", "tests/test_mock_without_spec.py" ; "mock_without_spec")]
#[test_case("split-composite-assert", "tests/test_split_composite_assert.py" ; "split_composite_assert")]
#[test_case("sleep-in-test", "tests/test_sleep_in_test.py" ; "sleep_in_test")]
#[test_case("sleep-in-test", "tests/test_sleep_patched.py" ; "sleep_in_test_patched")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock.py" ; "stdlib_mock")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock_mocker.py" ; "stdlib_mock_mocker")]
#[test_case("stdlib-mock", "stdlib_mock_helper.py" ; "stdlib_mock_helper")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test]
fn test_class_generic_before_3_13() {
    common::assert_rule_snapshot("test-class", "tests/test_class_generic.py", "py312");
}

#[test_case("up-unittest-assert-helper", "up_unittest_assert_helper.py", "py314" ; "unittest_assert_helper")]
#[test_case("up-unittest-assert-helper", "up_unittest_assert_helper.py", "py313" ; "unittest_assert_helper_py313")]
#[test_case("up-unittest-assert-helper", "up_unittest_assert_helper_base.py", "py314" ; "unittest_assert_helper_base")]
#[test_case("up-unittest-assert-helper", "up_unittest_assert_helper_comment.py", "py314" ; "unittest_assert_helper_comment")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context.py", "py311" ; "unittest_enter_context")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context.py", "py310" ; "unittest_enter_context_py310")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_used.py", "py311" ; "unittest_enter_context_used")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_comment.py", "py311" ; "unittest_enter_context_comment")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_inline.py", "py311" ; "unittest_enter_context_inline")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_not_testcase.py", "py311" ; "unittest_enter_context_not_testcase")]
fn versioned_rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
