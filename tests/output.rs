mod common;

use test_case::test_case;

use common::{SINGLE_X, check, project, read, simplint, simplint_with};

// Each report as the format renders it, one with a fix and one without, with the project's temporary directory as
// `[ROOT]`.
#[test_case("concise" ; "concise")]
#[test_case("full" ; "full")]
#[test_case("json" ; "json")]
#[test_case("json-lines" ; "json_lines")]
#[test_case("junit" ; "junit")]
#[test_case("grouped" ; "grouped")]
#[test_case("github" ; "github")]
#[test_case("gitlab" ; "gitlab")]
#[test_case("pylint" ; "pylint")]
#[test_case("azure" ; "azure")]
#[test_case("rdjson" ; "rdjson")]
#[test_case("sarif" ; "sarif")]
fn check_renders_every_output_format(format: &str) {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    let (code, stdout, _) = simplint(root.path(), &["check", "--output-format", format, "--select", "SMP001,SMP010"]);
    // SARIF names the simplint version, which each release changes. The root appears as a path, as a JSON string
    // with escaped backslashes, and in SARIF as a file URL, and Windows separates what follows it with backslashes.
    let root_path = root.path().display().to_string();
    let report = stdout
        .replace(&format!("/{}", root_path.replace('\\', "/").trim_start_matches('/')), "[ROOT]")
        .replace(&root_path.replace('\\', "\\\\"), "[ROOT]")
        .replace(&root_path, "[ROOT]")
        .replace("[ROOT]\\\\", "[ROOT]/")
        .replace("[ROOT]\\", "[ROOT]/")
        .replace(&format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION")), "\"version\": \"[VERSION]\"");
    insta::assert_snapshot!(format!("format_{format}"), format!("exit code {code}\n{report}"));
}

// A `noqa` comment for a report on the first line of a multi-line string goes on the string's last line.
#[test_case(&[] ; "suppressions_read")]
#[test_case(&["--ignore-noqa"] ; "suppressions_ignored")]
fn check_json_gives_the_row_a_noqa_comment_goes_on(flags: &[&str]) {
    let root = project(&[("module.py", "x = \"\"\"a\nb\"\"\"\n")]);
    let stdout =
        simplint(root.path(), &[&["check", "--output-format", "json", "--select", "SMP010"], flags].concat()).1;
    let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("json report");
    assert_eq!(diagnostics[0]["noqa_row"], 2);
}

#[test]
fn check_output_file_receives_the_report() {
    let root = project(&[("module.py", "x = 1\n")]);
    let result = check(root.path(), &["--select", "SMP010", "--output-file", "out/report.txt"]);
    assert_eq!(
        (result, read(root.path(), "out/report.txt")),
        ((1, String::new(), String::new()), format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"))
    );
}

#[test]
fn output_file_is_never_colored() {
    let root = project(&[("module.py", "x = 1\n")]);
    simplint(root.path(), &["check", "--color", "always", "--select", "SMP010", "--output-file", "report.txt"]);
    assert!(!read(root.path(), "report.txt").contains('\u{1b}'));
}

#[test_case(&["--quiet"], "module.py:1:1: SMP010 Single-letter name `x`; only loop targets and comprehensions may use one\n" ; "quiet")]
#[test_case(&["--silent"], "" ; "silent")]
fn check_log_levels_trim_output(flags: &[&str], stdout: &str) {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut args = vec!["--select", "SMP010"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args), (1, stdout.to_owned(), String::new()));
}

#[test_case("concise", "1\tSMP010\tsingle-letter-name\nFound 1 error.\n" ; "text")]
#[test_case(
    "json",
    "[\n  {\n    \"code\": \"SMP010\",\n    \"name\": \"single-letter-name\",\n    \"count\": 1,\n    \"fixable\": false,\n    \"fixable_count\": 0\n  }\n]\n" ;
    "json"
)]
fn check_statistics_counts_each_rule(format: &str, stdout: &str) {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(
        simplint(root.path(), &["check", "--statistics", "--output-format", format, "--select", "SMP010"]).1,
        stdout
    );
}

#[test]
fn check_statistics_rejects_other_formats() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = simplint(root.path(), &["check", "--statistics", "--output-format", "github"]);
    assert_eq!((code, stderr.contains("Unsupported serialization format for statistics")), (2, true));
}

