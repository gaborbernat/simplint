mod common;

use std::fs;

use common::{SINGLE_X, Watched, project};
use test_case::test_case;

#[test]
fn check_watch_reruns_on_changes() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut watched = Watched::start(root.path(), &[]);
    let first = watched.wait_for("Watching for file changes.");
    fs::write(root.path().join("other.py"), "y = 1\n").expect("write file");
    watched.wait_for("File change detected...");
    // A write can be seen before it completes, so the check that counts both files may not be the first one.
    let second = watched.wait_for("Found 2 errors.");
    fs::write(root.path().join("pyproject.toml"), "[tool.simplint]\nper-file-ignores = { \"*.py\" = [\"SMP101\"] }\n")
        .expect("write configuration");
    let third = watched.wait_for("Found 0 errors.");
    let (code, stderr) = watched.stop(root.path());
    assert_eq!(
        (
            first.contains("Found 1 error."),
            second.contains("Watching"),
            third.contains("Watching"),
            code,
            stderr.contains("simplint failed")
        ),
        (true, true, true, Some(2), true)
    );
}

// A stub declares the API of its module, so editing one checks again even though stubs are not linted.
#[test]
fn check_watch_reruns_on_stub_changes() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut watched = Watched::start(root.path(), &[]);
    watched.wait_for("Watching for file changes.");
    fs::write(root.path().join("module.pyi"), "x: int\n").expect("write stub");
    let rerun = watched.wait_for("Found");
    assert_eq!((rerun.contains("Found 1 error."), watched.stop(root.path()).0), (true, Some(2)));
}

// A hook configuration that turns the rule off changes the settings, so the run resolves them again.
#[test_case(".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8-lazy\n" ; "yaml")]
#[test_case(".pre-commit-config.yml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8-lazy\n" ; "yml")]
#[test_case("prek.toml", "[[repos]]\nrepo = \"x\"\nhooks = [{ id = \"flake8-lazy\" }]\n" ; "prek")]
fn check_watch_reruns_when_a_hook_configuration_turns_the_rule_off(name: &str, content: &str) {
    let root = project(&[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/sub/sibling.py", common::CLEAN_MODULE),
        ("package/sub/module.py", "from package.sub import sibling\n"),
    ]);
    let mut watched = Watched::start(root.path(), &["--select", "absolute-sibling-import"]);
    let first = watched.wait_for("Found 1 error.");
    fs::write(root.path().join(name), content).expect("write hook configuration");
    watched.wait_for("File change detected...");
    let second = watched.wait_for("Found 0 errors.");
    let (code, _) = watched.stop(root.path());
    assert_eq!((first.contains("Found 1 error."), second.contains("Found 0 errors."), code), (true, true, Some(2)));
}

// Opening a module, the reads and cache writes of each check, and files that are neither Python nor TOML start no
// check. The broken configuration arrives whole by a rename, so the run ends at the first event that counts.
#[test]
fn check_watch_ignores_unrelated_changes() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut watched = Watched::start(root.path(), &[]);
    watched.wait_for("Watching for file changes.");
    fs::write(root.path().join("notes.txt"), "y = 1\n").expect("write file");
    fs::File::options().write(true).open(root.path().join("module.py")).expect("open for writing");
    fs::write(root.path().join("broken.txt"), "[tool.simplint\n").expect("write configuration");
    fs::rename(root.path().join("broken.txt"), root.path().join("pyproject.toml")).expect("rename");
    let (code, rest) = watched.finish();
    assert_eq!((code, rest.iter().any(|line| line.contains("File change detected"))), (Some(2), false));
}

#[test]
fn check_watch_quiet_prints_only_diagnostics() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut watched = Watched::start(root.path(), &["--quiet"]);
    // The screen is cleared before each check, by escape codes or, on Windows without a console, a form feed.
    let clear = if cfg!(windows) { "\u{c}" } else { "\u{1b}[H\u{1b}[2J\u{1b}[3J" };
    let first = watched.wait_for("SMP101");
    assert_eq!((first, watched.stop(root.path()).0), (format!("{clear}module.py:1:1: {SINGLE_X}"), Some(2)));
}
