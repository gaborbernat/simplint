use test_case::test_case;

use crate::common;

#[test_case("test-class", "tests/test_class.py", None ; "test_class")]
#[test_case("test-class", "multiline_module_docstring_ok.py", None ; "test_class_regular_module")]
#[test_case("test-class", "tests/test_class_generic.py", None ; "test_class_generic")]
#[test_case("test-class", "tests/test_class_indent.py", None ; "test_class_indent")]
#[test_case("test-class", "tests/test_class_shadow.py", None ; "test_class_shadow")]
#[test_case("test-class", "tests/test_class_flattened_collision.py", None ; "test_class_flattened_collision")]
#[test_case("parametrize-without-id", "tests/test_parametrize_without_id.py", None ; "parametrize_without_id")]
#[test_case("parametrize-without-id", "tests/test_parametrize_scalar_ids.py", None ; "parametrize_scalar_ids")]
#[test_case("parametrize-without-id", "tests/test_parametrize_starred.py", None ; "parametrize_starred")]
#[test_case("parametrize-without-id", "tests/test_parametrize_spread.py", None ; "parametrize_spread")]
#[test_case("parametrize-without-id", "tests/test_parametrize_names.py", None ; "parametrize_names")]
#[test_case("parametrize-without-id", "tests/test_parametrize_bindings.py", None ; "parametrize_bindings")]
#[test_case("parametrize-without-id", "tests/test_parametrize_helpers.py", None ; "parametrize_helpers")]
#[test_case("parametrize-without-id", "tests/test_parametrize_ids_list.py", None ; "parametrize_ids_list")]
#[test_case("conditional-in-test", "tests/test_conditional_in_test.py", None ; "conditional_in_test")]
#[test_case("conditional-in-test", "multiline_module_docstring_ok.py", None ; "conditional_in_test_regular_module")]
#[test_case("constant-assert", "constant_assert.py", None ; "constant_assert")]
#[test_case("constant-assert", "constant_assert_all_rebound.py", None ; "constant_assert_all_rebound")]
#[test_case("constant-assert", "constant_assert_ruff_overlap.py", None ; "constant_assert_ruff_overlap")]
#[test_case("loop-in-test", "tests/test_loop_in_test.py", None ; "loop_in_test")]
#[test_case("loop-in-test", "tests/test_loop_in_test_shared.py", None ; "loop_in_test_shared_name")]
#[test_case("mock-assertion-typo", "mock_assertion_typo.py", None ; "mock_assertion_typo")]
#[test_case("mock-assertion-typo", "mock_assertion_typo_defined.py", None ; "mock_assertion_typo_defined")]
#[test_case("mock-assertion-typo", "mock_assertion_typo_parenthesized.py", None ; "mock_assertion_typo_parenthesized")]
#[test_case("mock-assertion-typo", "mock_assertion_typo_comment.py", None ; "mock_assertion_typo_comment")]
#[test_case("mock-attribute-assignment", "tests/test_mock_attribute_assignment.py", None ; "mock_attribute_assignment")]
#[test_case(
    "mock-attribute-assignment", "tests/test_mock_attribute_comment.py", None ; "mock_attribute_assignment_comment"
)]
#[test_case(
    "mock-attribute-assignment", "tests/test_mock_reserved_attribute.py", None ; "mock_attribute_assignment_reserved"
)]
#[test_case("mock-without-spec", "tests/test_mock_without_spec.py", None ; "mock_without_spec")]
#[test_case("mock-without-spec", "tests/test_mock_spec_members.py", None ; "mock_without_spec_members")]
#[test_case("split-composite-assert", "tests/test_split_composite_assert.py", None ; "split_composite_assert")]
#[test_case("sleep-in-test", "tests/test_sleep_in_test.py", None ; "sleep_in_test")]
#[test_case("sleep-in-test", "tests/test_sleep_patched.py", None ; "sleep_in_test_patched")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock.py", None ; "stdlib_mock")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock_mocker.py", None ; "stdlib_mock_mocker")]
#[test_case("stdlib-mock", "stdlib_mock_helper.py", None ; "stdlib_mock_helper")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context.py", Some("py311") ; "unittest_enter_context")]
#[test_case(
    "up-unittest-enter-context", "up_unittest_enter_context.py", Some("py310") ; "unittest_enter_context_py310"
)]
#[test_case(
    "up-unittest-enter-context", "up_unittest_enter_context_used.py", Some("py311") ; "unittest_enter_context_used"
)]
#[test_case(
    "up-unittest-enter-context",
    "up_unittest_enter_context_comment.py",
    Some("py311") ;
    "unittest_enter_context_comment"
)]
#[test_case(
    "up-unittest-enter-context", "up_unittest_enter_context_inline.py", Some("py311") ; "unittest_enter_context_inline"
)]
#[test_case("test-class", "tests/test_class_generic.py", Some("py312") ; "test_class_generic_before_3_13")]
#[test_case(
    "up-unittest-enter-context",
    "up_unittest_enter_context_not_testcase.py",
    Some("py311") ;
    "unittest_enter_context_not_testcase"
)]
#[test_case("unused-fixture", "tests/test_ufix_requests.py", None ; "unused_fixture")]
#[test_case("unused-fixture", "tests/conftest.py", None ; "unused_fixture_conftest")]
#[test_case("test-process-state", "tests/test_process_state.py", None ; "test_process_state")]
#[test_case("test-process-state", "tests/test_process_state_teardown_module.py", None ; "test_process_state_teardown")]
#[test_case("test-process-state", "tests/test_process_state_pytestmark.py", None ; "test_process_state_pytestmark")]
#[test_case("test-return-value", "tests/test_return_value.py", None ; "test_return_value")]
#[test_case("test-without-assertion", "tests/test_without_assertion.py", None ; "test_without_assertion")]
#[test_case("up-pytest-deprecated-api", "tests/test_up_pytest_deprecated_api.py", None ; "up_pytest_deprecated_api")]
#[test_case(
    "up-pytest-deprecated-api",
    "tests/test_up_pytest_deprecated_api_called_setup.py",
    None ;
    "up_pytest_deprecated_api_called_setup"
)]
#[test_case(
    "up-pytest-deprecated-api", "tests/test_up_pytest_deprecated_api_nose_module.py", None ; "up_pytest_deprecated_api_nose_module"
)]
#[test_case(
    "up-pytest-deprecated-api", "tests/test_up_pytest_deprecated_api_nose_class.py", None ; "up_pytest_deprecated_api_nose_class"
)]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

