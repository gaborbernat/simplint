use test_case::test_case;

use crate::common;

#[test_case("function-docstring", "function_docstring.py" ; "function_docstring")]
#[test_case("function-docstring", "function_docstring_exported.py" ; "function_docstring_exported")]
#[test_case("function-docstring", "function_docstring_dynamic.py" ; "function_docstring_dynamic_all")]
#[test_case("undocumented-public-api", "undocumented_public_api.py" ; "undocumented_public_api")]
#[test_case("function-docstring", "undocumented_public_api.py" ; "function_docstring_public_api")]
#[test_case("undocumented-public-api", "undocumented_public_api_imported.py" ; "undocumented_public_api_imported")]
#[test_case("function-docstring", "undocumented_public_api_imported.py" ; "function_docstring_imported")]
#[test_case("undocumented-public-api", "function_docstring_dynamic.py" ; "undocumented_public_api_dynamic_all")]
#[test_case("undocumented-public-api", "tests/test_module_docstring.py" ; "undocumented_public_api_test_module")]
#[test_case("multiline-module-docstring", "multiline_module_docstring.py" ; "multiline_module_docstring")]
#[test_case(
    "multiline-module-docstring", "multiline_module_docstring_ok.py" ; "multiline_module_docstring_single_line"
)]
#[test_case("multiline-module-docstring", "tests/test_module_docstring.py" ; "multiline_module_docstring_test_module")]
#[test_case("test-module-docstring", "tests/test_module_docstring.py" ; "test_module_docstring")]
#[test_case("test-module-docstring", "tests/test_module_docstring_inline.py" ; "test_module_docstring_inline")]
#[test_case("test-module-docstring", "multiline_module_docstring.py" ; "test_module_docstring_non_test")]
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
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP004"]);
    assert_eq!(common::read(root.path(), "tests/__init__.py"), "# Licensed under the license.\n#\n");
}
