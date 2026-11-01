#![allow(dead_code, reason = "every test binary compiles this module and uses only the helpers of its own concern")]

use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};
use std::process;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

pub const SECTION: &str = "SMP001 [*] Section comment groups code; extract a named function or class instead";
pub const SINGLE_X: &str = "SMP010 Single-letter name `x`; only loop targets and comprehensions may use one";

pub fn project(files: &[(&str, &str)]) -> TempDir {
    let root = TempDir::new().expect("temporary directory");
    for (name, content) in files {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
        fs::write(path, content).expect("write file");
    }
    root
}

pub fn check(root: &Path, args: &[&str]) -> (i32, String, String) {
    check_with_stdin(root, args, "")
}

pub fn check_with_stdin(root: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    simplint_with_stdin(root, &[&["check", "--output-format", "concise"], args].concat(), stdin)
}

// A file the run can read but not write, marked the way each platform marks one.
pub fn make_read_only(path: &Path) {
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).expect("make read-only");
}

// Windows denies reading only through access control lists, so the tests of unreadable files run on Unix.
#[cfg(unix)]
pub fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod");
}

// A relative path written with `/`, as reports name it on this platform.
pub fn native(path: &str) -> String {
    path.replace('/', MAIN_SEPARATOR_STR)
}

// Windows canonicalizes to a verbatim `\\?\` path, which simplint shows without the prefix, as Ruff does.
pub fn canonical(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).expect("canonical path");
    canonical.to_str().and_then(|text| text.strip_prefix(r"\\?\")).map_or_else(|| canonical.clone(), PathBuf::from)
}

// A path as `--show-settings` prints it: quoted, with backslashes escaped.
pub fn quoted(path: &Path) -> String {
    format!("{:?}", path.display().to_string())
}

pub fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).expect("read file")
}

pub fn simplint(root: &Path, args: &[&str]) -> (i32, String, String) {
    simplint_with_stdin(root, args, "")
}

pub fn simplint_with_stdin(root: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    simplint_with(root, args, stdin, &[])
}

