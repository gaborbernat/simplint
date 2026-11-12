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
        "SMP301 (future-annotations) and Ruff's I002 (missing-required-import) undo each other's fixes.\n    Why: \
         SMP301 removes `from __future__ import annotations` on Python 3.14+; I002 adds it back because \
         `lint.isort.required-imports` lists it.\n    Fix: drop the import from `required-imports`, or ignore SMP301 \
         in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "future_annotations")]
#[test_case(
    "[project]\nrequires-python = \">=3.15\"\n[tool.ruff]\nlint.preview = true\nlint.select = [\"PLW1514\"]\n",
    "module.py",
    &format!(
        "up-utf8-encoding-default (SMP521) and Ruff's unspecified-encoding (PLW1514) undo each other's fixes.\n    \
         Why: up-utf8-encoding-default removes `encoding=\"utf-8\"` on Python 3.15+, where UTF-8 is the default; \
         unspecified-encoding adds it back.\n    Fix: ignore unspecified-encoding in Ruff's `lint.ignore`, or ignore \
         up-utf8-encoding-default in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "utf8_encoding_default")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"TID252\"]\nlint.flake8-tidy-imports.ban-relative-imports = \"all\"\n",
    "module.py",
    &format!(
        "SMP302 (absolute-sibling-import) and Ruff's TID252 (relative-imports) undo each other's fixes.\n    Why: \
         SMP302 makes imports from the file's own package relative; TID252 turns them back into absolute ones because \
         `lint.flake8-tidy-imports.ban-relative-imports` is `\"all\"`.\n    Fix: set `ban-relative-imports` to \
         `\"parents\"`, or ignore SMP302 in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "relative_imports")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D103\"]\n",
    "module.py",
    &format!(
        "SMP901 (function-docstring) and Ruff's D103 (undocumented-public-function) work against each other.\n    \
         Why: SMP901 removes the docstrings of internal functions, classes and methods; D103 asks for them.\n    Fix: \
         ignore D103 in Ruff's `lint.ignore` and let SMP903 (undocumented-public-api) ask for documentation of the \
         public API, or ignore SMP901 in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "function_docstring")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D101\", \"D102\"]\n",
    "module.py",
    &format!(
        "SMP901 (function-docstring) and Ruff's D101 (undocumented-public-class), D102 (undocumented-public-method) \
         work against each other.\n    Why: SMP901 removes the docstrings of internal functions, classes and methods; \
         D101, D102 ask for them.\n    Fix: ignore D101, D102 in Ruff's `lint.ignore` and let SMP903 \
         (undocumented-public-api) ask for documentation of the public API, or ignore SMP901 \
         in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "function_docstring_classes")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D100\"]\n",
    "tests/test_module.py",
    &format!(
        "SMP902 (test-module-docstring) and Ruff's D100 (undocumented-public-module) work against each other.\n    \
         Why: SMP902 removes the docstrings of test modules; D100 asks for them.\n    Fix: ignore D100 for test files \
         in Ruff's `lint.per-file-ignores`, or ignore SMP902 in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first \
         file: tests/test_module.py"
    ) ; "test_module_docstring")]
#[test_case(
    "[tool.simplint]\ndocstring-style = \"rst\"\n[tool.ruff]\nlint.preview = true\n\
     lint.select = [\"DOC201\", \"DOC501\"]\n",
    "module.py",
    &format!(
        "rst-returns (SMP923) and Ruff's docstring-missing-returns (DOC201) work against each other.\n    Why: \
         rst-returns asks for the `:returns:` and `:yields:` fields of reStructuredText docstrings; \
         docstring-missing-returns reads them as missing sections.\n    Fix: ignore docstring-missing-returns in \
         Ruff's `lint.ignore`, or ignore rst-returns in `[tool.simplint]`.\n    \
         {BOTH_IN_PYPROJECT}; first file: module.py\n  \
         - rst-missing-raises (SMP924) and Ruff's docstring-missing-exception (DOC501) work against each other.\n    \
         Why: rst-missing-raises asks for the `:raises:` fields of reStructuredText docstrings; \
         docstring-missing-exception reads them as a missing section.\n    Fix: ignore docstring-missing-exception in \
         Ruff's `lint.ignore`, or ignore rst-missing-raises in `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first \
         file: module.py"
    ) ; "rst_docstrings")]
