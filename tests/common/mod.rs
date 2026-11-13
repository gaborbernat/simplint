#![allow(dead_code, reason = "every test binary compiles this module and uses only the helpers of its own concern")]

use std::fmt::Write as _;
use std::fs;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};
use std::process;

use assert_cmd::Command;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tempfile::TempDir;

pub const SECTION: &str = "SMP001 [*] Section comment groups code; extract a named function or class instead";
pub const SINGLE_X: &str = "SMP101 Single-letter name `x`; use a descriptive name";
pub const PASSED: &str = "All checks passed!\n";
pub const UNPARSED: &str = "warning: Skipped broken.py: it does not parse, so Ruff reports its syntax errors.\n";
pub const ONE_FIXED: &str = "Found 1 error (1 fixed, 0 remaining).\n";
pub const SECTIONED: &str = "# --- helpers ---\nVALUE = 1\n";
pub const CLEAN_MODULE: &str = "VALUE = 1\n";
// Ruff's PLR2004 would report the value inlined into a comparison, so the fix loop withdraws SMP202's fix.
pub const RUFF_PLR2004: (&str, &str) = ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"PLR2004\"]\n");
pub const COMPARED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value > _LIMIT\n";

pub fn project<N: AsRef<Path>>(files: &[(N, &str)]) -> TempDir {
    let root = TempDir::new().expect("temporary directory");
    for (name, content) in files {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
        fs::write(path, content).expect("write file");
    }
    root
}

pub fn empty_project() -> TempDir {
    TempDir::new().expect("temporary directory")
}

pub fn check(root: &Path, args: &[&str]) -> (i32, String, String) {
    check_with_stdin(root, args, "")
}

pub fn check_with_stdin(root: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    simplint_with_stdin(root, &[&["check", "--output-format", "concise"], args].concat(), stdin)
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

pub fn simplint(root: &Path, args: &[&str]) -> (i32, String, String) {
    simplint_with_stdin(root, args, "")
}

pub fn json<T: DeserializeOwned>(root: &Path, args: &[&str]) -> T {
    serde_json::from_str(&simplint(root, args).1).expect("json output")
}

pub fn check_in(root: &Path, cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let output = command(root).current_dir(cwd).arg("check").args(args).output().expect("runs");
    (
        output.status.code().expect("exit code"),
        String::from_utf8(output.stdout).expect("utf-8 stdout"),
        String::from_utf8(output.stderr).expect("utf-8 stderr"),
    )
}

pub fn assert_clean(root: &Path, select: &str) {
    let (_, stdout, stderr) = check(root, &["--select", select]);
    assert_eq!(stdout, PASSED, "{stderr}");
}

pub fn assert_report_count(root: &Path, code: &str, expected: usize) {
    let stdout = check(root, &["--no-cache", "--select", code]).1;
    assert_eq!(reports(&stdout, code).len(), expected, "{stdout}");
}

// The files the concise lines that report `code` name, `/`-separated and sorted, so they read the same on Windows.
pub fn report_paths(stdout: &str, code: &str) -> Vec<String> {
    let mut paths: Vec<String> = reports(stdout, code)
        .into_iter()
        .filter_map(|line| line.split(':').next())
        .map(|path| path.replace(std::path::MAIN_SEPARATOR, "/"))
        .collect();
    paths.sort_unstable();
    paths
}

// The concise lines that report `code`, matched as a whole code so that a message naming it is not counted.
pub fn reports<'a>(stdout: &'a str, code: &str) -> Vec<&'a str> {
    let token = format!(" {code} ");
    stdout.lines().filter(|line| line.contains(&token)).collect()
}

pub fn fix_unsafe(root: &Path, select: &str) -> (i32, String, String) {
    check(root, &["--fix", "--unsafe-fixes", "--select", select])
}

pub fn fix_safe(root: &Path, select: &str) -> (i32, String, String) {
    check(root, &["--no-cache", "--fix", "--select", select])
}

