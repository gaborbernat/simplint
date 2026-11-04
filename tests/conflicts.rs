mod common;

use std::path::Path;

use test_case::test_case;

use common::{canonical, check, check_with_stdin, project};

const SIMPLINT_314: &str = "[project]\nrequires-python = \">=3.14\"\n";
const FUTURE: &str = "lint.isort.required-imports = [\"from __future__ import annotations\"]\n";
const BOTH_IN_PYPROJECT: &str = "simplint settings: <root>/pyproject.toml; Ruff settings: <root>/pyproject.toml";

#[test_case(
    &format!("{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}"),
    "module.py",
    &format!(
        "SMP108 (future-annotations) removes `from __future__ import annotations` on Python 3.14 and newer, which \
         Ruff's I002 (missing-required-import) adds back because `lint.isort.required-imports` lists it; ignore SMP108 \
         in `[tool.simplint]`, or drop the import from `required-imports`.\n    {BOTH_IN_PYPROJECT}; first file: \
         module.py"
    ) ; "future_annotations")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"TID252\"]\nlint.flake8-tidy-imports.ban-relative-imports = \"all\"\n",
    "module.py",
    &format!(
        "SMP110 (absolute-sibling-import) makes imports from the file's own package relative, which Ruff's TID252 \
         (relative-imports) turns back into absolute ones because `lint.flake8-tidy-imports.ban-relative-imports` is \
         `\"all\"`; ignore SMP110 in `[tool.simplint]`, or set `ban-relative-imports` to `\"parents\"`.\n    \
         {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "relative_imports")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D103\"]\n",
    "module.py",
    &format!(
        "SMP300 (function-docstring) removes the docstrings of internal functions, classes and methods, which Ruff's \
         D103 (undocumented-public-function) asks for; ignore D103 in Ruff's `lint.ignore` and let SMP302 \
         (undocumented-public-api) ask for documentation of the public API, or ignore SMP300 in `[tool.simplint]`.\n    \
         {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "function_docstring")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D101\", \"D102\"]\n",
    "module.py",
    &format!(
        "SMP300 (function-docstring) removes the docstrings of internal functions, classes and methods, which Ruff's \
         D101 (undocumented-public-class), D102 (undocumented-public-method) ask for; ignore D101, D102 in Ruff's \
         `lint.ignore` and let SMP302 (undocumented-public-api) ask for documentation of the public API, or ignore \
         SMP300 in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "function_docstring_classes")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D100\"]\n",
    "tests/test_module.py",
    &format!(
        "SMP301 (test-module-docstring) removes the docstrings of test modules, which Ruff's D100 \
         (undocumented-public-module) asks for; ignore D100 for test files in Ruff's `lint.per-file-ignores`, or \
         ignore SMP301 in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: tests/test_module.py"
    ) ; "test_module_docstring")]
#[test_case(
    "[tool.simplint]\ndocstring-style = \"rest\"\n[tool.ruff]\nlint.preview = true\nlint.select = [\"DOC201\", \"DOC501\"]\n",
    "module.py",
    &format!(
        "rest-returns (SMP315) asks for the `:returns:` and `:yields:` fields of reStructuredText docstrings, which \
         Ruff's docstring-missing-returns (DOC201) reads as missing sections; ignore docstring-missing-returns in Ruff's \
         `lint.ignore`, or ignore rest-returns in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py\n  \
         - rest-missing-raises (SMP316) asks for the `:raises:` fields of reStructuredText docstrings, which Ruff's \
         docstring-missing-exception (DOC501) reads as a missing section; ignore docstring-missing-exception in Ruff's \
         `lint.ignore`, or ignore rest-missing-raises in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: \
         module.py"
    ) ; "rest_docstrings")]
fn conflict_fails_before_linting(pyproject: &str, file: &str, conflict: &str) {
    let root = project(&[("pyproject.toml", pyproject), (file, "value = 1\n")]);
    assert_eq!(conflicts(root.path(), &[]), (2, header(conflict)));
}

