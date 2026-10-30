use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;
use test_case::test_case;

fn project(files: &[(&str, &str)]) -> TempDir {
    let root = TempDir::new().expect("temporary directory");
    for (name, content) in files {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
        fs::write(path, content).expect("write file");
    }
    root
}

fn simplint(root: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::cargo_bin("simplint").expect("binary").current_dir(root).args(args).output().expect("runs");
    (
        output.status.code().expect("exit code"),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

#[test]
fn check_clean_project_passes() {
    let root = project(&[("module.py", "_VALUE: Final[int] = 1\n")]);
    assert_eq!(
        simplint(root.path(), &["check", "--select", "SMP001"]),
        (0, String::new(), "All checks passed!\n".to_owned())
    );
}

#[test]
fn check_reports_violations() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        simplint(root.path(), &["check", "--select", "SMP001,SMP010"]),
        (
            1,
            "./module.py:1:1: SMP001 [*] Section comment groups code; extract a named function or class instead\n\
             ./module.py:2:1: SMP010 Single-letter name `x`; only loop and comprehension targets may use one\n"
                .to_owned(),
            "Found 2 issues.\n[*] 1 fixable with `--fix`.\n".to_owned()
        )
    );
}

#[test]
fn check_fix_writes_file() {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    let result = simplint(root.path(), &["check", "--fix", "--select", "SMP001"]);
    assert_eq!(
        (result, fs::read_to_string(root.path().join("module.py")).expect("read")),
        ((0, String::new(), "Fixed 1 issues.\n".to_owned()), "VALUE = 1\n".to_owned())
    );
}

#[test]
fn check_unsafe_fixes_are_hidden_by_default() {
    let root = project(&[("module.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        simplint(root.path(), &["check", "--fix", "--select", "SMP002"]).2,
        "Found 1 issues.\n1 hidden fixes can be enabled with `--unsafe-fixes`.\n"
    );
}

#[test]
fn check_unsafe_fixes_apply_when_enabled() {
    let root = project(&[("module.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    simplint(root.path(), &["check", "--fix", "--unsafe-fixes", "--select", "SMP002"]);
    assert_eq!(fs::read_to_string(root.path().join("module.py")).expect("read"), "def run() -> None:\n    print()\n");
}

#[test]
fn check_diff_prints_without_writing() {
    let source = "# --- helpers ---\nVALUE = 1\n";
    let root = project(&[("module.py", source)]);
    let (code, stdout, _) = simplint(root.path(), &["check", "--diff", "--select", "SMP001"]);
    assert_eq!(
        (code, stdout, fs::read_to_string(root.path().join("module.py")).expect("read")),
        (
            0,
            "--- ./module.py\n+++ ./module.py\n@@ -1,2 +1 @@\n-# --- helpers ---\n VALUE = 1\n".to_owned(),
            source.to_owned()
        )
    );
}

#[test]
fn check_accepts_several_paths() {
    let root = project(&[("first/module.py", "x = 1\n"), ("second/module.py", "y = 1\n")]);
    assert_eq!(simplint(root.path(), &["check", "--select", "SMP010", "first", "second"]).2, "Found 2 issues.\n");
}

#[test]
fn check_ignore_option_skips_rule() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(simplint(root.path(), &["check", "--select", "SMP0", "--ignore", "single-letter-name"]).0, 0);
}

#[test]
fn check_unknown_selector_fails() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = simplint(root.path(), &["check", "--select", "NOPE"]);
    assert_eq!((code, stderr.contains("unknown rule selector `NOPE`")), (1, true));
}

#[test]
fn check_syntax_error_fails() {
    let root = project(&[("module.py", "def broken(:\n")]);
    let (code, _, stderr) = simplint(root.path(), &["check"]);
    assert_eq!((code, stderr.contains("syntax error")), (1, true));
}

#[test]
fn check_noqa_suppresses_listed_code() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP010\ny = 2  # noqa\nz = 3  # noqa: E501\n")]);
    assert_eq!(
        simplint(root.path(), &["check", "--select", "SMP010"]).1,
        "./module.py:3:1: SMP010 Single-letter name `z`; only loop and comprehension targets may use one\n"
    );
}

#[test]
fn check_unreadable_file_fails() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    fs::set_permissions(root.path().join("module.py"), fs::Permissions::from_mode(0o000)).expect("chmod");
    let (code, _, stderr) = simplint(root.path(), &["check"]);
    assert_eq!((code, stderr.contains("reading ./module.py")), (1, true));
}

#[test]
fn check_fix_on_read_only_file_fails() {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    fs::set_permissions(root.path().join("module.py"), fs::Permissions::from_mode(0o444)).expect("chmod");
    let (code, _, stderr) = simplint(root.path(), &["check", "--fix", "--select", "SMP001"]);
    assert_eq!((code, stderr.contains("writing ./module.py")), (1, true));
}

#[test]
fn rules_lists_codes() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules"]);
    assert_eq!(
        (code, stdout.lines().take(3).collect::<Vec<_>>()),
        (
            0,
            vec![
                "SMP001 section-comment (fix)",
                "SMP002 function-docstring (fix sometimes)",
                "SMP003 multiline-module-docstring"
            ]
        )
    );
}

#[test]
fn rules_json_carries_explanations() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules", "--output-format", "json"]);
    let first = stdout.split("},").next().expect("one entry").to_owned();
    assert_eq!(
        (
            code,
            first.contains(r#""code": "SMP001""#),
            first.contains(r#""fix": "always""#),
            first.contains("## What it does")
        ),
        (0, true, true, true)
    );
}

#[test_case("dataclass-kw-only", "# dataclass-kw-only (SMP114)", "Fix is always available." ; "by_name_always")]
#[test_case("SMP002", "# function-docstring (SMP002)", "Fix is sometimes available." ; "by_code_sometimes")]
#[test_case("SMP003", "# multiline-module-docstring (SMP003)", "No fix is available." ; "by_code_never")]
fn rule_explains(selector: &str, title: &str, fix: &str) {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", selector]);
    assert_eq!((code, stdout.lines().take(3).collect::<Vec<_>>()), (0, vec![title, "", fix]));
}

#[test]
fn rule_rejects_unknown() {
    let root = project(&[]);
    assert_eq!(simplint(root.path(), &["rule", "SMP999"]), (1, String::new(), "unknown rule `SMP999`\n".to_owned()));
}

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
select = ["SMP0", "SMP108", "SMP113"]
extend-select = ["SMP040"]
ignore = ["SMP001"]
per-file-ignores = { "legacy.py" = ["SMP010"] }
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
        simplint(root.path(), &["check"]).1,
        "./cli.py:1:5: SMP113 [*] `main` is imported by other modules; list it in `__all__`\n\
         ./module.py:2:24: SMP108 [*] Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`\n\
         ./module.py:3:1: SMP040 Import guard around a required dependency; import it directly and let a broken install fail\n"
    );
}

#[test]
fn pyproject_ruff_target_version_wins() {
    let root = project(&[
        ("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n[tool.ruff]\ntarget-version = \"py312\"\n"),
        ("module.py", "from __future__ import annotations\n"),
    ]);
    assert_eq!(simplint(root.path(), &["check", "--select", "SMP108"]).0, 0);
}

#[test]
fn pyproject_invalid_fails() {
    let root = project(&[("pyproject.toml", "[tool.simplint]\nselect = 1\n"), ("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = simplint(root.path(), &["check"]);
    assert_eq!((code, stderr.contains("parsing")), (1, true));
}
