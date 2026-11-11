mod common;

#[cfg(unix)]
use std::fs;

use test_case::test_case;

use common::{check, check_with_stdin, empty_project, project, read};

// Nesting this deep overflowed the stack of the parser and of every visitor before the work got a larger one.
const DEPTH: usize = 10_000;

#[test_case("[", "]" ; "nested_lists")]
#[test_case("-", "" ; "unary_minus")]
fn deeply_nested_expression_does_not_crash(open: &str, close: &str) {
    let source = format!("value = {}1{}\n", open.repeat(DEPTH), close.repeat(DEPTH));
    let root = project(&[("module.py", source.as_str())]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

#[test]
fn long_operator_chain_does_not_crash() {
    let source = format!("value = {}\n", vec!["1"; DEPTH].join(" + "));
    let root = project(&[("module.py", source.as_str())]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

// A link to a file outside the run is skipped: `--fix` must not write through it, and the report must not show the
// target's text.
#[cfg(unix)]
#[test_case(&["--fix", "--select", "SMP001"] ; "fix")]
#[test_case(&["--select", "SMP001"] ; "report")]
fn link_to_a_file_outside_the_run_is_skipped(flags: &[&str]) {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let target = outside.path().join("secret.py");
    fs::write(&target, common::SECTIONED).expect("write");
    let root = project(&[("module.py", common::CLEAN_MODULE)]);
    std::os::unix::fs::symlink(&target, root.path().join("link.py")).expect("symlink");
    let (code, stdout, stderr) = check(root.path(), flags);
    assert_eq!(
        (code, stdout.contains("helpers"), stderr.contains("Skipped symlink"), stderr.contains("link.py")),
        (0, false, true, true)
    );
    assert_eq!(read(outside.path(), "secret.py"), common::SECTIONED);
}

// A link whose target is in the run is linted as any file is, unless the run also finds the target, which is then linted
// once, by its own path.
#[cfg(unix)]
#[test_case(&[] , (true, false) ; "target_found")]
#[test_case(&["link.py"], (false, true) ; "link_named")]
fn link_inside_the_run_is_linted_once(paths: &[&str], reported: (bool, bool)) {
    let root = project(&[("module.py", "x = 1\n")]);
    std::os::unix::fs::symlink(root.path().join("module.py"), root.path().join("link.py")).expect("symlink");
    let (_, stdout, stderr) = check(root.path(), &[&["--select", "SMP101"], paths].concat());
    assert_eq!(
        (stdout.contains("module.py:1:1"), stdout.contains("link.py:1:1"), stderr.is_empty()),
        (reported.0, reported.1, true)
    );
}

// Files are fixed in parallel, so a file fixed through its link as well would be written twice, and the second write
// would fail.
#[cfg(unix)]
#[test]
fn file_reached_through_a_link_is_fixed_once() {
    let root = project(&[("module.py", common::SECTIONED)]);
    std::os::unix::fs::symlink("module.py", root.path().join("link.py")).expect("symlink");
    let (code, stdout, _) = check(root.path(), &["--fix", "--select", "SMP001"]);
    assert_eq!(
        (code, stdout.contains("Found 1 error (1 fixed, 0 remaining)"), read(root.path(), "module.py")),
        (0, true, common::CLEAN_MODULE.to_owned())
    );
}

// Hard links share one file, so the run lints it once: under the smallest path in sort order, with symlinks last, rather
// than under each name in parallel, where a fix would race on the file and each report would show once per name.
#[cfg(unix)]
#[test]
fn hard_links_to_one_file_are_linted_once() {
    let root = project(&[("module.py", "VALUE = vars(None)\n"), ("other.py", "VALUE = vars(None)\n")]);
    fs::hard_link(root.path().join("module.py"), root.path().join("link.py")).expect("hard link");
    let (_, stdout, _) = check(root.path(), &["--select", "SMP604"]);
    assert_eq!(stdout.matches("SMP604").count(), 2);
}

// Standard input names no file, so `--add-noqa` and `--add-ignore` have nothing to edit and must not edit the files
// under the working directory instead.
#[test_case("--add-noqa" ; "noqa")]
#[test_case("--add-ignore" ; "ignore")]
fn suppression_refuses_standard_input(flag: &str) {
    let source = "def run(x):\n    return x\n";
    let root = project(&[("module.py", source)]);
    let (code, _, stderr) = check_with_stdin(root.path(), &["--select", "SMP101", flag, "-"], common::CLEAN_MODULE);
    assert_eq!(
        (code, stderr.contains(&format!("{flag} cannot be used with standard input")), read(root.path(), "module.py")),
        (2, true, source.to_owned())
    );
}

// `--stdin-filename` reads standard input whatever paths come with it, and reports it under that name.
#[test]
fn stdin_filename_reads_standard_input_over_the_paths() {
    let root = project(&[("on_disk.py", "def zed(q):\n    return q\n")]);
    let (code, stdout, stderr) = check_with_stdin(
        root.path(),
        &["--select", "SMP101", "--stdin-filename", "x.py", "on_disk.py"],
        "def other(w):\n    return w\n",
    );
    assert_eq!(
        (code, stdout, stderr),
        (
            1,
            "x.py:1:11: SMP101 Single-letter name `w`; use a descriptive name\nFound 1 error.\n".to_owned(),
            "warning: Ignoring file on_disk.py in favor of standard input.\n".to_owned()
        )
    );
}

// Standard input is read only for a lone `-`, so a `-` beside other paths leaves the run to lint those paths.
#[test]
fn dash_beside_paths_is_not_standard_input() {
    let root = project(&[("on_disk.py", "def zed(q):\n    return q\n")]);
    let (_, stdout, _) =
        check_with_stdin(root.path(), &["--select", "SMP101", "-", "on_disk.py"], "def other(w):\n    return w\n");
    assert_eq!((stdout.contains("`q`"), stdout.contains("`w`")), (true, false));
}

// Standard input is filtered as the file of its name on disk is: under `--force-exclude` an excluded name, and a
// notebook, TOML or Markdown name, report nothing. Without `--force-exclude` an excluded name is linted, as on disk.
#[test_case(&["--exclude", "module.py", "--force-exclude"], "module.py", (0, false) ; "excluded_under_force_exclude")]
#[test_case(&["--exclude", "module.py"], "module.py", (1, true) ; "excluded_without_force_exclude")]
#[test_case(&[], "module.ipynb", (0, false) ; "notebook")]
#[test_case(&[], "pyproject.toml", (0, false) ; "toml")]
#[test_case(&[], "module.md", (0, false) ; "markdown")]
#[test_case(&[], "module.py", (1, true) ; "python")]
fn stdin_name_the_file_filters_leave_out_reports_nothing(flags: &[&str], name: &str, expected: (i32, bool)) {
    let root = empty_project();
    let args = [&["--select", "SMP101", "--stdin-filename", name], flags, &["-"]].concat();
    let (code, stdout, _) = check_with_stdin(root.path(), &args, "def run(x):\n    return x\n");
    assert_eq!((code, stdout.contains("SMP101")), expected);
}

// Ruff's `--fix` writes an excluded standard input back as it came, so a pipeline keeps the source.
#[test]
fn fix_writes_back_standard_input_excluded_by_force_exclude() {
    let source = "def run(items):\n    count = len(items)\n    if count == 0:\n        return 1\n    return 2\n";
    let root = empty_project();
    let args = [
        "--fix",
        "--select",
        "SMP502",
        "--exclude",
        "module.py",
        "--force-exclude",
        "--stdin-filename",
        "module.py",
        "-",
    ];
    let (code, stdout, _) = check_with_stdin(root.path(), &args, source);
    assert_eq!((code, stdout), (0, source.to_owned()));
}
