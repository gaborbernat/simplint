mod common;

use std::fs;

use test_case::test_case;

use common::{
    PASSED, SECTION, SINGLE_X, UNPARSED, check, check_with_stdin, empty_project, project, read, simplint,
    simplint_with_stdin,
};

#[test]
fn check_clean_project_passes() {
    let root = project(&[("module.py", "_VALUE: Final[int] = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP001"]), (0, PASSED.to_owned(), String::new()));
}

#[test]
fn check_reports_violations() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP101"]),
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
        check(root.path(), &["--exit-zero", "--select", "SMP101"]),
        (0, format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test]
fn check_accepts_several_paths() {
    let root = project(&[("first/module.py", "x = 1\n"), ("second/module.py", "y = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101", "first", "second"]).1.lines().last(),
        Some("Found 2 errors.")
    );
}

#[test_case(&["--select", "SMP101"], PASSED ; "check")]
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
    assert_eq!(check(root.path(), &["--select", "SMP101", "script"]).1.lines().last(), Some("Found 1 error."));
}

#[test]
fn show_files_lists_the_files_to_check() {
    let root = project(&[("pkg/b.py", ""), ("a.py", ""), ("stub.pyi", ""), ("notes.txt", "")]);
    let (code, stdout, _) = check(root.path(), &["--show-files"]);
    let (top, nested) = (root.path().join("a.py"), root.path().join("pkg").join("b.py"));
    assert_eq!((code, stdout), (0, format!("{}\n{}\n", top.display(), nested.display())));
}

#[test_case(&["--exclude", "module.py"], 0 ; "exclude")]
#[test_case(&["--extend-exclude", "module.py"], 0 ; "extend_exclude")]
#[test_case(&["--extend-exclude", "module.py", "module.py"], 1 ; "explicit_path_wins")]
#[test_case(&["--extend-exclude", "module.py", "--force-exclude", "module.py"], 0 ; "force_exclude")]
#[test_case(
    &["--extend-exclude", "module.py", "--force-exclude", "--no-force-exclude", "module.py"],
    1
    ; "no_force_exclude"
)]
fn exclusion_flags(flags: &[&str], code: i32) {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut args = vec!["--select", "SMP101"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).0, code);
}

#[test_case(&["--respect-gitignore"], 0 ; "respect")]
#[test_case(&["--no-respect-gitignore"], 1 ; "ignore")]
fn gitignore_flags(flags: &[&str], code: i32) {
    let root = project(&[(".gitignore", "module.py\n"), ("module.py", "x = 1\n")]);
    fs::create_dir(root.path().join(".git")).expect("git directory");
    let mut args = vec!["--select", "SMP101"];
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
        check(root.path(), &["--extend-select", "SMP101"]).1,
        format!("module.py:1:1: {SECTION}\nmodule.py:2:1: {SINGLE_X}\nFound 2 errors.\n[*] 1 fixable with `--fix`.\n")
    );
}

#[test]
fn extend_ignore_removes_rules() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP101", "--extend-ignore", "SMP001"]).1,
        format!("module.py:2:1: {SINGLE_X}\nFound 1 error.\n")
    );
}

#[test_case(&["--select", "SMP1", "--ignore", "SMP101"], 1 ; "specific_ignore")]
#[test_case(&["--select", "SMP001,SMP101", "--ignore", "SMP0"], 2 ; "specific_select")]
fn more_specific_selector_wins(flags: &[&str], reported: usize) {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(check(root.path(), flags).1.matches("module.py:").count(), reported);
}

