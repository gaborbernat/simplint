mod common;

use std::fs;
use std::path::Path;

use test_case::test_case;

use common::{SECTION, SINGLE_X, check, project, read, simplint};

#[test]
fn pyproject_configures_rules_and_target() {
    let root = project(&[
        (
            "pyproject.toml",
            r#"
[project]
requires-python = ">=3.14"
dependencies = ["Requests>=2"]
scripts = { tool = "cli:main" }

[tool.ruff]
line-length = 100
extend-exclude = ["generated"]

[tool.simplint]
select = ["SMP0", "SMP301", "SMP401"]
extend-select = ["SMP303"]
ignore = ["SMP001"]
per-file-ignores = { "legacy.py" = ["SMP101"] }
"#,
        ),
        ("cli.py", "def main() -> None:\n    pass\n"),
        (
            "module.py",
            "# --- helpers ---\nfrom __future__ import annotations\ntry:\n    import requests\nexcept ImportError:\n    requests = None\n",
        ),
        ("legacy.py", "x = 1\n"),
        ("generated/output.py", "x = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &[]).1,
        "cli.py:1:5: SMP401 `main` is imported by other modules; list it in `__all__`\n\
         module.py:2:24: SMP301 [*] Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`\n\
         module.py:3:1: SMP303 Import guard around a required dependency; import it directly and let a broken install fail\n\
         Found 3 errors.\n[*] 1 fixable with `--fix` (2 unsafe fixes available with `--fix --unsafe-fixes`).\n"
    );
}

#[test]
fn pyproject_run_options_apply() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nselect = [\"SMP001\"]\nfix = true\nshow-fixes = true\noutput-format = \"concise\"\n",
        ),
        ("module.py", "# --- helpers ---\nVALUE = 1\n"),
    ]);
    assert_eq!(
        simplint(root.path(), &["check"]).1,
        "\nFixed 1 error:\n- module.py:\n    1 × SMP001 (section-comment)\n\nFound 1 error (1 fixed, 0 remaining).\n"
    );
}

#[test]
fn command_line_flags_override_pyproject_run_options() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nselect = [\"SMP001\"]\nfix = true\nunsafe-fixes = true\nfix-only = true\n",
        ),
        ("module.py", "# --- helpers ---\nVALUE = 1\n"),
    ]);
    let result = check(root.path(), &["--no-fix-only", "--no-fix", "--no-unsafe-fixes", "--no-show-fixes"]);
    assert_eq!((result.0, read(root.path(), "module.py")), (1, "# --- helpers ---\nVALUE = 1\n".to_owned()));
}

#[test]
fn pyproject_unknown_simplint_key_fails() {
    let root = project(&[("pyproject.toml", "[tool.simplint]\nnot-an-option = 1\n"), ("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("unknown field `not-an-option`")), (2, true));
}

#[test_case("[tool.simplint]\nselect = 1\n" ; "wrong_type")]
#[test_case("[project]\nname = 3\n" ; "project_wrong_type")]
#[test_case("[project\n" ; "not_toml")]
#[test_case("[tool.simplint]\ndocstring-style = \"numpy\"\n" ; "unknown_docstring_style")]
fn pyproject_invalid_fails(content: &str) {
    let root = project(&[("pyproject.toml", content), ("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("Failed to parse")), (2, true));
}

#[test]
fn pyproject_extend_per_file_ignores_adds_to_per_file_ignores() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nper-file-ignores = { \"other.py\" = [\"SMP001\"] }\nextend-per-file-ignores = { \"module.py\" = [\"SMP101\"] }\n",
        ),
        ("module.py", "x = 1\n"),
        ("other.py", "# --- helpers ---\nx = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP101"]),
        (1, format!("other.py:2:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test]
fn dependency_guard_maps_import_names_and_skips_markers() {
    let root = project(&[
        ("pyproject.toml", "[project]\ndependencies = [\"PyYAML>=6\", \"tomli>=2; python_version < '3.11'\"]\n"),
        (
            "module.py",
            "try:\n    import yaml\nexcept ImportError:\n    yaml = None\ntry:\n    import tomli\nexcept ImportError:\n    tomli = None\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP303"]).1,
        "module.py:1:1: SMP303 Import guard around a required dependency; import it directly and let a broken install \
         fail\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    );
}

#[test]
fn nested_pyproject_configures_its_own_files() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP101\"]\n"),
        ("sub/pyproject.toml", "[tool.simplint]\nselect = [\"SMP001\"]\n"),
        ("module.py", "x = 1\n"),
        ("sub/module.py", "x = 1\n"),
        ("sub/other.py", "y = 1\n"),
    ]);
    assert_eq!(check(root.path(), &[]).1, format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"));
}

#[test]
fn nested_pyproject_without_simplint_table_keeps_the_parent_settings() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP1\"]\nignore = [\"SMP102\"]\n"),
        ("sub/pyproject.toml", "[project]\nname = \"sub\"\n"),
        ("sub/module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    let stdout = format!("{}:2:1: {SINGLE_X}\nFound 1 error.\n", common::native("sub/module.py"));
    assert_eq!(check(root.path(), &[]).1, stdout);
}

#[test]
fn pyproject_without_simplint_table_still_gives_project_metadata() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"root\"\n"),
        ("sub/pyproject.toml", "[project]\nname = \"sub\"\ndependencies = [\"PyYAML\"]\n"),
        ("sub/module.py", "try:\n    import yaml\nexcept ImportError:\n    yaml = None\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP303"]).0, 1);
}

