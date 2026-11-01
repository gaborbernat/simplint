mod common;

use std::fs;

use test_case::test_case;

use common::{SECTION, SINGLE_X, check, check_with_stdin, project, simplint_with_stdin};

#[test]
fn check_clean_project_passes() {
    let root = project(&[("module.py", "_VALUE: Final[int] = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP001"]), (0, "All checks passed!\n".to_owned(), String::new()));
}

#[test]
fn check_reports_violations() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP010"]),
        (
            1,
            format!(
                "module.py:1:1: {SECTION}\nmodule.py:2:1: {SINGLE_X}\nFound 2 errors.\n[*] 1 fixable with `--fix`.\n"
            ),
            String::new()
        )
    );
}

#[test]
fn check_exit_zero_succeeds_with_violations() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(
        check(root.path(), &["--exit-zero", "--select", "SMP010"]),
        (0, format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test]
fn check_accepts_several_paths() {
    let root = project(&[("first/module.py", "x = 1\n"), ("second/module.py", "y = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP010", "first", "second"]).1.lines().last(),
        Some("Found 2 errors.")
    );
}

#[test_case(&["--select", "SMP010"], "All checks passed!\n" ; "check")]
#[test_case(&["--show-files"], "" ; "show_files")]
fn check_without_python_files_warns(flags: &[&str], stdout: &str) {
    let root = project(&[("notes.txt", "x = 1\n")]);
    assert_eq!(
        check(root.path(), flags),
        (0, stdout.to_owned(), "warning: No Python files found under the given path(s)\n".to_owned())
    );
}

#[test]
fn check_lints_a_file_named_on_the_command_line_whatever_its_extension() {
    let root = project(&[("script", "x = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP010", "script"]).1.lines().last(), Some("Found 1 error."));
}

#[test]
fn show_files_lists_the_files_to_check() {
    let root = project(&[("pkg/b.py", ""), ("a.py", ""), ("stub.pyi", ""), ("notes.txt", "")]);
    let (code, stdout, _) = check(root.path(), &["--show-files"]);
    let (a, b) = (root.path().join("a.py"), root.path().join("pkg").join("b.py"));
    assert_eq!((code, stdout), (0, format!("{}\n{}\n", a.display(), b.display())));
}

#[test_case(&["--exclude", "module.py"], 0 ; "exclude")]
#[test_case(&["--extend-exclude", "module.py"], 0 ; "extend_exclude")]
#[test_case(&["--extend-exclude", "module.py", "module.py"], 1 ; "explicit_path_wins")]
#[test_case(&["--extend-exclude", "module.py", "--force-exclude", "module.py"], 0 ; "force_exclude")]
#[test_case(&["--extend-exclude", "module.py", "--force-exclude", "--no-force-exclude", "module.py"], 1 ; "no_force_exclude")]
fn exclusion_flags(flags: &[&str], code: i32) {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut args = vec!["--select", "SMP010"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).0, code);
}

#[test_case(&["--respect-gitignore"], 0 ; "respect")]
#[test_case(&["--no-respect-gitignore"], 1 ; "ignore")]
fn gitignore_flags(flags: &[&str], code: i32) {
    let root = project(&[(".gitignore", "module.py\n"), ("module.py", "x = 1\n")]);
    fs::create_dir(root.path().join(".git")).expect("git directory");
    let mut args = vec!["--select", "SMP010"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).0, code);
}

#[test]
fn check_ignore_option_skips_rule() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP0", "--ignore", "single-letter-name"]).0, 0);
}

#[test]
fn check_extend_select_adds_rules() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP001\"]\n"),
        ("module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--extend-select", "SMP010"]).1,
        format!("module.py:1:1: {SECTION}\nmodule.py:2:1: {SINGLE_X}\nFound 2 errors.\n[*] 1 fixable with `--fix`.\n")
    );
}

#[test]
fn extend_ignore_removes_rules() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP010", "--extend-ignore", "SMP001"]).1,
        format!("module.py:2:1: {SINGLE_X}\nFound 1 error.\n")
    );
}

#[test_case(&["--select", "SMP0", "--ignore", "SMP001"], 1 ; "specific_ignore")]
#[test_case(&["--select", "SMP001,SMP010", "--ignore", "SMP0"], 2 ; "specific_select")]
fn more_specific_selector_wins(flags: &[&str], reported: usize) {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(check(root.path(), flags).1.matches("module.py:").count(), reported);
}