#[test]
fn check_statistics_marks_partly_fixable_rules() {
    let root = project(&[("module.py", "# --- helpers ---\n# --- more ---  # noqa: SMP001\n")]);
    assert_eq!(
        check(root.path(), &["--statistics", "--select", "SMP001", "--ignore-noqa"]).1,
        "2\tSMP001\t[*] section-comment\nFound 2 errors.\n[*] 2 fixable with `--fix`.\n"
    );
}

#[test]
fn check_statistics_of_a_clean_project_prints_nothing() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert_eq!(check(root.path(), &["--statistics", "--select", "SMP010"]), (0, String::new(), String::new()));
}

#[test]
fn check_statistics_mark_partly_and_not_fixable_rules() {
    let root = project(&[(
        "module.py",
        "# --- helpers ---\nx = 1\n\n\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n\n\ndef stop() -> None:\n    \"\"\"Stop.\"\"\";  # kept\n    print()\n",
    )]);
    assert_eq!(
        check(root.path(), &["--statistics", "--unsafe-fixes", "--select", "SMP001,SMP002,SMP010"]).1,
        "2\tSMP002\t[-] function-docstring\n1\tSMP001\t[*] section-comment\n1\tSMP010\t[ ] single-letter-name\n\
         Found 4 errors.\n[*] 2 fixable with `--fix`.\n"
    );
}

#[test]
fn check_forced_colors_style_the_output() {
    let root = project(&[("module.py", "x = 1\n")]);
    let output = common::command(root.path())
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .args(["check", "--statistics", "--select", "SMP010"])
        .output()
        .expect("runs");
    assert!(String::from_utf8(output.stdout).expect("utf-8 stdout").contains('\u{1b}'));
}

#[test_case(&["--color", "always"], &[], true ; "always")]
#[test_case(&["--color", "never"], &[("FORCE_COLOR", "1")], false ; "never_beats_force_color")]
#[test_case(&["--color", "auto"], &[("FORCE_COLOR", "1")], true ; "force_color")]
#[test_case(&[], &[("FORCE_COLOR", "")], false ; "empty_force_color")]
fn color_follows_the_flag_then_force_color(flags: &[&str], env: &[(&str, &str)], colored: bool) {
    let root = project(&[("module.py", "x = 1\n")]);
    let mut args = vec!["check", "--statistics", "--select", "SMP010"];
    args.extend_from_slice(flags);
    assert_eq!(simplint_with(root.path(), &args, "", env).1.contains('\u{1b}'), colored);
}

#[test]
fn preview_help_says_simplint_has_no_preview_rules() {
    let root = project(&[]);
    let help = simplint(root.path(), &["check", "--help"]).1;
    let preview = help.lines().skip_while(|line| !line.contains("--preview")).skip(1);
    assert_eq!(
        preview.take_while(|line| !line.is_empty()).map(str::trim).collect::<Vec<_>>().join(" "),
        "Enable Ruff's preview mode: reports name rules instead of codes, and simplint's fixes are checked against \
         Ruff's preview rules too. simplint has no preview rules of its own. Use `--no-preview` to disable"
    );
}

#[test]
fn preview_shows_codes_only_for_rules() {
    let root = project(&[("module.py", "x = (\n")]);
    let (_, stable, _) = simplint(root.path(), &["check", "--output-format", "json"]);
    let (_, preview, _) = simplint(root.path(), &["check", "--output-format", "json", "--preview"]);
    assert_eq!((stable.contains(r#""code": "invalid-syntax""#), preview.contains(r#""code": null"#)), (true, true));
}

#[test]
fn preview_names_rules_as_ruff_does() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\npreview = true\n"),
        ("module.py", "# --- helpers ---\nVALUE = 1\n"),
    ]);
    let reported = check(root.path(), &["--select", "SMP001"]).1;
    let fixed = check(root.path(), &["--select", "SMP001", "--fix", "--show-fixes"]).1;
    assert_eq!(
        (reported.lines().next(), fixed.lines().nth(3)),
        (
            Some(
                "module.py:1:1: section-comment: [*] Section comment groups code; extract a named function or class \
                 instead"
            ),
            Some("    1 × section-comment (SMP001)")
        )
    );
}
