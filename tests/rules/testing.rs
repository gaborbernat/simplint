use test_case::test_case;

use crate::common;

#[test_case("test-class", "tests/test_class.py" ; "test_class")]
#[test_case("test-class", "multiline_module_docstring_ok.py" ; "test_class_regular_module")]
#[test_case("test-class", "tests/test_class_generic.py" ; "test_class_generic")]
#[test_case("parametrize-without-id", "tests/test_parametrize_without_id.py" ; "parametrize_without_id")]
#[test_case("parametrize-without-id", "tests/test_parametrize_scalar_ids.py" ; "parametrize_scalar_ids")]
#[test_case("parametrize-without-id", "tests/test_parametrize_starred.py" ; "parametrize_starred")]
#[test_case("parametrize-without-id", "tests/test_parametrize_spread.py" ; "parametrize_spread")]
#[test_case("parametrize-without-id", "tests/test_parametrize_names.py" ; "parametrize_names")]
#[test_case("parametrize-without-id", "tests/test_parametrize_bindings.py" ; "parametrize_bindings")]
#[test_case("parametrize-without-id", "tests/test_parametrize_helpers.py" ; "parametrize_helpers")]
#[test_case("conditional-in-test", "tests/test_conditional_in_test.py" ; "conditional_in_test")]
#[test_case("conditional-in-test", "multiline_module_docstring_ok.py" ; "conditional_in_test_regular_module")]
#[test_case("constant-assert", "constant_assert.py" ; "constant_assert")]
#[test_case("constant-assert", "constant_assert_all_rebound.py" ; "constant_assert_all_rebound")]
#[test_case("constant-assert", "constant_assert_ruff_overlap.py" ; "constant_assert_ruff_overlap")]
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
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP809"]);
    assert_eq!(stdout.matches("SMP809").count(), reports, "{stdout}");
}

#[test]
fn test_class_generic_before_3_13() {
    common::assert_rule_snapshot("test-class", "tests/test_class_generic.py", "py312");
}

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
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP814", "--output-format", "concise"]);
    assert_eq!(stdout.contains("`not_called`"), reported, "{stdout}");
}

// Ruff's assert-tuple reports a non-empty tuple in an assert, which SMP804 leaves to it while F631 runs. A tuple of
// three elements has no fix for SMP804 to offer.
#[test_case("def check() -> None:\n    assert (True, 1, 2)\n", "F631", "Assertion on a constant always passes" ; "assert_tuple")]
fn constant_assert_follows_ruff_assert_tuple(source: &str, code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP804"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}

// Ruff's PLW0129 reports an assert on a string, bytes or f-string literal without a fix, which SMP804 leaves to it while
// the project's Ruff configuration selects it, unless SMP804 can delete the assertion. A concatenation is not a literal,
// so SMP804 keeps it.
#[test_case("PLW0129", "assert \"ready\"", true ; "deletable_string")]
#[test_case("PLW0129", "check(); assert \"ready\"", false ; "string_sharing_its_line")]
#[test_case("PLW0129", "check(); assert \"ready\", \"message\"", false ; "string_with_message")]
#[test_case("PLW0129", "check(); assert \"ready\" \"set\"", false ; "implicit_concatenation")]
#[test_case("PLW0129", "check(); assert b\"ready\"", false ; "bytes")]
#[test_case("PLW0129", "assert f\"ready {check()}\"", false ; "fstring_with_effect")]
#[test_case("PLW0129", "assert \"a\" + \"b\"", true ; "concatenation")]
#[test_case("PLW0129", "assert True", true ; "boolean")]
fn constant_assert_follows_ruff_assert_on_string_literal(code: &str, statement: &str, reported_when_selected: bool) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", &format!("def check() -> None:\n    {statement}\n")),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP804"]).1;
        assert_eq!(
            stdout.contains("Assertion on a constant always passes"),
            !selected || reported_when_selected,
            "{stdout}"
        );
    }
}

// Ruff's ASYNC110 reports a `while` whose whole body awaits `asyncio.sleep`, which SMP808 leaves to it while the
// project's Ruff configuration selects it.
#[test_case("while not done():\n        await asyncio.sleep(1)", "ASYNC110", 0 ; "sole_await_left_to_ruff")]
#[test_case("while not done():\n        await asyncio.sleep(1)", "E501", 1 ; "sole_await_reported_without_async110")]
#[test_case("while not done():\n        await asyncio.sleep(1)\n        done()", "ASYNC110", 1 ; "await_with_another_statement")]
fn sleep_in_test_leaves_busy_waits_to_async110(body: &str, code: &str, reports: usize) {
    let source = format!("import asyncio\n\n\nasync def test_busy(done):\n    {body}\n");
    let settings = format!("[tool.ruff.lint]\nselect = [\"{code}\"]\n");
    let root = common::project(&[("pyproject.toml", &settings), ("test_busy.py", &source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP808"]);
    assert_eq!(stdout.matches("SMP808").count(), reports, "{stdout}");
}