pub fn simplint_with(root: &Path, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> (i32, String, String) {
    let output = Command::from_std(command(root))
        .envs(env.iter().copied())
        .args(args)
        .write_stdin(stdin)
        .output()
        .expect("runs");
    (
        output.status.code().expect("exit code"),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

// The user's own Ruff configuration and simplint's environment variables would leak into every test, so neither is
// visible to the binary.
pub fn command(root: &Path) -> process::Command {
    let mut command = process::Command::new(assert_cmd::cargo::cargo_bin("simplint"));
    command
        .current_dir(root)
        .env("XDG_CONFIG_HOME", root.join(".config"))
        .env("NO_COLOR", "1")
        .env_remove("SIMPLINT_OUTPUT_FORMAT")
        .env_remove("SIMPLINT_OUTPUT_FILE")
        .env_remove("SIMPLINT_NO_CACHE")
        .env_remove("RUFF_CACHE_DIR")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE");
    command
}

// A rule's reports on one fixture as `line:col CODE message`, with the fix's safety and title, then, when any report has
// a fix, the fixture after `--fix --unsafe-fixes`. The snapshot is named after the rule, then the fixture, of which a
// fixture named after the rule adds only its suffix, then any target version other than Python 3.14. It sits in the
// rule's folder, `tests/fixtures/<family>/<rule_name>/`, which mirrors `src/rules/<family>/<rule_name>.rs`.
pub fn assert_rule_snapshot(rule: &str, fixture: &str, target_version: &str) {
    let rule_name = rule.replace('-', "_");
    let stem = fixture.trim_end_matches(".py");
    let file = stem.rsplit('/').next().unwrap_or(stem);
    let variant = file
        .split_once(&rule_name)
        .map_or_else(|| format!("_{}", stem.replace('/', "_")), |(_, suffix)| suffix.to_owned());
    let version = if target_version == "py314" { String::new() } else { format!("_{target_version}") };
    let name = format!("{rule_name}{variant}{version}");
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(rule_folder(&rule_name));
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| insta::assert_snapshot!(name, lint_fixture(rule, fixture, target_version)));
}

fn lint_fixture(rule: &str, fixture: &str, target_version: &str) -> String {
    let root = fixture_project();
    let path = root.path().join(fixture);
    let select = ["check", "--no-cache", "--select", rule, "--target-version", target_version];
    let report = simplint(root.path(), &[&select[..], &["--output-format", "json"]].concat()).1;
    let diagnostics: Vec<Value> = serde_json::from_str(&report).expect("json report");
    let diagnostics: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| Path::new(text(&diagnostic["filename"])) == path).collect();
    let mut output = String::new();
    for diagnostic in &diagnostics {
        let title = &diagnostic["fix"]["message"];
        let fix = match diagnostic["fix"]["applicability"].as_str() {
            Some("safe") => format!(" [fix: {}]", text(title)),
            Some(_) => format!(" [unsafe fix: {}]", text(title)),
            None => String::new(),
        };
        let location = &diagnostic["location"];
        let (code, message) = (text(&diagnostic["code"]), text(&diagnostic["message"]));
        writeln!(output, "{}:{} {code} {message}{fix}", location["row"], location["column"]).expect("write");
    }
    if diagnostics.iter().any(|diagnostic| !diagnostic["fix"].is_null()) {
        // The run fixes every fixture of the copy, so only the summary lines of this one count.
        let fix = ["--fix", "--unsafe-fixes", "--show-fixes", "--output-format", "concise"];
        let summary = simplint(root.path(), &[&select[..], &fix[..]].concat()).1;
        let remaining = summary.lines().filter(|line| line.starts_with(&format!("{}:", native(fixture)))).count();
        let fixed = read(root.path(), fixture);
        write!(output, "--- fixed ({} applied, {remaining} remaining) ---\n{fixed}", fixed_count(&summary, fixture))
            .expect("write");
    }
    output
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn rule_folder(rule_name: &str) -> PathBuf {
    let family = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules"))
        .expect("read the rule families")
        .map(|entry| entry.expect("rule family entry").path())
        .find(|family| family.join(format!("{rule_name}.rs")).is_file())
        .expect("a rule family holds the rule");
    fixtures().join(family.file_name().expect("a family name")).join(rule_name)
}

// Every fixture, so rules that look across modules see the same project each time, with the configuration the rule
// fixtures assume: `requests` as a dependency, an entry point, and no Ruff rule, so the fixes show simplint's own work.
// The rule folders merge into one project root, so each fixture keeps its module name and can import the fixtures of
// other rules, as `consumer/use.py` does.
fn fixture_project() -> TempDir {
    let root = TempDir::new().expect("temporary directory");
    let fixtures = fixtures();
    // The first two levels are `<family>/<rule_name>`.
    for entry in walkdir::WalkDir::new(&fixtures).min_depth(3) {
        let entry = entry.expect("fixture entry");
        let below_rule: PathBuf =
            entry.path().strip_prefix(&fixtures).expect("below the fixtures").components().skip(2).collect();
        let target = root.path().join(below_rule);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target).expect("create fixture directory");
        } else if entry.path().extension().is_none_or(|extension| extension != "snap") {
            fs::copy(entry.path(), target).expect("copy fixture");
        }
    }
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\ndependencies = [\"requests\"]\nscripts = { run = \"missing_from_all_scripts:run\" }\n\n[tool.ruff.lint]\n\
         select = []\n",
    )
    .expect("write pyproject.toml");
    root
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}

// The `--show-fixes` summary lists each file as `- <file>:`, then one `<count> × <rule>` line per rule.
fn fixed_count(summary: &str, fixture: &str) -> usize {
    let header = format!("- {}:", native(fixture));
    summary
        .lines()
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| line.starts_with("    "))
        .map(|line| line.trim_start().split_once(' ').expect("a count").0.parse::<usize>().expect("a number"))
        .sum()
}

// A `--watch` run, read line by line; a run that misses an expected line within the deadline is killed.
pub struct Watched {
    child: process::Child,
    lines: mpsc::Receiver<String>,
}

impl Watched {
    pub fn start(root: &Path, flags: &[&str]) -> Self {
        let mut child = command(root)
            .args(["check", "--watch", "--output-format", "concise", "--select", "SMP010"])
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

    pub fn wait_for(&mut self, needle: &str) -> String {
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

    // Watch mode runs until a check cannot run, so a configuration simplint cannot read ends it.
    pub fn stop(mut self, root: &Path) -> (Option<i32>, String) {
        fs::write(root.join("pyproject.toml"), "[tool.simplint\n").expect("write configuration");
        self.drain();
        let output = self.child.wait_with_output().expect("exits");
        (output.status.code(), String::from_utf8(output.stderr).expect("utf-8 stderr"))
    }

    // The exit code and the output not yet read of a run that ends on its own.
    pub fn finish(mut self) -> (Option<i32>, Vec<String>) {
        let rest = self.drain();
        (self.child.wait_with_output().expect("exits").status.code(), rest)
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
}
