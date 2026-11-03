use test_case::test_case;

use crate::common;

#[test_case("function-docstring", "_internal/function_docstring.py" ; "function_docstring")]
#[test_case("function-docstring", "function_docstring_exported.py" ; "function_docstring_exported")]
#[test_case("function-docstring", "function_docstring_dynamic.py" ; "function_docstring_dynamic_all")]
#[test_case("function-docstring", "function_docstring_contract.py" ; "function_docstring_contract")]
#[test_case("function-docstring", "function_docstring_runtime.py" ; "function_docstring_runtime")]
#[test_case("undocumented-public-api", "undocumented_public_api.py" ; "undocumented_public_api")]
#[test_case("function-docstring", "undocumented_public_api.py" ; "function_docstring_public_api")]
#[test_case("undocumented-public-api", "undocumented_public_api_imported.py" ; "undocumented_public_api_imported")]
#[test_case("function-docstring", "undocumented_public_api_imported.py" ; "function_docstring_imported")]
#[test_case("undocumented-public-api", "function_docstring_dynamic.py" ; "undocumented_public_api_dynamic_all")]
#[test_case("undocumented-public-api", "tests/test_module_docstring.py" ; "undocumented_public_api_test_module")]
#[test_case("undocumented-public-api", "facade/_core.py" ; "undocumented_public_api_reexported_through_all")]
#[test_case("undocumented-public-api", "facade/_helpers.py" ; "undocumented_public_api_shared_internally")]
#[test_case("undocumented-public-api", "loose/_impl.py" ; "undocumented_public_api_redundant_alias")]
#[test_case("undocumented-public-api", "loose/_stars.py" ; "undocumented_public_api_star_import")]
#[test_case("undocumented-public-api", "loose/_extra.py" ; "undocumented_public_api_star_import_without_all")]
#[test_case("undocumented-public-api", "loose/_deep.py" ; "undocumented_public_api_chain")]
#[test_case("undocumented-public-api", "switched.py" ; "undocumented_public_api_top_level_blocks")]
#[test_case("function-docstring", "switched.py" ; "function_docstring_top_level_blocks")]
#[test_case("function-docstring", "facade/_core.py" ; "function_docstring_reexported_through_all")]
#[test_case("function-docstring", "facade/_helpers.py" ; "function_docstring_shared_internally")]
#[test_case("test-module-docstring", "tests/test_module_docstring.py" ; "test_module_docstring")]
#[test_case("test-module-docstring", "tests/test_module_docstring_inline.py" ; "test_module_docstring_inline")]
#[test_case("test-module-docstring", "multiline_module_docstring.py" ; "test_module_docstring_non_test")]
#[test_case("rest-field-syntax", "rest_field_syntax.py" ; "rest_field_syntax")]
#[test_case("rest-unknown-param", "rest_unknown_param.py" ; "rest_unknown_param")]
#[test_case("rest-missing-param", "rest_missing_param.py" ; "rest_missing_param")]
#[test_case("rest-param-order", "rest_param_order.py" ; "rest_param_order")]
#[test_case("rest-type-field", "rest_type_field.py" ; "rest_type_field")]
#[test_case("rest-returns", "rest_returns.py" ; "rest_returns")]
#[test_case("rest-missing-raises", "rest_missing_raises.py" ; "rest_missing_raises")]
#[test_case("rest-missing-attribute", "rest_missing_attribute.py" ; "rest_missing_attribute")]
#[test_case("rest-unknown-attribute", "rest_unknown_attribute.py" ; "rest_unknown_attribute")]
#[test_case("rest-duplicate-param", "rest_duplicate_param.py" ; "rest_duplicate_param")]
#[test_case("rest-attribute-kind", "rest_attribute_kind.py" ; "rest_attribute_kind")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

// `ruff format` keeps blank lines after a module of comments only, and Ruff reports blank lines ending the file.
#[test]
fn test_module_docstring_removal_leaves_no_blank_lines_ending_the_file() {
    let root = common::project(&[
        ("pyproject.toml", ""),
        ("tests/__init__.py", "# Licensed under the license.\n#\n\n\n\"\"\"Tests.\n\nMore.\n\"\"\"\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP301"]);
    assert_eq!(common::read(root.path(), "tests/__init__.py"), "# Licensed under the license.\n#\n");
}

// The comment wraps to the shorter of Ruff's line and doc line limits at the indentation `ruff format` gives the body,
// so a body indented by two spaces wraps the same.
#[test_case("    " ; "formatted")]
#[test_case("  " ; "two_spaces")]
fn function_docstring_comment_wraps_to_ruff_limits(indent: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[tool.ruff]\nline-length = 40\n[tool.ruff.lint]\nselect = [\"E501\", \"W505\"]\n\
             [tool.ruff.lint.pycodestyle]\nmax-doc-length = 30\n",
        ),
        (
            "module.py",
            &format!(
                "def _run() -> None:\n{indent}\"\"\"Retry once, as the first call after a restart times out.\"\"\"\n{indent}print()\n"
            ),
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP300"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        format!(
            "def _run() -> None:\n{indent}# Retry once, as the first\n{indent}# call after a restart\n{indent}# times out.\n{indent}print()\n"
        )
    );
}

// Code in a docstring reads as commented-out code once it is a comment, which Ruff's ERA001 reports.
#[test]
fn function_docstring_comment_is_withdrawn_when_ruff_reports_it() {
    let source =
        "def _run() -> None:\n    \"\"\"Call it as shown below.\n\n    result = run(config)\n    \"\"\"\n    print()\n";
    let root =
        common::project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"ERA001\"]\n"), ("module.py", source)]);
    let stdout = common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP300"]).1;
    assert_eq!(
        (stdout, common::read(root.path(), "module.py")),
        (
            "module.py:2:5: SMP300 Function docstring on internal code; let the name, signature and types document it\n\
             Found 1 error.\n"
                .to_owned(),
            source.to_owned()
        )
    );
}

#[test]
fn function_docstring_comment_keeps_the_line_endings() {
    let source = "def _run() -> None: \"\"\"Retry once, as the first call\r\n    after a restart times out.\"\"\"\r\n";
    let root = common::project(&[("pyproject.toml", ""), ("module.py", source)]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP300"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        "def _run() -> None:\r\n    # Retry once, as the first call\r\n    # after a restart times out.\r\n    ...\r\n"
    );
}

// Fixes in one pass that touch wait for the next, so removing the statement after the docstring does not leave a body
// of comments only.
#[test]
fn function_docstring_comment_waits_for_a_removed_statement() {
    let root = common::project(&[
        ("pyproject.toml", ""),
        (
            "test_module.py",
            "def test_probe() -> None:\n    \"\"\"Check that adding one more test keeps the run stable.\"\"\"\n    assert True\n",
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP300,SMP205"]);
    assert_eq!(
        common::read(root.path(), "test_module.py"),
        "def test_probe() -> None:\n    # Check that adding one more test keeps the run stable.\n    pass\n"
    );
}

// Without `docstring-style`, the field lists of a project's docstrings are not known to be reStructuredText.
#[test]
fn rest_rules_need_the_docstring_style() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[build-system]\nrequires = []\n"),
        (
            "demo.py",
            "def run(path: str) -> str:\n    \"\"\"Run.\n\n    :parm path: Where.\n    \"\"\"\n    return path\n",
        ),
    ]);
    assert_eq!(common::check(root.path(), &["--select", "SMP31"]).1, "All checks passed!\n");
}