#[test_case(&format!("[project]\nrequires-python = \">=3.13\"\n[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}"), "module.py" ; "target_before_lazy_annotations")]
#[test_case(&format!("{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\nlint.isort.required-imports = [\"import os\"]\n"), "module.py" ; "other_required_import")]
#[test_case(&format!("{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}[tool.simplint]\nper-file-ignores = {{ \"module.py\" = [\"SMP108\"] }}\n"), "module.py" ; "simplint_rule_ignored_for_the_file")]
#[test_case("[tool.ruff]\nlint.select = [\"TID252\"]\n", "module.py" ; "parent_relative_imports_banned")]
#[test_case("[tool.ruff]\nlint.select = [\"D103\"]\n", "_private.py" ; "private_module")]
#[test_case("[tool.ruff]\nlint.select = [\"D100\"]\n", "module.py" ; "not_a_test_module")]
#[test_case("[tool.ruff]\nlint.select = [\"D100\"]\nlint.per-file-ignores = { \"tests/*\" = [\"D100\"] }\n", "tests/test_module.py" ; "ruff_rule_ignored_for_tests")]
#[test_case("[tool.ruff]\nlint.preview = true\nlint.select = [\"DOC201\"]\n", "module.py" ; "docstrings_not_rest")]
fn combination_without_conflict_lints(pyproject: &str, file: &str) {
    let root = project(&[("pyproject.toml", pyproject), (file, "value = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP300,SMP301,SMP108,SMP110,SMP315"]),
        (0, "All checks passed!\n".to_owned(), String::new())
    );
}

#[test]
fn conflicts_list_by_name_in_preview_without_configuration_files() {
    let root = project(&[("tests/test_module.py", "value = 1\n")]);
    let args = [
        "--isolated",
        "--preview",
        "--config",
        "lint.select = [\"D100\", \"TID252\"]",
        "--config",
        "lint.flake8-tidy-imports.ban-relative-imports = \"all\"",
    ];
    let place = "simplint settings: no configuration file; Ruff settings: no configuration file; first file: \
                 tests/test_module.py";
    assert_eq!(
        conflicts(root.path(), &args),
        (
            2,
            header(&format!(
                "test-module-docstring (SMP301) removes the docstrings of test modules, which Ruff's \
                 undocumented-public-module (D100) asks for; ignore undocumented-public-module for test files in \
                 Ruff's `lint.per-file-ignores`, or ignore test-module-docstring in `[tool.simplint]`.\n    {place}\n  \
                 - absolute-sibling-import (SMP110) makes imports from the file's own package relative, which Ruff's \
                 relative-imports (TID252) turns back into absolute ones because \
                 `lint.flake8-tidy-imports.ban-relative-imports` is `\"all\"`; ignore absolute-sibling-import in \
                 `[tool.simplint]`, or set `ban-relative-imports` to `\"parents\"`.\n    {place}"
            ))
        )
    );
}

#[test]
fn conflict_fails_standard_input() {
    let root =
        project(&[("pyproject.toml", &format!("{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}"))]);
    let (code, stdout, stderr) =
        check_with_stdin(root.path(), &["--stdin-filename", "module.py", "-"], "from __future__ import annotations\n");
    assert_eq!((code, stdout, stderr.contains("SMP108 (future-annotations) removes")), (2, String::new(), true));
}

fn header(conflicts: &str) -> String {
    format!(
        "simplint failed\n  Cause: simplint and Ruff rules work against each other, so fixing never leaves the code \
         clean:\n  - {conflicts}\n"
    )
}

// The exit code and the error, with the project directory as `<root>` however the settings spell it, and `/` between
// path parts on every platform.
fn conflicts(root: &Path, args: &[&str]) -> (i32, String) {
    let (code, _, stderr) = check(root, args);
    let mut error = stderr;
    for spelling in [canonical(root), root.to_path_buf()] {
        error = error.replace(&spelling.display().to_string(), "<root>");
    }
    (code, error.replace('\\', "/"))
}