#[test_case(&["--per-file-ignores", "module.py:SMP010"], 0 ; "basename")]
#[test_case(&["--per-file-ignores", "pkg/*:SMP010"], 0 ; "relative_path")]
#[test_case(&["--per-file-ignores", "./pkg/*:SMP010"], 0 ; "dot_relative_path")]
#[test_case(&["--per-file-ignores", "!other.py:SMP010"], 0 ; "negated")]
#[test_case(&["--per-file-ignores", "other.py:SMP010"], 1 ; "other_file")]
#[test_case(&["--per-file-ignores", "other.py:SMP010", "--extend-per-file-ignores", "module.py:SMP010"], 0 ; "extend")]
#[test_case(&["--per-file-ignores", "other.py:SMP010"], 1 ; "replaces_configuration")]
fn per_file_ignores_flags(flags: &[&str], code: i32) {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"pkg/*\" = [\"SMP010\"] }\n"),
        ("pkg/module.py", "x = 1\n"),
    ]);
    let mut args = vec!["--select", "SMP010"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).0, code);
}

#[test]
fn per_file_ignores_flag_needs_a_pattern_and_code() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--per-file-ignores", "module.py"]);
    assert_eq!((code, stderr.contains("Expected <FilePattern>:<RuleCode> pattern")), (2, true));
}

#[test_case(&["--select", "NOPE"], "`NOPE` in `select`" ; "select")]
#[test_case(&["--select", ""], "`` in `select`" ; "empty")]
#[test_case(&["--extend-select", "SMP9"], "`SMP9` in `extend-select`" ; "extend_select")]
#[test_case(&["--ignore", "SMP9"], "`SMP9` in `ignore`" ; "ignore")]
#[test_case(&["--fixable", "SMP9"], "`SMP9` in `fixable`" ; "fixable")]
#[test_case(&["--extend-fixable", "SMP9"], "`SMP9` in `extend-fixable`" ; "extend_fixable")]
#[test_case(&["--unfixable", "SMP9"], "`SMP9` in `unfixable`" ; "unfixable")]
#[test_case(&["--per-file-ignores", "a.py:SMP9"], "`SMP9` in `per-file-ignores`" ; "per_file_ignores")]
#[test_case(&["--config", "extend-unfixable = [\"SMP9\"]"], "`SMP9` in `extend-unfixable`" ; "inline_extend_unfixable")]
#[test_case(&["--config", "extend-safe-fixes = [\"SMP9\"]"], "`SMP9` in `extend-safe-fixes`" ; "inline_extend_safe_fixes")]
#[test_case(&["--config", "extend-unsafe-fixes = [\"SMP9\"]"], "`SMP9` in `extend-unsafe-fixes`" ; "inline_extend_unsafe_fixes")]
#[test_case(&["--config", "extend-per-file-ignores = { \"a.py\" = [\"SMP9\"] }"], "`SMP9` in `extend-per-file-ignores`" ; "inline_extend_per_file_ignores")]
fn check_unknown_selector_names_the_cli(flags: &[&str], selector: &str) {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert_eq!(
        check(root.path(), flags),
        (2, String::new(), format!("simplint failed\n  Cause: Unknown rule selector {selector} from the CLI\n"))
    );
}

#[test_case("F401" ; "code")]
#[test_case("E5" ; "prefix")]
#[test_case("unused-import" ; "name")]
fn check_points_ruff_selectors_to_ruff(selector: &str) {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", selector]),
        (
            2,
            String::new(),
            format!(
                "simplint failed\n  Cause: Unknown rule selector `{selector}` in `select` from the CLI; it selects Ruff \
                 rules, so select it in Ruff's configuration instead\n"
            )
        )
    );
}

#[test_case(&["--add-noqa", "--fix"] ; "add_noqa_fix")]
#[test_case(&["--add-noqa", "--add-ignore"] ; "add_noqa_add_ignore")]
#[test_case(&["--show-files", "--show-settings"] ; "show_files_show_settings")]
#[test_case(&["--statistics", "--watch"] ; "statistics_watch")]
#[test_case(&["--show-settings", "--stdin-filename", "x.py"] ; "show_settings_stdin")]
fn conflicting_flags_fail(flags: &[&str]) {
    let root = project(&[]);
    let (code, _, stderr) = check(root.path(), flags);
    assert_eq!((code, stderr.contains("cannot be used with")), (2, true));
}

#[test_case("py37" ; "py37")]
#[test_case("py38" ; "py38")]
#[test_case("py39" ; "py39")]
#[test_case("py310" ; "py310")]
#[test_case("py311" ; "py311")]
#[test_case("py312" ; "py312")]
#[test_case("py313" ; "py313")]
#[test_case("py314" ; "py314")]
#[test_case("py315" ; "py315")]
fn check_accepts_every_target_version(version: &str) {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert_eq!(check(root.path(), &["--target-version", version, "--select", "SMP010"]).0, 0);
}

#[test]
fn check_syntax_error_is_reported() {
    let root = project(&[("broken.py", "def broken(:\n"), ("module.py", "x = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP010"]).1,
        format!(
            "broken.py:1:12: invalid-syntax: Expected a parameter or the end of the parameter list\nbroken.py:2:1: \
             invalid-syntax: unexpected EOF while parsing\nmodule.py:1:1: {SINGLE_X}\nFound 3 errors.\n"
        )
    );
}

