use test_case::test_case;

use crate::common;

#[test_case("test-class", "tests/test_class.py" ; "test_class")]
#[test_case("test-class", "multiline_module_docstring_ok.py" ; "test_class_regular_module")]
#[test_case("test-class", "tests/test_class_generic.py" ; "test_class_generic")]
#[test_case("parametrize-without-id", "tests/test_parametrize_without_id.py" ; "parametrize_without_id")]
#[test_case("conditional-in-test", "tests/test_conditional_in_test.py" ; "conditional_in_test")]
#[test_case("conditional-in-test", "multiline_module_docstring_ok.py" ; "conditional_in_test_regular_module")]
#[test_case("constant-assert", "constant_assert.py" ; "constant_assert")]
#[test_case("constant-assert", "constant_assert_all_rebound.py" ; "constant_assert_all_rebound")]
#[test_case("loop-in-test", "tests/test_loop_in_test.py" ; "loop_in_test")]
#[test_case("mock-assertion-typo", "mock_assertion_typo.py" ; "mock_assertion_typo")]
#[test_case("mock-assertion-typo", "mock_assertion_typo_defined.py" ; "mock_assertion_typo_defined")]
#[test_case("mock-attribute-assignment", "tests/test_mock_attribute_assignment.py" ; "mock_attribute_assignment")]
#[test_case("mock-without-spec", "tests/test_mock_without_spec.py" ; "mock_without_spec")]
#[test_case("split-composite-assert", "tests/test_split_composite_assert.py" ; "split_composite_assert")]
#[test_case("sleep-in-test", "tests/test_sleep_in_test.py" ; "sleep_in_test")]
#[test_case("sleep-in-test", "tests/test_sleep_patched.py" ; "sleep_in_test_patched")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock.py" ; "stdlib_mock")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock_mocker.py" ; "stdlib_mock_mocker")]
#[test_case("stdlib-mock", "stdlib_mock_helper.py" ; "stdlib_mock_helper")]
#[test_case("unused-fixture", "tests/test_ufix_requests.py" ; "unused_fixture")]
#[test_case("unused-fixture", "tests/conftest.py" ; "unused_fixture_conftest")]
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

// A `pytest_plugins` a test module sets may load fixtures that request a `conftest.py` fixture overriding one of
// theirs, so the conftest's fixtures are kept; pytest's own plugins request none of the run's.
#[test_case("pytest_plugins = [\"sphinx.testing.fixtures\"]", false ; "plugin_list")]
#[test_case("pytest_plugins: tuple[str, ...] = (\"sphinx.testing.fixtures\",)", false ; "annotated_tuple")]
#[test_case("pytest_plugins = \"sphinx.testing.fixtures\"", false ; "plugin_string")]
#[test_case("pytest_plugins = PLUGINS", false ; "computed")]
#[test_case("pytest_plugins = [\"pytester\", \"_pytest.pytester\"]", true ; "builtin_plugins")]
#[test_case("PLUGINS = [\"sphinx.testing.fixtures\"]", true ; "no_plugins")]
fn unused_fixture_keeps_conftest_fixtures_plugins_may_request(plugins: &str, reported: bool) {
    let conftest = "import pytest\n\n\n@pytest.fixture\ndef app_dir() -> str:\n    return \"/tmp\"\n";
    let root = common::project(&[
        ("tests/conftest.py", conftest),
        ("tests/test_app.py", &format!("PLUGINS = [\"x\"]\n{plugins}\n\n\ndef test_app() -> None:\n    assert True\n")),
    ]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP815", "--output-format", "concise"]);
    assert_eq!(stdout.contains("`app_dir`"), reported, "{stdout}");
}
