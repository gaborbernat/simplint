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
#[test_case("mock-attribute-assignment", "tests/test_mock_attribute_comment.py", None ; "mock_attribute_assignment_comment")]
#[test_case("mock-attribute-assignment", "tests/test_mock_reserved_attribute.py", None ; "mock_attribute_assignment_reserved")]
#[test_case("mock-without-spec", "tests/test_mock_without_spec.py", None ; "mock_without_spec")]
#[test_case("split-composite-assert", "tests/test_split_composite_assert.py", None ; "split_composite_assert")]
#[test_case("sleep-in-test", "tests/test_sleep_in_test.py", None ; "sleep_in_test")]
#[test_case("sleep-in-test", "tests/test_sleep_patched.py", None ; "sleep_in_test_patched")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock.py", None ; "stdlib_mock")]
#[test_case("stdlib-mock", "tests/test_stdlib_mock_mocker.py", None ; "stdlib_mock_mocker")]
#[test_case("stdlib-mock", "stdlib_mock_helper.py", None ; "stdlib_mock_helper")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context.py", Some("py311") ; "unittest_enter_context")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context.py", Some("py310") ; "unittest_enter_context_py310")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_used.py", Some("py311") ; "unittest_enter_context_used")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_comment.py", Some("py311") ; "unittest_enter_context_comment")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_inline.py", Some("py311") ; "unittest_enter_context_inline")]
#[test_case("test-class", "tests/test_class_generic.py", Some("py312") ; "test_class_generic_before_3_13")]
#[test_case("up-unittest-enter-context", "up_unittest_enter_context_not_testcase.py", Some("py311") ; "unittest_enter_context_not_testcase")]
#[test_case("unused-fixture", "tests/test_ufix_requests.py", None ; "unused_fixture")]
#[test_case("unused-fixture", "tests/conftest.py", None ; "unused_fixture_conftest")]
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

// Ruff's PGH005 reports a non-existent mock method in an assert, and on a line of its own only with preview on. SMP814
// leaves those to Ruff while PGH005 runs, and reports them otherwise.
#[test_case("[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "mock.not_called()", true ; "selected_call")]
#[test_case("[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "mock.not_called", true ; "selected_attribute")]
#[test_case("[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "assert mock.not_called()", false ; "selected_assert")]
#[test_case("[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "assert not mock.not_called()", true ; "selected_assert_not")]
#[test_case("[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "assert check(mock.not_called)", true ; "selected_assert_call_argument")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "mock.not_called()", false ; "selected_preview_call")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "mock.not_called", false ; "selected_preview_attribute")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "assert mock.not_called()", false ; "selected_preview_assert")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"PGH005\"]\n", "assert not mock.not_called()", true ; "selected_preview_assert_not")]
#[test_case("[tool.ruff.lint]\nselect = []\n", "mock.not_called()", true ; "unselected_call")]
#[test_case("[tool.ruff.lint]\nselect = []\n", "assert mock.not_called()", true ; "unselected_assert")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = []\n", "mock.not_called()", true ; "unselected_preview_call")]
#[test_case("[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = []\n", "assert mock.not_called()", true ; "unselected_preview_assert")]
#[test_case(
    "[tool.ruff.lint]\nselect = [\"PGH005\"]\nper-file-ignores = { \"test_save.py\" = [\"PGH005\"] }\n",
    "assert mock.not_called()",
    false
    ; "selected_but_ignored_for_the_file"
)]
fn mock_assertion_typo_follows_ruff_pgh005(pyproject: &str, statement: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", pyproject),
        (
            "test_save.py",
            &format!(
                "from unittest.mock import MagicMock\n\n\ndef test_save(mock: MagicMock) -> None:\n    {statement}\n"
            ),
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP814"]);
    assert_eq!(stdout.contains("`not_called`"), reported, "{stdout}");
}

// Ruff's assert-tuple reports a non-empty tuple in an assert, which SMP804 leaves to it while F631 runs.
#[test_case("def check() -> None:\n    assert (True, 1, 2)\n", "F631", "Assertion on a constant always passes" ; "assert_tuple")]
#[test_case("def check() -> None:\n    assert (True, \"message\")\n", "F631", "Assertion on a constant always passes" ; "assert_tuple_with_fix")]
fn constant_assert_follows_ruff_assert_tuple(source: &str, code: &str, fragment: &str) {
    let case = common::RuffOverlap { settings: "", select: "SMP804", ruff_code: code, files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, fragment), [true, false]);
}

// Ruff's PLW0129 reports an assert on a string, bytes or f-string literal, which SMP804 leaves to it. A concatenation
// is not a literal, so SMP804 keeps it.
#[test_case("PLW0129", "assert \"ready\"", false ; "deletable_string")]
#[test_case("PLW0129", "check(); assert \"ready\"", false ; "string_sharing_its_line")]
#[test_case("PLW0129", "check(); assert \"ready\", \"message\"", false ; "string_with_message")]
#[test_case("PLW0129", "check(); assert \"ready\" \"set\"", false ; "implicit_concatenation")]
#[test_case("PLW0129", "check(); assert b\"ready\"", false ; "bytes")]
#[test_case("PLW0129", "assert f\"ready {check()}\"", false ; "fstring_with_effect")]
#[test_case("PLW0129", "assert \"a\" + \"b\"", true ; "concatenation")]
#[test_case("PLW0129", "assert True", true ; "boolean")]
fn constant_assert_follows_ruff_assert_on_string_literal(code: &str, statement: &str, reported_when_selected: bool) {
    let module = format!("def check() -> None:\n    {statement}\n");
    let case =
        common::RuffOverlap { settings: "", select: "SMP804", ruff_code: code, files: &[("module.py", &module)] };
    assert_eq!(
        common::reported_without_and_with_ruff(&case, "Assertion on a constant always passes"),
        [true, reported_when_selected]
    );
}

// Ruff's ASYNC110 reports a `while` whose whole body awaits `asyncio.sleep`, so SMP808 leaves that sleep to Ruff
// whether or not the project selects it. A body with another statement is not one ASYNC110 reports, so the rule keeps it.
#[test_case("while not done():\n        await asyncio.sleep(1)", 0 ; "sole_await_left_to_ruff")]
#[test_case("while not done():\n        await asyncio.sleep(1)\n        done()", 1 ; "await_with_another_statement")]
fn sleep_in_test_leaves_busy_waits_to_async110(body: &str, reports: usize) {
    let source = format!("import asyncio\n\n\nasync def test_busy(done):\n    {body}\n");
    let root = common::project(&[("test_busy.py", &source)]);
    common::assert_report_count(root.path(), "SMP808", reports);
}