#[test_case("create_autospec", 0 ; "autospec_only")]
#[test_case("create_autospec, patch", 1 ; "also_patching")]
fn stdlib_mock_allows_create_autospec_only(imports: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"example\"\nversion = \"1\"\ndependencies = [\"pytest-mock\"]\n"),
        (
            "tests/test_mock.py",
            &format!("from unittest.mock import {imports}\n\n\ndef test_mock() -> None:\n    assert True\n"),
        ),
    ]);
    common::assert_report_count(root.path(), "SMP809", reports);
}

// Ruff's TID251 reports an import that `banned-api` bans by its module, a parent of it, or a member's full name. SMP809
// leaves those to it, whether or not the project selects TID251.
#[test_case("unittest.mock", "from unittest.mock import MagicMock", 0 ; "module_keyed")]
#[test_case("unittest", "from unittest.mock import MagicMock", 0 ; "parent_keyed_from_module")]
#[test_case("unittest", "import unittest.mock", 0 ; "parent_keyed_import")]
#[test_case("unittest", "from unittest import mock", 0 ; "parent_keyed_member")]
#[test_case("unittest.mock.patch", "from unittest.mock import patch", 0 ; "all_members_keyed")]
#[test_case("unittest.mock.patch", "from unittest.mock import patch, MagicMock", 1 ; "member_keyed")]
#[test_case("unittest.mock.patch", "from unittest.mock import MagicMock", 1 ; "other_member_keyed")]
#[test_case("unittest.mockery", "from unittest.mock import MagicMock", 1 ; "sibling_keyed")]
#[test_case("os.path", "from unittest.mock import MagicMock", 1 ; "unrelated_keyed")]
fn stdlib_mock_follows_ruff_banned_api(key: &str, imports: &str, reports: usize) {
    let root = common::project(&[
        (
            "pyproject.toml",
            &format!(
                "[project]\nname = \"example\"\nversion = \"1\"\ndependencies = [\"pytest-mock\"]\n\n\
                 [tool.ruff.lint.flake8-tidy-imports.banned-api]\n\"{key}\" = {{ msg = \"use mocker\" }}\n"
            ),
        ),
        ("tests/test_mock.py", &format!("{imports}\n\n\ndef test_mock() -> None:\n    assert True\n")),
    ]);
    common::assert_report_count(root.path(), "SMP809", reports);
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
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP815"]);
    assert_eq!(stdout.contains("`app_dir`"), reported, "{stdout}");
}