#[test_case(&["--per-file-ignores", "module.py:SMP101"], 0 ; "basename")]
#[test_case(&["--per-file-ignores", "pkg/*:SMP101"], 0 ; "relative_path")]
#[test_case(&["--per-file-ignores", "./pkg/*:SMP101"], 0 ; "dot_relative_path")]
#[test_case(&["--per-file-ignores", "!other.py:SMP101"], 0 ; "negated")]
#[test_case(&["--per-file-ignores", "other.py:SMP101"], 1 ; "other_file")]
#[test_case(&["--per-file-ignores", "other.py:SMP101", "--extend-per-file-ignores", "module.py:SMP101"], 0 ; "extend")]
#[test_case(&["--per-file-ignores", "other.py:SMP101"], 1 ; "replaces_configuration")]
fn per_file_ignores_flags(flags: &[&str], code: i32) {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"pkg/*\" = [\"SMP101\"] }\n"),
        ("pkg/module.py", "x = 1\n"),
    ]);
    let mut args = vec!["--select", "SMP101"];
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
#[test_case(&["--extend-select", "SMP99"], "`SMP99` in `extend-select`" ; "extend_select")]
#[test_case(&["--ignore", "SMP99"], "`SMP99` in `ignore`" ; "ignore")]
#[test_case(&["--fixable", "SMP99"], "`SMP99` in `fixable`" ; "fixable")]
#[test_case(&["--extend-fixable", "SMP99"], "`SMP99` in `extend-fixable`" ; "extend_fixable")]
#[test_case(&["--unfixable", "SMP99"], "`SMP99` in `unfixable`" ; "unfixable")]
#[test_case(&["--per-file-ignores", "a.py:SMP99"], "`SMP99` in `per-file-ignores`" ; "per_file_ignores")]
#[test_case(
    &["--config", "extend-unfixable = [\"SMP99\"]"],
    "`SMP99` in `extend-unfixable`"
    ; "inline_extend_unfixable"
)]
#[test_case(
    &["--config", "extend-safe-fixes = [\"SMP99\"]"],
    "`SMP99` in `extend-safe-fixes`"
    ; "inline_extend_safe_fixes"
)]
#[test_case(
    &["--config", "extend-unsafe-fixes = [\"SMP99\"]"],
    "`SMP99` in `extend-unsafe-fixes`"
    ; "inline_extend_unsafe_fixes"
)]
#[test_case(
    &["--config", "extend-per-file-ignores = { \"a.py\" = [\"SMP99\"] }"],
    "`SMP99` in `extend-per-file-ignores`" ;
    "inline_extend_per_file_ignores"
)]
fn check_unknown_selector_names_the_cli(flags: &[&str], selector: &str) {
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    assert_eq!(
        check(root.path(), flags),
        (2, String::new(), format!("simplint failed\n  Cause: Unknown rule selector {selector} from the CLI\n"))
    );
}

