// Output that cannot be written fails the run. `/dev/full` refuses every write, as a full disk does.
#![cfg(target_os = "linux")]

mod common;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use test_case::test_case;

use common::{command, project, set_mode, simplint};

const FULL: &str = "No space left on device";

#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

fn on_full_disk(root: &Path, args: &[&str], stream: Stream) -> (i32, String) {
    let full = || Stdio::from(File::create("/dev/full").expect("open /dev/full"));
    let mut command = command(root);
    command.args(args).stdin(Stdio::null());
    let output = match stream {
        Stream::Stdout => command.stdout(full()).output(),
        Stream::Stderr => command.stderr(full()).output(),
    }
    .expect("runs");
    (output.status.code().expect("exit code"), String::from_utf8(output.stderr).expect("utf-8 stderr"))
}

#[test_case(&["rule", "SMP001"] ; "rule")]
#[test_case(&["rule", "--all"] ; "every_rule")]
#[test_case(&["rules"] ; "rules")]
#[test_case(&["config"] ; "config")]
#[test_case(&["linter"] ; "linter")]
#[test_case(&["version"] ; "version")]
#[test_case(&["cli-schema"] ; "cli_schema")]
#[test_case(&["config-schema"] ; "config_schema")]
#[test_case(&["check", "--select", "SMP101"] ; "report")]
#[test_case(&["check", "--select", "SMP101", "--statistics"] ; "statistics")]
#[test_case(&["check", "--select", "SMP101", "--show-files"] ; "show_files")]
#[test_case(&["check", "--select", "SMP101", "--show-settings"] ; "show_settings")]
#[test_case(&["check", "--select", "SMP001", "--diff"] ; "diff")]
#[test_case(&["check", "--select", "SMP101", "--watch"] ; "watch")]
fn a_command_fails_when_standard_output_is_full(args: &[&str]) {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);

    let (code, stderr) = on_full_disk(root.path(), args, Stream::Stdout);

    assert_eq!((code, stderr.contains(FULL)), (2, true), "{stderr}");
}

#[test]
fn check_fails_when_the_output_file_is_full() {
    let root = project(&[("module.py", "x = 1\n")]);

    let (code, stderr) = on_full_disk(root.path(), &["check", "--output-file", "/dev/full"], Stream::Stdout);

    assert_eq!((code, stderr.contains(FULL)), (2, true), "{stderr}");
}

// A report larger than the pipe's buffer blocks until the reader goes away, so closing the pipe once the count is
// out makes the report, not the notices before it, the write that fails. A closed pipe ends the run quietly.
#[test]
fn watch_ends_when_the_report_meets_a_closed_pipe() {
    let root = project(&[("module.py", &"# --- helpers ---\nx = 1\n".repeat(500))]);
    let mut child = command(root.path())
        .args(["check", "--select", "SMP001", "--watch"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("runs");
    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut line = String::new();
    while !line.contains("Watching for file changes") {
        line.clear();
        stdout.read_line(&mut line).expect("read stdout");
    }

    drop(stdout);

    let output = child.wait_with_output().expect("runs");
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert_eq!((output.status.code(), stderr.as_str()), (Some(0), ""));
}

#[test]
fn fixed_standard_input_fails_when_standard_output_is_full() {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut child = command(root.path())
        .args(["check", "--select", "SMP001", "--fix", "-"])
        .stdin(Stdio::piped())
        .stdout(File::create("/dev/full").expect("open /dev/full"))
        .stderr(Stdio::piped())
        .spawn()
        .expect("runs");
    std::io::Write::write_all(&mut child.stdin.take().expect("piped stdin"), b"# --- helpers ---\nVALUE = 1\n")
        .expect("write stdin");

    let output = child.wait_with_output().expect("runs");

    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert_eq!((output.status.code(), stderr.contains(FULL)), (Some(2), true), "{stderr}");
}

#[test]
fn clean_fails_when_standard_error_is_full() {
    let root = project(&[("module.py", "x = 1\n")]);
    simplint(root.path(), &["check", "--select", "SMP101"]);

    let (code, _) = on_full_disk(root.path(), &["clean"], Stream::Stderr);

    assert_eq!((code, root.path().join(".ruff_cache/simplint").is_dir()), (2, true));
}

#[test]
fn clean_fails_when_the_cache_cannot_be_removed() {
    let root = project(&[("module.py", "x = 1\n")]);
    simplint(root.path(), &["check", "--select", "SMP101"]);
    let cache = root.path().join(".ruff_cache");
    set_mode(&cache, 0o555);

    let (code, _, stderr) = simplint(root.path(), &["clean"]);

    set_mode(&cache, 0o755);
    assert_eq!((code, stderr.contains("Permission denied")), (2, true), "{stderr}");
}

#[test]
fn a_panic_ends_the_run_with_the_panic_status() {
    let root = project(&[("module.py", "x = 1\n")]);

    let (code, _) = on_full_disk(root.path(), &["check", "--select", "SMP101", "--add-noqa"], Stream::Stderr);

    assert_eq!(code, 101);
}

// With no process to spare, the worker threads do not start, which fails the run before it reads anything.
#[cfg(target_os = "linux")]
#[test]
fn a_run_without_threads_fails() {
    let root = project(&[("module.py", "x = 1\n")]);

    let output = Command::new("bash")
        .current_dir(root.path())
        .env("SIMPLINT", assert_cmd::cargo::cargo_bin("simplint"))
        .args(["-c", "ulimit -u 1 && exec \"$SIMPLINT\" check"])
        .output()
        .expect("runs");

    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");
    assert_eq!((output.status.code(), stderr.contains("simplint failed")), (Some(2), true), "{stderr}");
}

#[test_case("clean" ; "clean")]
#[test_case("check" ; "check")]
fn a_command_fails_in_a_removed_working_directory(subcommand: &str) {
    let root = project(&[("gone/module.py", "x = 1\n")]);

    let output = Command::new("sh")
        .current_dir(root.path().join("gone"))
        .env("SIMPLINT", assert_cmd::cargo::cargo_bin("simplint"))
        .args(["-c", &format!("rm -r \"$PWD\" && exec \"$SIMPLINT\" {subcommand}")])
        .output();

    let stderr = String::from_utf8(output.expect("runs").stderr).expect("utf-8 stderr");
    assert!(stderr.contains("No such file or directory"), "{stderr}");
}