// Ruff's PGH005 reports a non-existent mock method in the test of an assert and in a statement of its own, and PGH005
// offers no fix. SMP814 leaves a name it has no fix for to Ruff, and reports the names it can fix, whether or not the
// project selects PGH005.
#[test_case("mock.not_called()", true ; "call_statement")]
#[test_case("mock.not_called", false ; "attribute_statement")]
#[test_case("assert mock.not_called()", true ; "assert_call")]
#[test_case("assert mock.not_called", false ; "assert_attribute")]
#[test_case("assert not mock.not_called()", true ; "assert_not")]
#[test_case("assert check(mock.not_called)", true ; "assert_call_argument")]
#[test_case("assert not mock.called_once", true ; "assert_not_attribute")]
#[test_case("assert (mock.not_called()  # the writer\n    )", false ; "comment_in_the_fix")]
#[test_case("assert mock.called_once_with()", true ; "assert_call_fixed")]
#[test_case("mock.called_once_with()", true ; "call_statement_fixed")]
#[test_case("assert mock.called_once", false ; "assert_attribute_without_fix")]
#[test_case("mock.called_once", false ; "attribute_statement_without_fix")]
fn mock_assertion_typo_leaves_ruff_names_without_a_fix(statement: &str, reported: bool) {
    let root = common::project(&[(
        "test_save.py",
        &format!("from unittest.mock import MagicMock\n\n\ndef test_save(mock: MagicMock) -> None:\n    {statement}\n"),
    )]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP814"]);
    assert_eq!(stdout.contains("is not a mock assertion"), reported, "{stdout}");
}

// Ruff's F631 and PLW0129 report a non-empty tuple and a string, bytes or f-string literal, so SMP804 leaves a case
// without a fix of its own to them, whether or not the project selects them, and reports the cases it can fix.
#[test_case("def check() -> None:\n    assert (True, 1, 2)\n", false ; "assert_tuple")]
#[test_case("def check() -> None:\n    assert (1, 2, 3)\n", false ; "assert_tuple_without_fix")]
#[test_case("def check() -> None:\n    assert (True, \"message\")\n", true ; "assert_tuple_with_fix")]
#[test_case("def check() -> None:\n    assert \"ready\"\n", true ; "deletable_string")]
#[test_case("def check() -> None:\n    assert b\"ready\"\n", true ; "deletable_bytes")]
#[test_case("def check() -> None:\n    assert f\"ready\"\n", true ; "deletable_fstring")]
#[test_case("def check() -> None:\n    assert \"ready\", \"message\"\n", true ; "deletable_string_with_message")]
#[test_case("def check() -> None:\n    check(); assert \"ready\"\n", false ; "string_sharing_its_line")]
#[test_case("def check() -> None:\n    check(); assert \"ready\", \"message\"\n", false ; "string_with_message")]
#[test_case("def check() -> None:\n    check(); assert \"ready\" \"set\"\n", false ; "implicit_concatenation")]
#[test_case("def check() -> None:\n    check(); assert b\"ready\"\n", false ; "bytes")]
#[test_case("def check() -> None:\n    assert f\"ready {check()}\"\n", false ; "fstring_with_effect")]
#[test_case("def check() -> None:\n    assert \"a\" + \"b\"\n", true ; "concatenation")]
#[test_case("def check() -> None:\n    assert True\n", true ; "boolean")]
fn constant_assert_leaves_ruff_cases_without_a_fix(source: &str, reported: bool) {
    let root = common::project(&[("module.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP804"]);
    assert_eq!(stdout.contains("Assertion on a constant always passes"), reported, "{stdout}");
}

// Ruff's ASYNC110 reports a `while` whose whole body awaits `asyncio.sleep`, so SMP808 leaves that sleep to Ruff
// whether or not the project selects it. A body with another statement is not one ASYNC110 reports, so the rule keeps
// it.
#[test_case("while not done():\n        await asyncio.sleep(1)", 0 ; "sole_await_left_to_ruff")]
#[test_case("while not done():\n        await asyncio.sleep(1)\n        done()", 1 ; "await_with_another_statement")]
fn sleep_in_test_leaves_busy_waits_to_async110(body: &str, reports: usize) {
    let source = format!("import asyncio\n\n\nasync def test_busy(done):\n    {body}\n");
    let root = common::project(&[("test_busy.py", &source)]);
    common::assert_report_count(root.path(), "SMP808", reports);
}

const MARKED: &str = "import pytest\n\n\n@pytest.mark.slow\n@pytest.mark.skip\ndef test_marked() -> None:\n    assert True\n";

// pytest knows the marks the configuration registers, its own, and the ones common plugins register; without a
// configuration the registered marks are unknown.
#[test_case("[tool.pytest.ini_options]\nmarkers = [\"slow: slow tests\"]\n", MARKED, 0 ; "registered")]
#[test_case("[tool.pytest.ini_options]\nmarkers = [\"fast\"]\n", MARKED, 1 ; "unregistered")]
#[test_case("[tool.pytest]\nmarkers = [\"fast\"]\n", MARKED, 1 ; "native_table")]
#[test_case("", MARKED, 0 ; "no_configuration")]
#[test_case(
    "[tool.pytest.ini_options]\nmarkers = [\"fast\"]\n",
    "import pytest\n\n\n@pytest.mark.timeout(5)\ndef test_marked() -> None:\n    assert True\n",
    0 ;
    "plugin_mark"
)]
#[test_case(
    "[tool.pytest.ini_options]\nmarkers = [\"fast\"]\n",
    "import pytest\n\nSLOW = pytest.mark.slow\n\n\n@pytest.mark.slow\ndef test_marked() -> None:\n    assert True\n",
    0 ;
    "bound_mark"
)]
fn unregistered_marker_reads_pytest_configuration(configuration: &str, test: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[project]\nname = \"example\"\nversion = \"1\"\n\n{configuration}")),
        ("tests/test_marks.py", test),
    ]);
    common::assert_report_count(root.path(), "SMP820", reports);
}