pub fn assert_fix_keeps_report(root: &Path, select: &str, fix: &[&str], file: &str, report: &str) {
    let source = read(root, file);
    let (code, stdout, _) = check(root, &[&["--no-cache", "--select", select], fix].concat());
    assert_eq!((code, stdout.as_str(), read(root, file)), (1, report, source));
}

pub fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).expect("read file")
}

// Ruff takes over the case once its rule is selected, so the case is checked both ways.
pub fn reported_without_and_with_ruff(case: &RuffOverlap, fragment: &str) -> [bool; 2] {
    [false, true].map(|selected| {
        let ruff_select = if selected { format!("\"{}\"", case.ruff_code) } else { String::new() };
        let pyproject = format!("[tool.ruff.lint]\nselect = [{ruff_select}]\n{}", case.settings);
        let mut files = vec![("pyproject.toml", pyproject.as_str())];
        files.extend_from_slice(case.files);
        let root = project(&files);
        check(root.path(), &["--select", case.select]).1.contains(fragment)
    })
}

pub struct RuffOverlap<'a> {
    pub settings: &'a str,
    pub select: &'a str,
    pub ruff_code: &'a str,
    pub files: &'a [(&'a str, &'a str)],
}

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

// Matches how `--show-settings` prints a path.
pub fn quoted(path: &Path) -> String {
    format!("{:?}", path.display().to_string())
}

pub fn fixture_source(path: &str) -> String {
    fs::read_to_string(fixtures().join(path)).expect("fixture")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// The snapshot is named after the rule, then the fixture's suffix, then the target version unless it is Python 3.14. It
// sits in the rule's fixture folder, which mirrors `src/rules`.
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

fn rule_folder(rule_name: &str) -> PathBuf {
    let family = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rules"))
        .expect("read the rule families")
        .map(|entry| entry.expect("rule family entry").path())
        .find(|family| family.join(format!("{rule_name}.rs")).is_file())
        .expect("a rule family holds the rule");
    fixtures().join(family.file_name().expect("a family name")).join(rule_name)
}

fn lint_fixture(rule: &str, fixture: &str, target_version: &str) -> String {
    let root = fixture_project();
    let path = root.path().join(fixture);
    let select = ["check", "--no-cache", "--select", rule, "--target-version", target_version];
    let diagnostics: Vec<Value> = json(root.path(), &[&select[..], &["--output-format", "json"]].concat());
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

// Every fixture is copied so rules that look across modules see the same project. Its configuration matches what the
// rule fixtures assume. Only the Ruff rules that overlap fixtures select run, so the fixes show simplint's own work.
// The rule folders merge into one project root, so each fixture keeps its module name and can import the fixtures of
// other rules, as `consumer/use.py` does.
pub fn fixture_project() -> TempDir {
    let root = TempDir::new().expect("temporary directory");
    let fixtures = fixtures();
    for entry in walkdir::WalkDir::new(&fixtures).min_depth(3) {
        let entry = entry.expect("fixture entry");
        let below_rule: PathBuf =
            entry.path().strip_prefix(&fixtures).expect("below the fixtures").components().skip(2).collect();
        let target = root.path().join(below_rule);
        // Insta deletes a stale `.snap.new` or `.pending-snap` while other tests walk the fixtures, so no snapshot file
        // is copied.
        let name = entry.file_name().to_string_lossy();
        let snapshot = [".snap", ".snap.new", ".pending-snap"].iter().any(|suffix| name.ends_with(suffix));
        if entry.file_type().is_dir() {
            fs::create_dir_all(target).expect("create fixture directory");
        } else if !snapshot {
            fs::copy(entry.path(), target).expect("copy fixture");
        }
    }
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\ndependencies = [\"pytest-mock\", \"requests\"]\n\
         scripts = { run = \"missing_from_all_scripts:run\" }\n\n\
         [tool.simplint]\nproject-kind = \"library\"\ndocstring-style = \"rst\"\n\n\
         [tool.ruff.lint]\nselect = [\"ANN401\", \"COM812\", \"G003\", \"ISC003\", \"N816\", \"PLW0129\"]\n",
    )
    .expect("write pyproject.toml");
    root
}

pub fn text(value: &Value) -> &str {
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