#[test_case(
    "[tool.ruff]\nlint.preview = true\nlint.select = [\"unnecessary-regular-expression-compile\"]\n",
    "module.py",
    &format!(
        "inline-regex (SMP534) and Ruff's unnecessary-regular-expression-compile work against each other.\n    Why: \
         inline-regex moves a `re.compile` in a function to a module-level constant, which compiles once; \
         unnecessary-regular-expression-compile reports the inline compile and suggests `re.match`, advice that holds \
         only for code that runs once.\n    Fix: \
         ignore unnecessary-regular-expression-compile in Ruff's `lint.ignore`, or ignore inline-regex in \
         `[tool.simplint]`.\n    {BOTH_IN_PYPROJECT}; first file: module.py"
    ) ; "inline_regex_compile")]
fn conflict_fails_before_linting(pyproject: &str, file: &str, conflict: &str) {
    let root = project(&[("pyproject.toml", pyproject), (file, "value = 1\n")]);
    assert_eq!(conflicts(root.path(), &[]), (2, header(conflict)));
}

#[test_case(
    &format!("[project]\nrequires-python = \">=3.13\"\n[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}"),
    "module.py" ;
    "target_before_lazy_annotations"
)]
#[test_case(
    &format!("{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\nlint.isort.required-imports = [\"import os\"]\n"),
    "module.py" ;
    "other_required_import"
)]
#[test_case(
    &format!(
        "{SIMPLINT_314}[tool.ruff]\nlint.select = [\"I002\"]\n{FUTURE}[tool.simplint]\n\
         per-file-ignores = {{ \"module.py\" = [\"SMP301\"] }}\n"
    ),
    "module.py" ;
    "simplint_rule_ignored_for_the_file"
)]
#[test_case(
    "[project]\nrequires-python = \">=3.14\"\n[tool.ruff]\nlint.preview = true\nlint.select = [\"PLW1514\"]\n",
    "module.py" ;
    "target_before_utf8_default"
)]
#[test_case(
    "[tool.ruff]\nlint.select = [\"TID252\"]\n",
    "module.py" ;
    "parent_relative_imports_banned"
)]
#[test_case(
    "[tool.ruff]\nlint.select = [\"TID252\"]\nlint.flake8-tidy-imports.ban-relative-imports = \"all\"\n\
     [tool.flake8-lazy]\n",
    "module.py" ;
    "flake8_lazy_leaves_relative_imports_alone"
)]
#[test_case("[tool.ruff]\nlint.select = [\"D103\"]\n", "_private.py" ; "private_module")]
#[test_case("[tool.ruff]\nlint.select = [\"D100\"]\n", "module.py" ; "not_a_test_module")]
#[test_case(
    "[tool.ruff]\nlint.select = [\"D100\"]\nlint.per-file-ignores = { \"tests/*\" = [\"D100\"] }\n",
    "tests/test_module.py" ;
    "ruff_rule_ignored_for_tests"
)]
#[test_case("[tool.ruff]\nlint.preview = true\nlint.select = [\"DOC201\"]\n", "module.py" ; "docstrings_not_rst")]
fn combination_without_conflict_lints(pyproject: &str, file: &str) {
    let root = project(&[("pyproject.toml", pyproject), (file, "value = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP901,SMP902,SMP301,SMP302,SMP521,SMP923"]),
        (0, common::PASSED.to_owned(), String::new())
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
                "test-module-docstring (SMP902) and Ruff's undocumented-public-module (D100) work against each \
                 other.\n    Why: test-module-docstring removes the docstrings of test modules; \
                 undocumented-public-module asks for them.\n    Fix: ignore undocumented-public-module for test files \
                 in Ruff's `lint.per-file-ignores`, or ignore test-module-docstring in `[tool.simplint]`.\n    \
                 {place}\n  - absolute-sibling-import (SMP302) and Ruff's relative-imports (TID252) undo each other's \
                 fixes.\n    Why: absolute-sibling-import makes imports from the file's own package relative; \
                 relative-imports turns them back into absolute ones because \
                 `lint.flake8-tidy-imports.ban-relative-imports` is `\"all\"`.\n    Fix: set `ban-relative-imports` \
                 to `\"parents\"`, or ignore absolute-sibling-import in `[tool.simplint]`.\n    {place}"
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
    assert_eq!((code, stdout, stderr.contains("SMP301 (future-annotations) and Ruff's")), (2, String::new(), true));
}

// The error names the project as `<root>` whatever spelling the settings use, with `/` between path parts.
fn conflicts(root: &Path, args: &[&str]) -> (i32, String) {
    let (code, _, stderr) = check(root, args);
    let mut error = stderr;
    for spelling in [canonical(root), root.to_path_buf()] {
        error = error.replace(&spelling.display().to_string(), "<root>");
    }
    (code, error.replace('\\', "/"))
}

fn header(conflicts: &str) -> String {
    format!(
        "simplint failed\n  Cause: simplint and Ruff rules work against each other, so fixing never leaves the code \
         clean:\n  - {conflicts}\n"
    )
}