#[test_case("F401" ; "code")]
#[test_case("E5" ; "prefix")]
#[test_case("unused-import" ; "name")]
fn check_points_ruff_selectors_to_ruff(selector: &str) {
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    assert_eq!(
        check(root.path(), &["--select", selector]),
        (
            2,
            String::new(),
            format!(
                "simplint failed\n  Cause: Unknown rule selector `{selector}` in `select` from the \
                 CLI; it selects Ruff \
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
    let root = empty_project();
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
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    assert_eq!(check(root.path(), &["--target-version", version, "--select", "SMP101"]).0, 0);
}

// Ruff reports the syntax errors of a file that does not parse, so simplint only warns about it and the run goes on.
#[test_case(&["--output-format", "concise"], PASSED ; "concise")]
#[test_case(&["--output-format", "full"], PASSED ; "full")]
#[test_case(&["--output-format", "json"], "[]" ; "json")]
fn unparsed_file_is_skipped_with_a_warning(flags: &[&str], stdout: &str) {
    let root = project(&[("broken.py", "def broken(:\n")]);
    assert_eq!(
        simplint(root.path(), &[&["check", "--select", "SMP101"], flags].concat()),
        (0, stdout.to_owned(), UNPARSED.to_owned())
    );
}

#[test]
fn unparsed_file_beside_a_report_leaves_that_report_and_status_1() {
    let root = project(&[("broken.py", "def broken(:\n"), ("module.py", "x = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101"]),
        (1, format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"), UNPARSED.to_owned())
    );
}

#[test]
fn fix_leaves_an_unparsed_file_as_it_is() {
    let root = project(&[("broken.py", "def broken(:\n")]);
    let (code, stdout, stderr) = check(root.path(), &["--fix", "--select", "SMP101"]);
    assert_eq!((code, stdout, stderr), (0, PASSED.to_owned(), UNPARSED.to_owned()));
    assert_eq!(read(root.path(), "broken.py"), "def broken(:\n");
}

#[test]
fn check_stdin_skips_source_that_does_not_parse() {
    let root = empty_project();
    assert_eq!(
        check_with_stdin(root.path(), &["--select", "SMP101", "--stdin-filename", "broken.py", "-"], "def broken(:\n"),
        (0, PASSED.to_owned(), UNPARSED.to_owned())
    );
}

#[cfg(unix)]
#[test]
fn check_unreadable_file_is_reported() {
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    common::set_mode(&root.path().join("module.py"), 0o000);
    assert_eq!(
        check(root.path(), &[]),
        (1, "module.py:1:1: E902 Permission denied (os error 13)\nFound 1 error.\n".to_owned(), String::new())
    );
}

#[cfg(unix)]
#[test]
fn unreadable_file_is_a_warning_while_ruff_io_error_is_off() {
    let root =
        project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F\"]\n"), ("module.py", common::CLEAN_MODULE)]);
    common::set_mode(&root.path().join("module.py"), 0o000);
    let (code, stdout, stderr) = check(root.path(), &[]);
    assert_eq!(
        (code, stdout, stderr.contains("Failed to lint module.py: Permission denied")),
        (0, PASSED.to_owned(), true)
    );
}

#[cfg(unix)]
#[test]
fn check_warns_about_unreadable_directories() {
    let root = project(&[("locked/module.py", "x = 1\n"), ("module.py", common::CLEAN_MODULE)]);
    common::set_mode(&root.path().join("locked"), 0o000);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    common::set_mode(&root.path().join("locked"), 0o755);
    assert_eq!((code, stderr.starts_with("warning: Encountered error:")), (0, true));
}

#[test]
fn check_stops_quietly_when_the_reader_goes_away() {
    // More report than a pipe buffer holds, so the run still writes once the reader is gone however late it goes.
    let root = project(&[("module.py", &common::SECTIONED.repeat(5000))]);
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
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    assert!(check(root.path(), &["--verbose"]).2.contains("Using Ruff default settings"));
}

#[test]
fn check_reads_stdin() {
    let root = empty_project();
    assert_eq!(
        check_with_stdin(root.path(), &["--select", "SMP101", "-"], "x = 1\n"),
        (1, format!("-:1:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test_case("--fix", common::SECTIONED, (0, common::CLEAN_MODULE, common::ONE_FIXED) ; "fix_writes_the_fixed_source")]
#[test_case("--fix", common::CLEAN_MODULE, (0, common::CLEAN_MODULE, PASSED) ; "fix_keeps_source_with_nothing_to_fix")]
#[test_case(
    "--diff",
    common::SECTIONED,
    (1, "--- module.py\n+++ module.py\n@@ -1,2 +1 @@\n-# --- helpers ---\n VALUE = 1\n\n", "Would fix 1 error.\n") ;
    "diff_prints_the_diff"
)]
fn check_fix_from_stdin(flag: &str, stdin: &str, expected: (i32, &str, &str)) {
    let root = empty_project();
    let (code, stdout, stderr) =
        check_with_stdin(root.path(), &[flag, "--select", "SMP001", "--stdin-filename", "module.py"], stdin);
    assert_eq!((code, stdout.as_str(), stderr.as_str()), expected);
}

#[test]
fn check_stdin_resolves_the_package_of_its_filename() {
    let root = project(&[("pkg/__init__.py", "")]);
    let (code, stdout, _) = check_with_stdin(
        root.path(),
        &["--select", "SMP302", "--stdin-filename", "pkg/module.py"],
        "from pkg.other import VALUE\n",
    );
    assert_eq!(
        (code, stdout.lines().next()),
        (1, Some("pkg/module.py:1:6: SMP302 [*] Import from the same package relatively: `from .other import ...`"))
    );
}

#[test_case(
    &[(
        "pyproject.toml",
        "[tool.ruff]\ntarget-version = \"py39\"\nper-file-target-version = \
         { \"legacy/*.py\" = \"py314\" }\n"
    )],
    "legacy/model.py",
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Point:\n    x: int\n",
    "SMP701"
    ; "per_file_target_version"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"tool\"\nscripts = { tool = \"pkg.cli:main\" }\n"),
        ("lib/pkg/__init__.py", ""),
    ],
    "lib/pkg/cli.py",
    "__all__ = []\n\n\ndef main() -> None: ...\n",
    "SMP401"
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

// Standard input is judged by the path it stands for, so a helper read from a test directory is a test module's and
// its SMP409 is left out, as it is when the file is checked by path.
#[test_case("tests", PASSED ; "test_directory")]
#[test_case(
    "lib",
    "helpers.py:1:5: SMP409 `helper` is never used in the run; remove it or give it a caller\nFound 1 error.\n" ;
    "other_directory"
)]
fn check_stdin_judges_its_test_module_by_the_directory_it_stands_in(dir: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n[tool.simplint]\n"),
        (&format!("{dir}/helpers.py"), "def helper() -> None:\n    pass\n"),
    ]);
    let (_, from_stdin, _) = simplint_with_stdin(
        &root.path().join(dir),
        &["check", "--output-format", "concise", "--select", "SMP409", "--stdin-filename", "helpers.py"],
        "def helper() -> None:\n    pass\n",
    );
    assert_eq!(from_stdin, stdout);
}

// Ruff reports a path that does not exist as `E902`, which fails the run, as it reports an unreadable file.
#[test]
fn check_reports_a_missing_path_as_io_error() {
    let root = project(&[("real.py", common::CLEAN_MODULE)]);
    let (code, stdout, _) = check(root.path(), &["--select", "SMP101", "typo.py", "real.py"]);
    assert_eq!((code, stdout), (1, format!("typo.py:1:1: E902 {}\nFound 1 error.\n", missing_file_message())));
}

// The operating system words the error, so Windows and Unix report the same missing file differently.
fn missing_file_message() -> String {
    std::io::Error::from_raw_os_error(2).to_string()
}

// With Ruff's `E902` off, a missing path stays a warning and the run keeps its status, as it always has.
#[test]
fn check_warns_about_a_missing_path_while_io_error_is_off() {
    let root =
        project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F\"]\n"), ("real.py", common::CLEAN_MODULE)]);
    let (code, stdout, stderr) = check(root.path(), &["--select", "SMP101", "typo.py", "real.py"]);
    assert_eq!(
        (code, stdout, stderr),
        (0, PASSED.to_owned(), format!("warning: Encountered error: {}\n", missing_file_message()))
    );
}