#[cfg(unix)]
#[test]
fn check_unreadable_file_is_reported() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    common::set_mode(&root.path().join("module.py"), 0o000);
    assert_eq!(
        check(root.path(), &[]),
        (1, "module.py:1:1: E902 Permission denied (os error 13)\nFound 1 error.\n".to_owned(), String::new())
    );
}

#[cfg(unix)]
#[test]
fn unreadable_file_is_a_warning_while_ruff_io_error_is_off() {
    let root = project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F\"]\n"), ("module.py", "VALUE = 1\n")]);
    common::set_mode(&root.path().join("module.py"), 0o000);
    let (code, stdout, stderr) = check(root.path(), &[]);
    assert_eq!(
        (code, stdout, stderr.contains("Failed to lint module.py: Permission denied")),
        (0, "All checks passed!\n".to_owned(), true)
    );
}

#[cfg(unix)]
#[test]
fn check_warns_about_unreadable_directories() {
    let root = project(&[("locked/module.py", "x = 1\n"), ("module.py", "VALUE = 1\n")]);
    common::set_mode(&root.path().join("locked"), 0o000);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP010"]);
    common::set_mode(&root.path().join("locked"), 0o755);
    assert_eq!((code, stderr.starts_with("warning: Encountered error:")), (0, true));
}

#[test]
fn check_stops_quietly_when_the_reader_goes_away() {
    // More report than a pipe buffer holds, so the run still writes once the reader is gone however late it goes.
    let root = project(&[("module.py", &"# --- helpers ---\nVALUE = 1\n".repeat(5000))]);
    let mut child = common::command(root.path())
        .args(["check", "--select", "SMP001"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawns");
    drop(child.stdout.take());
    assert_eq!(child.wait().expect("exits").code(), Some(0));
}

#[test]
fn check_verbose_logs_debug_output() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert!(check(root.path(), &["--verbose"]).2.contains("Using Ruff default settings"));
}

#[test]
fn check_reads_stdin() {
    let root = project(&[]);
    assert_eq!(
        check_with_stdin(root.path(), &["--select", "SMP010", "-"], "x = 1\n"),
        (1, format!("-:1:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test_case(
    "--fix", "# --- helpers ---\nVALUE = 1\n", 0, "VALUE = 1\n", "Found 1 error (1 fixed, 0 remaining).\n" ;
    "fix_writes_the_fixed_source"
)]
#[test_case("--fix", "VALUE = 1\n", 0, "VALUE = 1\n", "All checks passed!\n" ; "fix_keeps_source_with_nothing_to_fix")]
#[test_case(
    "--diff",
    "# --- helpers ---\nVALUE = 1\n",
    1,
    "--- module.py\n+++ module.py\n@@ -1,2 +1 @@\n-# --- helpers ---\n VALUE = 1\n\n",
    "Would fix 1 error.\n" ;
    "diff_prints_the_diff"
)]
fn check_fix_from_stdin(flag: &str, stdin: &str, code: i32, stdout: &str, stderr: &str) {
    let root = project(&[]);
    assert_eq!(
        check_with_stdin(root.path(), &[flag, "--select", "SMP001", "--stdin-filename", "module.py"], stdin),
        (code, stdout.to_owned(), stderr.to_owned())
    );
}

#[test]
fn check_stdin_resolves_the_package_of_its_filename() {
    let root = project(&[("pkg/__init__.py", "")]);
    let (code, stdout, _) = check_with_stdin(
        root.path(),
        &["--select", "SMP110", "--stdin-filename", "pkg/module.py"],
        "from pkg.other import VALUE\n",
    );
    assert_eq!(
        (code, stdout.lines().next()),
        (1, Some("pkg/module.py:1:6: SMP110 [*] Import from the same package relatively: `from .other import ...`"))
    );
}

#[test_case(
    &[("pyproject.toml", "[tool.ruff]\ntarget-version = \"py39\"\nper-file-target-version = { \"legacy/*.py\" = \"py314\" }\n")],
    "legacy/model.py",
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Point:\n    x: int\n",
    "SMP111"
    ; "per_file_target_version"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"tool\"\nscripts = { tool = \"pkg.cli:main\" }\n"),
        ("lib/pkg/__init__.py", ""),
    ],
    "lib/pkg/cli.py",
    "__all__ = []\n\n\ndef main() -> None: ...\n",
    "SMP113"
    ; "entry_point_outside_the_source_roots"
)]
fn check_stdin_lints_like_the_file_it_stands_for(files: &[(&str, &str)], name: &str, source: &str, rule: &str) {
    let root = project(&[files, &[(name, source)]].concat());
    let by_path = check(root.path(), &["--select", rule, name]);
    let from_stdin = simplint_with_stdin(
        root.path(),
        &["check", "--output-format", "concise", "--select", rule, "--stdin-filename", &common::native(name)],
        source,
    );
    assert_eq!((from_stdin.0, &from_stdin), (1, &by_path));
}
