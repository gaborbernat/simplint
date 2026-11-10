mod common;

#[cfg(unix)]
use std::fs;

use test_case::test_case;

#[cfg(unix)]
use common::read;
use common::{check, project};

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
    fs::write(&target, "# --- helpers ---\nVALUE = 1\n").expect("write");
    let root = project(&[("module.py", "VALUE = 1\n")]);
    std::os::unix::fs::symlink(&target, root.path().join("link.py")).expect("symlink");
    let (code, stdout, stderr) = check(root.path(), flags);
    assert_eq!(
        (code, stdout.contains("helpers"), stderr.contains("Skipped symlink"), stderr.contains("link.py")),
        (0, false, true, true)
    );
    assert_eq!(read(outside.path(), "secret.py"), "# --- helpers ---\nVALUE = 1\n");
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
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    std::os::unix::fs::symlink("module.py", root.path().join("link.py")).expect("symlink");
    let (code, stdout, _) = check(root.path(), &["--fix", "--select", "SMP001"]);
    assert_eq!(
        (code, stdout.contains("Found 1 error (1 fixed, 0 remaining)"), read(root.path(), "module.py")),
        (0, true, "VALUE = 1\n".to_owned())
    );
}