// A workspace root can configure simplint without describing the project, so each member's own `[project]` and
// `[build-system]` decide whether it is a library, and a member without either inherits the root's.
#[test_case(
    "[project]\nname = \"ws\"\n[tool.simplint]\n", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n", 0 ;
    "member_library_under_unbuilt_root"
)]
#[test_case(
    "[build-system]\nrequires = []\n[project]\nname = \"ws\"\n[tool.simplint]\n", "[project]\nname = \"demo\"\n", 1 ;
    "member_application_under_built_root"
)]
#[test_case(
    "[build-system]\nrequires = []\n[project]\nname = \"ws\"\n[tool.simplint]\n", "[tool.ruff]\n", 0 ;
    "member_without_project_inherits_root"
)]
fn member_metadata_comes_from_its_own_pyproject(root_pyproject: &str, member_pyproject: &str, exit: i32) {
    let root = project(&[
        ("pyproject.toml", root_pyproject),
        ("sub/pyproject.toml", member_pyproject),
        ("sub/demo/__init__.py", "from ._app import run as run\n"),
        ("sub/demo/_app.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP407"]).0, exit);
}

#[cfg(unix)]
#[test]
fn unreadable_pyproject_fails() {
    let root = project(&[("pyproject.toml", "[project]\n"), ("module.py", "VALUE = 1\n")]);
    common::set_mode(&root.path().join("pyproject.toml"), 0o000);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("Failed to read")), (2, true));
}

#[test_case(&[] ; "discovered")]
#[test_case(&["--config", "sub/pyproject.toml"] ; "config_file")]
fn unknown_selector_names_its_pyproject(flags: &[&str]) {
    let root = project(&[
        ("sub/pyproject.toml", "[tool.simplint]\nselect = [\"SMP999\"]\n"),
        ("sub/module.py", "VALUE = 1\n"),
    ]);
    let pyproject = common::canonical(root.path()).join("sub").join("pyproject.toml");
    assert_eq!(
        check(root.path(), &[flags, &["sub"]].concat()),
        (
            2,
            String::new(),
            format!(
                "simplint failed\n  Cause: Unknown rule selector `SMP999` in `select` from `{}`\n",
                pyproject.display()
            )
        )
    );
}

#[test]
fn isolated_ignores_configuration() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nignore = [\"SMP101\"]\n[tool.ruff]\nexclude = [\"module.py\"]\n"),
        ("module.py", "x = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--isolated", "--select", "SMP101"]).0, 1);
}

#[test]
fn isolated_rejects_a_config_file() {
    let root = project(&[("ruff.toml", "line-length = 100\n"), ("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--isolated", "--config", "ruff.toml"]);
    assert_eq!(
        (code, stderr.contains("The argument `--config=ruff.toml` cannot be used with `--isolated`")),
        (2, true)
    );
}

// The configuration replaces the project's, which selects only SMP001; a `ruff.toml` holds no simplint options, so the
// defaults apply.
#[test_case(
    "pyproject.toml",
    "[tool.simplint]\nselect = [\"SMP101\"]\n",
    "module.py:1:1: SMP101 Single-letter name `x`; only comprehensions may use one\nFound 1 error.\n" ;
    "pyproject"
)]
#[test_case(
    "ruff.toml",
    "target-version = \"py310\"\n",
    "module.py:1:1: SMP101 Single-letter name `x`; only comprehensions may use one\nmodule.py:1:1: \
     SMP102 Module-scope variable `x` should be named `X`\nmodule.py:1:1: SMP404 `x` is not in `__all__`; name it `_x`\n\
     module.py:1:1: SMP601 `x` is never reassigned; annotate it `Final`\nFound 4 errors.\n3 unsafe fixes available with \
     `--fix --unsafe-fixes`.\n" ;
    "ruff_toml"
)]
fn config_file_is_used_for_every_file(name: &str, content: &str, stdout: &str) {
    let path = format!("config/{name}");
    let root = project(&[
        (&path, content),
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP001\"]\n"),
        ("module.py", "x = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--config", &path]), (1, stdout.to_owned(), String::new()));
}

