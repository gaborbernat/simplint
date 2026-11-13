mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use test_case::test_case;

use common::{SINGLE_X, command, project};

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

// A nested configuration is read per file during the check rather than when the run resolves its settings, so a
// broken one ends the run from inside the check.
#[test]
fn check_watch_ends_when_a_nested_configuration_breaks() {
    let root = project(&[("module.py", "x = 1\n"), ("sub/other.py", "y = 1\n")]);
    let mut watched = Watched::start(root.path(), &[]);
    watched.wait_for("Watching for file changes.");
    fs::write(root.path().join("broken.txt"), "[tool.simplint\n").expect("write configuration");
    fs::rename(root.path().join("broken.txt"), root.path().join("sub/pyproject.toml")).expect("rename");
    let (code, rest) = watched.finish();
    assert_eq!((code, rest.iter().any(|line| line.contains("File change detected"))), (Some(2), true));
}

#[test]
fn check_watch_quiet_prints_only_diagnostics() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut watched = Watched::start(root.path(), &["--quiet"]);
    let first = watched.wait_for("SMP101");
    assert_eq!((first, watched.stop(root.path()).0), (format!("{}module.py:1:1: {SINGLE_X}", clear_screen()), Some(2)));
}

#[test]
fn check_watch_fails_on_a_path_it_cannot_watch() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = common::simplint(root.path(), &["check", "--watch", "missing.py"]);
    assert_eq!((code, stderr.contains("simplint failed")), (2, true), "{stderr}");
}

// A `--watch` run, read line by line; a run that misses an expected line within the deadline is killed.
struct Watched {
    child: process::Child,
    lines: mpsc::Receiver<String>,
}

impl Watched {
    fn start(root: &Path, flags: &[&str]) -> Self {
        let mut child = command(root)
            .args(["check", "--watch", "--output-format", "concise", "--select", "SMP101"])
            .args(flags)
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .expect("spawns");
        let (sender, lines) = mpsc::channel();
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        std::thread::spawn(move || stdout.lines().map_while(Result::ok).try_for_each(|line| sender.send(line)));
        Self { child, lines }
    }

    fn wait_for(&mut self, needle: &str) -> String {
        loop {
            match self.lines.recv_timeout(Duration::from_secs(60)) {
                Ok(line) if line.contains(needle) => return line,
                Ok(_) => {}
                Err(error) => {
                    self.child.kill().ok();
                    panic!("no `{needle}` in the watch output: {error}");
                }
            }
        }
    }

    // Watch mode runs until a check cannot run, so a configuration simplint cannot read ends it. It arrives whole by a
    // rename, as a run that read it half written would fail at a different step on each run.
    fn stop(mut self, root: &Path) -> (Option<i32>, String) {
        fs::write(root.join("broken.txt"), "[tool.simplint\n").expect("write configuration");
        fs::rename(root.join("broken.txt"), root.join("pyproject.toml")).expect("rename");
        self.drain();
        let output = self.child.wait_with_output().expect("exits");
        (output.status.code(), String::from_utf8(output.stderr).expect("utf-8 stderr"))
    }

    // The output closes when the run ends, so a run still going at the deadline missed the event that should end it.
    fn drain(&mut self) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut rest = Vec::new();
        loop {
            match self.lines.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(line) => rest.push(line),
                Err(RecvTimeoutError::Disconnected) => return rest,
                Err(RecvTimeoutError::Timeout) => {
                    self.child.kill().ok();
                    panic!("the watch run did not end");
                }
            }
        }
    }

    fn finish(mut self) -> (Option<i32>, Vec<String>) {
        let rest = self.drain();
        (self.child.wait_with_output().expect("exits").status.code(), rest)
    }
}

// The screen is cleared before each check, by escape codes or, on Windows without a console, a form feed.
fn clear_screen() -> &'static str {
    if cfg!(windows) { "\u{c}" } else { "\u{1b}[H\u{1b}[2J\u{1b}[3J" }
}