#[test_case("pytest.ini", "[pytest]\nmarkers =\n    slow: slow tests\n" ; "pytest_ini")]
#[test_case(".pytest.ini", "[pytest]\nmarkers =\n    slow\n" ; "hidden_pytest_ini")]
#[test_case("tox.ini", "[pytest]\nmarkers =\n    slow: slow tests\n" ; "tox_ini")]
#[test_case("setup.cfg", "[tool:pytest]\nmarkers =\n    slow: slow tests\n" ; "setup_cfg")]
fn unregistered_marker_reads_ini_files(file: &str, configuration: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"example\"\nversion = \"1\"\n"),
        (file, configuration),
        ("tests/test_marks.py", "import pytest\n\n\n@pytest.mark.slow\n@pytest.mark.fast\ndef test_marked() -> None:\n    assert True\n"),
    ]);
    common::assert_report_count(root.path(), "SMP820", 1);
}

const CHECKED: &str = "def test_value() -> None:\n    assert True\n";

// pytest collects `test_*.py` and `*_test.py` unless `python_files` says otherwise; a module it skips on purpose is
// left alone.
#[test_case("", "tests/check_value.py", CHECKED, 1 ; "default_patterns")]
#[test_case("", "tests/value_test.py", CHECKED, 0 ; "suffix_pattern")]
#[test_case("[tool.pytest.ini_options]\npython_files = [\"check_*.py\", \"test_*.py\"]\n", "tests/check_value.py", CHECKED, 0 ; "configured")]
#[test_case("[tool.pytest.ini_options]\npython_files = \"check_*.py\"\n", "tests/test_value.py", CHECKED, 0 ; "no_sibling_collected")]
#[test_case(
    "",
    "tests/check_value.py",
    "def test_value() -> None:\n    assert True\n\n\nif __name__ == \"__main__\":\n    test_value()\n",
    0 ;
    "main_guard"
)]
#[test_case("", "tests/check_value.py", "\"\"\"Run with `pytest tests/check_value.py`.\"\"\"\n\n\ndef test_value() -> None:\n    assert True\n", 0 ; "docstring_names_file")]
#[test_case("", "tests/check_value.py", "def helper() -> None:\n    assert True\n", 0 ; "no_tests")]
#[test_case("", "tests/conftest.py", CHECKED, 0 ; "conftest")]
fn uncollected_test_module_follows_python_files(configuration: &str, module: &str, source: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[project]\nname = \"example\"\nversion = \"1\"\n\n{configuration}")),
        ("tests/test_other.py", CHECKED),
        (module, source),
    ]);
    common::assert_report_count(root.path(), "SMP821", reports);
}

#[test_case("tests/test_runner.py", "from check_value import test_value\n" ; "imported_test")]
#[test_case("tests/test_runner.py", "def test_runs(pytester) -> None:\n    pytester.copy_example(\"check_value.py\")\n" ; "named_as_data")]
fn uncollected_test_module_used_elsewhere(other: &str, source: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"example\"\nversion = \"1\"\n"),
        (other, source),
        ("tests/check_value.py", CHECKED),
    ]);
    common::assert_report_count(root.path(), "SMP821", 0);
}