#[test]
fn relative_config_anchors_per_file_ignores_at_its_directory() {
    let root = project(&[
        ("sub/pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"pkg/*\" = [\"SMP101\"] }\n"),
        ("sub/pkg/pyproject.toml", "[tool.simplint]\n"),
        ("sub/pkg/module.py", "x = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--config", "sub/pyproject.toml", "--select", "SMP101"]),
        (0, "All checks passed!\n".to_owned(), String::new())
    );
}

#[test_case(&["--config", "select = ['SMP001']"] ; "after_the_subcommand")]
#[test_case(&["--config", "select = ['SMP001']", "--config", "select = ['SMP101']"] ; "first_override_wins")]
fn inline_config_overrides_simplint_options(flags: &[&str]) {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP101\"]\n"),
        ("module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    let mut args = vec!["check", "--output-format", "concise"];
    args.extend_from_slice(flags);
    assert_eq!(
        simplint(root.path(), &args).1,
        format!("module.py:1:1: {SECTION}\nFound 1 error.\n[*] 1 fixable with `--fix`.\n")
    );
}

#[test]
fn inline_config_goes_before_the_subcommand() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        simplint(root.path(), &["--config", "select = ['SMP001']", "check", "--output-format", "concise"]),
        (1, format!("module.py:1:1: {SECTION}\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"), String::new())
    );
}

#[test]
fn inline_config_splits_simplint_and_ruff_keys() {
    let root = project(&[
        ("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"),
        ("module.py", "from __future__ import annotations\nx = 1\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--config", "select = ['SMP301']\ntarget-version = 'py313'"]);
    assert_eq!((code, stdout), (0, "All checks passed!\n".to_owned()));
}

#[test_case(&[("pyproject.toml", "[tool.simplint.naming]\nkeep-names = [\"app_name\"]\n")], &[] ; "pyproject")]
#[test_case(&[], &["--config", "naming.keep-names = ['app_name']"] ; "inline")]
#[test_case(
    &[("pyproject.toml", "[tool.simplint.naming]\nkeep-names = [\"other\"]\n")],
    &["--config", "naming = { keep-names = ['app_name'] }\nline-length = 100"] ;
    "inline_beats_pyproject"
)]
fn keep_names_spare_module_variables(files: &[(&str, &str)], flags: &[&str]) {
    let root = project(&[files, &[("module.py", "app_name = \"shop\"\nlimit = 3\n")]].concat());
    assert_eq!(
        check(root.path(), &[&["--select", "SMP102"], flags].concat()).1,
        "module.py:2:1: SMP102 Module-scope variable `limit` should be named `LIMIT`\nFound 1 error.\n\
         1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    );
}

#[test_case("[tool.simplint.imports]\nmodule-attributes = [\"pkg.findings\"]\n", &[] ; "pyproject")]
#[test_case("", &["--config", "imports.module-attributes = ['pkg.findings']"] ; "inline")]
fn module_attributes_leave_listed_modules_alone(pyproject: &str, flags: &[&str]) {
    let root = project(&[
        ("pyproject.toml", pyproject),
        ("pkg/__init__.py", "# A package.\n"),
        ("pkg/findings.py", "class DbWriter:\n    pass\n"),
        ("pkg/consumer.py", "from . import findings\n\nwriter = findings.DbWriter()\n"),
    ]);
    assert_eq!(
        check(root.path(), &[&["--select", "SMP304"], flags].concat()),
        (0, "All checks passed!\n".to_owned(), String::new())
    );
}

#[test]
fn module_attributes_report_unlisted_modules() {
    let root = project(&[
        ("pkg/__init__.py", "# A package.\n"),
        ("pkg/findings.py", "class DbWriter:\n    pass\n"),
        ("pkg/consumer.py", "from . import findings\n\nwriter = findings.DbWriter()\n"),
    ]);
    assert!(check(root.path(), &["--select", "SMP304"]).1.contains("SMP304 Import `DbWriter` from `.findings`"));
}

#[test]
fn module_attributes_ignore_modules_outside_the_run() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nextend-exclude = [\"excluded\"]\n"),
        ("excluded/__init__.py", "# Excluded.\n"),
        ("excluded/mod.py", "class Writer:\n    pass\n"),
        ("pkg/__init__.py", "# A package.\n"),
        ("pkg/consumer.py", "from excluded import mod\n\nwriter = mod.Writer()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP304"]), (0, "All checks passed!\n".to_owned(), String::new()));
}

#[test_case(&[], &["--config", "max-paragraph-words = 4"], "module.py:1:1: SMP002 Paragraph exceeds 4 words; split it at a change of topic\nFound 1 error.\n" ; "limit_inline")]
#[test_case(&[("pyproject.toml", "[tool.simplint]\nmax-paragraph-words = 4\n")], &[], "module.py:1:1: SMP002 Paragraph exceeds 4 words; split it at a change of topic\nFound 1 error.\n" ; "limit_in_pyproject")]
#[test_case(&[], &[], "All checks passed!\n" ; "default_limit")]
fn paragraph_word_limit_decides_long_paragraphs(files: &[(&str, &str)], flags: &[&str], expected: &str) {
    let root = project(&[files, &[("module.py", "# One two three four five.\nVALUE = 1\n")]].concat());
    assert_eq!(check(root.path(), &[&["--select", "SMP002"], flags].concat()).1, expected);
}

#[test_case(&[], &[], "module.py:6:5: SMP505 `_parse_header` is called only from `load`, and inlining keeps `load` short; inline it\nFound 1 error.\n" ; "defaults")]
#[test_case(&[("pyproject.toml", "[tool.simplint.logic]\nmax-helper-statements = 2\n")], &[], "All checks passed!\n" ; "helper_limit_in_pyproject")]
#[test_case(&[], &["--config", "logic.max-caller-statements = 3"], "All checks passed!\n" ; "caller_limit_inline")]
fn logic_limits_decide_single_caller_helpers(files: &[(&str, &str)], flags: &[&str], expected: &str) {
    let source = "def load(text):\n    header = _parse_header(text)\n    return {\"header\": header}\n\n\ndef _parse_header(text):\n    first = text.splitlines()[0]\n    return first.strip()\n";
    let root = project(&[files, &[("module.py", source)]].concat());
    assert_eq!(check(root.path(), &[&["--select", "SMP505"], flags].concat()).1, expected);
}

const COMPLEX_HELPER: &str = r##"def load(text):
    header = _parse_header(text)
    return {"header": header}


def _parse_header(text):
    try:
        for line in text.splitlines():
            if line.startswith("#"):
                continue
            elif line:
                break
        else:
            line = ""
        while len(line) > 1:
            line = line[1:]
        else:
            line = line.strip()
        with open(line) as handle:
            line = handle.read()
        match line:
            case "a" | "b":
                value = 1
            case str() if line:
                value = 2
            case _:
                value = 3
        class Marker:
            pass
        def marker():
            return 1
    except OSError:
        value = 4
    except ValueError:
        value = 5
    else:
        value = 6
    finally:
        value = 7
    return value
"##;
const ASSIGNED_RETURNS: &str = r#"def load(text):
    if not text:
        return ""
    header = _parse_header(text)
    return header


def _parse_header(text):
    if text:
        return text.strip()
    return ""
"#;
const RETURNED_CALL: &str = r#"def load(text):
    if not text:
        return ""
    return _parse_header(text)


def _parse_header(text):
    if text:
        return text.strip()
    return ""
"#;
const ELIF_CALL: &str = r#"def load(text):
    if text is None:
        return ""
    elif _parse_header(text):
        return "set"
    return "unset"


def _parse_header(text):
    if text:
        return text.strip()
    return ""
"#;
const TERNARY_CALL: &str = r"def load(text, a, b):
    return a if b else _parse_header(text)


def _parse_header(text):
    assert text
    return text.strip()
";
const AND_CALL: &str = r"def load(text, a):
    return a and _parse_header(text)


def _parse_header(text):
    assert text
    return text.strip()
";
const ELIF_WITH_RETURN: &str = r#"def load(text):
    if text is None:
        return ""
    elif _parse_header(text):
        return "set"
    return "unset"


def _parse_header(text):
    with open(text) as handle:
        return handle.read()
"#;
const TERNARY_LONE_RETURN: &str = r"def load(text, a, b):
    return a if b else _parse_header(text)


def _parse_header(text):
    return text.strip()
";
const ELIF_ONE_RETURN: &str = r#"def load(text):
    if text is None:
        return ""
    elif _parse_header(text):
        return "set"
    return "unset"


def _parse_header(text):
    assert text
    return text.strip()
"#;
const TRY_HELPER: &str = r#"def load(text):
    header = _parse_header(text)
    return {"header": header}


def _parse_header(text):
    try:
        return text.strip()
    except ValueError:
        return ""
"#;

#[test_case("", COMPLEX_HELPER, true ; "default_reports")]
#[test_case("[tool.ruff.lint]\nselect = [\"C901\"]\n\n[tool.ruff.lint.mccabe]\nmax-complexity = 10\n", COMPLEX_HELPER, false ; "complexity_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"C901\"]\n\n[tool.ruff.lint.mccabe]\nmax-complexity = 11\n", COMPLEX_HELPER, true ; "complexity_limit_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 12\n", COMPLEX_HELPER, false ; "branches_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 13\n", COMPLEX_HELPER, true ; "branches_limit_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 4\n", ELIF_CALL, false ; "elif_branches_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 5\n", ELIF_CALL, true ; "elif_branches_limit_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 31\n", COMPLEX_HELPER, false ; "statements_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 32\n", COMPLEX_HELPER, true ; "statements_limit_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 5\n", TRY_HELPER, false ; "try_returns_statements_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 6\n", TRY_HELPER, true ; "try_returns_statements_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 6\n", ELIF_CALL, false ; "elif_statements_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0915\"]\n\n[tool.ruff.lint.pylint]\nmax-statements = 7\n", ELIF_CALL, true ; "elif_statements_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"C901\"]\n\n[tool.ruff.lint.mccabe]\nmax-complexity = 1\n", TERNARY_CALL, false ; "ternary_operand_complexity_skipped")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0911\"]\n\n[tool.ruff.lint.pylint]\nmax-returns = 1\n", TERNARY_CALL, false ; "ternary_operand_returns_skipped")]
#[test_case("", TERNARY_CALL, true ; "ternary_operand_default_reports")]
#[test_case("[tool.ruff.lint]\nselect = [\"C901\"]\n\n[tool.ruff.lint.mccabe]\nmax-complexity = 1\n", AND_CALL, false ; "later_boolean_operand_skipped")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 2\n", ELIF_ONE_RETURN, false ; "elif_single_return_branches_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 3\n", ELIF_ONE_RETURN, true ; "elif_single_return_branches_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 2\n", ELIF_WITH_RETURN, false ; "elif_with_return_branches_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0912\"]\n\n[tool.ruff.lint.pylint]\nmax-branches = 3\n", ELIF_WITH_RETURN, true ; "elif_with_return_branches_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"C901\"]\n\n[tool.ruff.lint.mccabe]\nmax-complexity = 1\n", TERNARY_LONE_RETURN, true ; "ternary_lone_return_complexity_reported")]
#[test_case("", ASSIGNED_RETURNS, true ; "returns_default_reports")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0911\"]\n\n[tool.ruff.lint.pylint]\nmax-returns = 2\n", ASSIGNED_RETURNS, true ; "assigned_call_returns_at_the_count")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0911\"]\n\n[tool.ruff.lint.pylint]\nmax-returns = 2\n", RETURNED_CALL, false ; "returned_call_limit_crossed")]
#[test_case("[tool.ruff.lint]\nselect = [\"PLR0911\"]\n\n[tool.ruff.lint.pylint]\nmax-returns = 3\n", RETURNED_CALL, true ; "returned_call_limit_at_the_count")]
fn ruff_limits_decide_single_caller_helpers(pyproject: &str, source: &str, reported: bool) {
    let root = project(&[("pyproject.toml", pyproject), ("module.py", source)]);
    assert_eq!(check(root.path(), &["--select", "SMP505"]).1.contains("SMP505"), reported);
}

#[test]
fn inline_config_reports_simplint_table_errors() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--config", "naming.keep-names = 1"]);
    assert_eq!((code, stderr.contains("tip: invalid type: integer `1`, expected a sequence")), (2, true));
}

#[test]
fn inline_config_reports_ruff_errors() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--config", "line-length = 'x'"]);
    assert_eq!(
        (code, stderr.contains("Could not parse the supplied argument as a `ruff.toml` configuration option")),
        (2, true)
    );
}

#[test]
fn ruff_excludes_apply() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nextend-exclude = [\"generated\"]\n"),
        ("generated/output.py", "x = 1\n"),
        (".gitignore", "ignored.py\n"),
        ("ignored.py", "x = 1\n"),
        ("module.py", "VALUE = 1\n"),
    ]);
    fs::create_dir(root.path().join(".git")).expect("git directory");
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

#[test]
fn ruff_lint_exclude_applies() {
    let root =
        project(&[("pyproject.toml", "[tool.ruff.lint]\nexclude = [\"module.py\"]\n"), ("module.py", "x = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

// `ruff check` already warns about its own configuration, so simplint repeating it on every run is noise; a warning
// about simplint's own work still shows.
#[test_case("module.py", "" ; "ruff_configuration")]
#[test_case("notes.txt", "warning: No Python files found under the given path(s)\n" ; "simplint_warning")]
fn ruff_configuration_warnings_stay_with_ruff(file: &str, stderr: &str) {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nextend-select = [\"E\"]\n\n[tool.ruff.lint]\nignore = [\"ANN101\"]\n"),
        (file, "VALUE = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).2, stderr);
}

// pytest imports a test module whose directory lacks `__init__.py` as a top-level module, where a relative import
// fails, even when Ruff's `namespace-packages` names the directory.
#[test_case(&[], 0 ; "namespace_package_directory")]
#[test_case(&[("tests/unit/__init__.py", "")], 1 ; "regular_package_directory")]
fn absolute_sibling_import_needs_an_init_module(files: &[(&str, &str)], code: i32) {
    let root = project(
        &[
            files,
            &[
                ("pyproject.toml", "[tool.ruff]\nnamespace-packages = [\"tests\"]\n"),
                ("tests/unit/support.py", "VALUE = 1\n"),
                ("tests/unit/test_module.py", "from tests.unit.support import VALUE\n\nassert VALUE\n"),
            ],
        ]
        .concat(),
    );
    assert_eq!(check(root.path(), &["--select", "SMP302"]).0, code);
}

// Without a `pyproject.toml` the settings root is the directory the run starts from, so a module under a `test` or
// `tests` package counts as test code, whatever directory of the package it sits in.
#[test_case("test_module.py", true ; "test_prefix")]
#[test_case("test/support/__init__.py", true ; "support_package")]
#[test_case("test/test_import/data/module.py", true ; "data_under_test_package")]
#[test_case("tests/helpers.py", true ; "tests_package")]
#[test_case("support/module.py", false ; "not_a_test_module")]
fn test_module_without_pyproject(name: &str, reported: bool) {
    let root = project(&[(name, "\"\"\"Support module.\"\"\"\n")]);
    let (code, stdout, _) = check(root.path(), &["--select", "SMP902"]);
    assert_eq!((code, stdout.contains("SMP902")), (i32::from(reported), reported));
}

const LAZY_ANNOTATIONS: &str = "module.py:1:24: SMP301 [*] Annotations are evaluated lazily from Python 3.14; drop `from \
                                 __future__ import annotations`\nFound 1 error.\n[*] 1 fixable with `--fix`.\n";
const PASSED: &str = "All checks passed!\n";

#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python")]
#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \"~=3.14\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python_compatible")]
#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \"==3.14.*\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python_wildcard")]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &["--target-version", "py313"], PASSED ;
    "flag_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &["--config", "target-version = 'py313'"], PASSED ;
    "inline_config_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n[tool.ruff]\ntarget-version = \"py312\"\n")], &[], PASSED ;
    "pyproject_ruff_target_version_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"), (".ruff.toml", "target-version = \"py313\"\n")],
    &[],
    PASSED ;
    "dot_ruff_toml_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"), ("ruff.toml", "target-version = \"py313\"\n")],
    &[],
    PASSED ;
    "ruff_toml_wins"
)]
#[test_case(
    &[(".config/ruff/ruff.toml", "target-version = \"py312\"\n")], &[], PASSED ;
    "user_configuration_without_a_project_configuration"
)]
#[test_case(
    &[
        (".config/ruff/ruff.toml", "target-version = \"py312\"\n"),
        ("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"),
    ],
    &[],
    LAZY_ANNOTATIONS ;
    "requires_python_beats_the_user_configuration"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py314\"\nper-file-target-version = { \"legacy.py\" = \"py310\" }\n"),
        ("legacy.py", "from __future__ import annotations\n"),
    ],
    &[],
    LAZY_ANNOTATIONS ;
    "per_file_target_version"
)]
fn target_version_resolution(files: &[(&str, &str)], flags: &[&str], stdout: &str) {
    let root = project(&[files, &[("module.py", "from __future__ import annotations\n")]].concat());
    assert_eq!(check(root.path(), &[&["--select", "SMP301"], flags].concat()).1, stdout);
}

#[test]
fn show_settings_describes_the_first_file() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nrequires-python = \">=3.12\"\ndependencies = [\"requests\"]\nscripts = { tool = \"cli:main\" }\n[dependency-groups]\ndev = [\"pytest-mock>=3\"]\n[tool.simplint]\nselect = [\"SMP001\"]\nper-file-ignores = { \"!b.py\" = [\"SMP001\"] }\nextend-safe-fixes = [\"SMP001\"]\nproject-kind = \"library\"\napi-packages = [\"src/app\"]\ndocstring-style = \"rst\"\n[tool.simplint.naming]\nkeep-names = [\"app_name\"]\n[tool.simplint.imports]\nmodule-attributes = [\"app.settings\"]\n[tool.simplint.logic]\nmax-tuple-elements = 3\nmax-cognitive-complexity = 15\n",
        ),
        ("b.py", ""),
        ("a.py", ""),
    ]);
    let (code, stdout, _) = check(root.path(), &["--show-settings", "--unfixable", "ALL"]);
    let path = |name: &str| common::quoted(&root.path().join(name));
    let (file, pyproject) = (path("a.py"), path("pyproject.toml"));
    // The directories print quoted as they display, the files with escaped backslashes.
    let cache_dir = format!("\"{}\"", root.path().join(".ruff_cache").join("simplint").display());
    let project_root = format!("\"{}\"", root.path().display());
    let ignored = common::quoted(Path::new(&format!("!{}", root.path().join("b.py").display())));
    assert_eq!(
        (code, stdout),
        (
            0,
            format!(
                "Resolved settings for: {file}\nSettings path: {pyproject}\n\n# General Settings\n\
                 cache_dir = {cache_dir}\nfix = false\nfix_only = false\noutput_format = concise\n\
                 show_fixes = false\nunsafe_fixes = hint\n\n# Linter Settings\nlinter.project_root = {project_root}\n\
                 linter.target_version = 3.12\nlinter.rules.enabled = [\n\tsection-comment (SMP001),\n]\n\
                 linter.rules.should_fix = []\nlinter.per_file_ignores = [\n\t{ignored} = [\"section-comment (SMP001)\"],\n]\n\
                 linter.safety_table.forced_safe = [\n\tsection-comment (SMP001),\n]\nlinter.safety_table.forced_unsafe = []\n\
                 linter.dependencies = [\n\t\"requests\",\n]\nlinter.declared_dependencies = [\n\t\"pytest_mock\",\n\t\"requests\",\n]\nlinter.entry_points = [\n\t\"cli:main\",\n]\n\
                 linter.max_paragraph_words = 100\nlinter.naming.keep_names = [\n\t\"app_name\",\n]\nlinter.naming.meaningless_names = [\n\t\"foo\",\n\t\"baz\",\n\t\"qux\",\n\t\"quux\",\n]\nlinter.naming.catch_all_modules = [\n\t\"utils\",\n\t\"util\",\n\t\"utilities\",\n\t\"helpers\",\n\t\"helper\",\n\t\"common\",\n\t\"misc\",\n]\nlinter.imports.module_attributes = [\n\t\"app.settings\",\n]\n\
                 linter.logic.max_helper_statements = 3\nlinter.logic.max_caller_statements = 20\nlinter.logic.max_tuple_elements = 3\n\
                 linter.logic.max_cognitive_complexity = 15\nlinter.logic.min_duplicate_statements = 6\nlinter.project_kind = library\n\
                 linter.api_packages = [\n\t\"app\",\n]\n\
                 linter.docstring_style = rst\n"
            )
        )
    );
}

#[test]
fn show_settings_without_a_pyproject_has_no_settings_path() {
    let root = project(&[("module.py", "")]);
    let (code, stdout, _) = check(root.path(), &["--show-settings"]);
    let file = common::quoted(&root.path().join("module.py"));
    assert_eq!(
        (code, stdout.lines().take(3).collect::<Vec<_>>()),
        (0, vec![format!("Resolved settings for: {file}").as_str(), "", "# General Settings"])
    );
}

#[test]
fn show_settings_without_files_fails() {
    let root = project(&[("notes.txt", "")]);
    let (code, _, stderr) = check(root.path(), &["--show-settings"]);
    assert_eq!((code, stderr.contains("No files found under the given path")), (2, true));
}

// What `--show-settings` prints for one setting, from its name up to the next setting.
fn shown<'s>(stdout: &'s str, setting: &str, next: &str) -> &'s str {
    let start = stdout.find(setting).expect("the setting is shown");
    &stdout[start..stdout[start..].find(next).map_or(stdout.len(), |end| start + end)]
}

#[test_case("[build-system]\nrequires = []\n", &[], "library" ; "build_system")]
#[test_case("", &[("setup.py", "")], "library" ; "setup_py")]
#[test_case("", &[("setup.cfg", "")], "library" ; "setup_cfg")]
#[test_case("[project]\nname = \"tool\"\n", &[], "application" ; "nothing_installs_it")]
#[test_case("[build-system]\nrequires = []\n[tool.uv]\npackage = false\n", &[], "application" ; "uv_installs_no_package")]
#[test_case("[build-system]\nrequires = []\n[tool.maturin]\nbindings = \"bin\"\n", &[], "application" ; "maturin_binary")]
#[test_case("[tool.simplint]\nproject-kind = \"library\"\n", &[], "library" ; "set_to_library")]
#[test_case("[build-system]\nrequires = []\n[tool.simplint]\nproject-kind = \"application\"\n", &[], "application" ; "set_to_application")]
fn project_kind_follows_how_the_project_installs(pyproject: &str, files: &[(&str, &str)], kind: &str) {
    let root = project(&[&[("pyproject.toml", pyproject), ("module.py", "")], files].concat());
    let stdout = check(root.path(), &["--show-settings"]).1;
    assert_eq!(shown(&stdout, "linter.project_kind", "\n"), format!("linter.project_kind = {kind}"));
}

#[test_case(
    "[tool.hatch.build.targets.wheel]\npackages = [\"src/hatched\"]\n", &[], "[\n\t\"hatched\",\n]" ;
    "hatch_packages"
)]
#[test_case("[tool.uv.build-backend]\nmodule-name = \"built\"\n", &[], "[\n\t\"built\",\n]" ; "uv_module_name")]
#[test_case(
    "[tool.uv.build-backend]\nmodule-name = \"acme.widgets\"\n", &[], "[\n\t\"acme.widgets\",\n]" ;
    "uv_namespace_module_name"
)]
#[test_case("[tool.maturin]\nmodule-name = \"native._core\"\n", &[], "[\n\t\"native\",\n]" ; "maturin_module_name")]
#[test_case(
    "[tool.setuptools]\npackages = 3\n", &[("src/typed/py.typed", ""), ("src/plain/__init__.py", "")],
    "[\n\t\"typed\",\n]" ; "py_typed_marker"
)]
#[test_case("[project]\nname = \"named-package\"\n", &[("named_package/__init__.py", "")], "[\n\t\"named_package\",\n]" ; "project_name")]
#[test_case("[project]\nname = \"absent\"\n", &[], "every module outside docs and tests" ; "fallback")]
#[test_case("[tool.simplint]\napi-packages = [\"chosen.sub\"]\n", &[], "[\n\t\"chosen.sub\",\n]" ; "configured")]
fn api_packages_come_from_the_build_configuration(pyproject: &str, files: &[(&str, &str)], packages: &str) {
    let root = project(&[&[("pyproject.toml", pyproject), ("module.py", "")], files].concat());
    let stdout = check(root.path(), &["--show-settings"]).1;
    let shown = shown(&stdout, "linter.api_packages", "\nlinter.docstring_style");
    assert_eq!(shown, format!("linter.api_packages = {packages}"));
}

// Ruff has no namesake for these options, so an inline `--config` must not hand them to Ruff's parser.
#[test]
fn inline_config_sets_options_ruff_does_not_know() {
    let root = project(&[("module.py", "")]);
    let args = [
        "--show-settings",
        "--config",
        "project-kind = \"library\"",
        "--config",
        "api-packages = [\"module\"]",
        "--config",
        "docstring-style = \"rst\"",
    ];
    let stdout = check(root.path(), &args).1;
    assert_eq!(
        shown(&stdout, "linter.project_kind", "\nlinter.docstring_style = rst\n"),
        "linter.project_kind = library\nlinter.api_packages = [\n\t\"module\",\n]"
    );
}

// Ruff quotes the text of an `extend`ed file it cannot load; one outside the project is named without it.
#[test]
fn extend_outside_the_project_is_named_without_its_text() {
    let root = project(&[
        ("repo/pyproject.toml", "[tool.ruff]\nextend = \"../outside/notes.txt\"\n"),
        ("repo/module.py", "VALUE = 1\n"),
        ("outside/notes.txt", "API_TOKEN = \"hunter2\"\n"),
    ]);
    let (code, _, stderr) = check(&root.path().join("repo"), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("hunter2"), stderr.contains("is not valid configuration")), (2, false, true));
}

// The same goes for a nested configuration file that `extend`s outside the project, which Ruff loads per directory.
#[test]
fn nested_extend_outside_the_project_is_named_without_its_text() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\n"),
        ("nested/pyproject.toml", "[tool.ruff]\nextend = \"../../outside/notes.txt\"\n"),
        ("nested/module.py", "VALUE = 1\n"),
        ("outside/notes.txt", "API_TOKEN = \"hunter2\"\n"),
    ]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("hunter2"), stderr.contains("is not valid configuration")), (2, false, true));
}

// An `extend`ed file inside the project is part of the run, so Ruff's report of it stays.
#[test]
fn extend_inside_the_project_keeps_ruffs_report() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nextend = \"extra.toml\"\n"),
        ("extra.toml", "API_TOKEN = \"hunter2\"\n"),
        ("module.py", "VALUE = 1\n"),
    ]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("Failed to load extended configuration")), (2, true));
}

// `--isolated` reads no `pyproject.toml`, so an invalid one beside a nested test project does not fail the run.
#[test]
fn isolated_run_reads_no_nested_pyproject() {
    let root = project(&[
        ("tests/roots/test-a/conf.py", ""),
        ("tests/roots/test-a/pyproject.toml", "this is = = not toml\n"),
        ("tests/roots/test-a/mod.py", "X = 1\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--isolated", "--select", "SMP101"]);
    assert_eq!((code, stdout.contains("SMP101")), (1, true));
}
