mod common;

use std::fs;

use test_case::test_case;

use common::{project, simplint};

#[test]
fn rules_lists_codes() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules"]);
    assert_eq!(
        (code, stdout.lines().take(3).collect::<Vec<_>>()),
        (0, vec!["SMP001 section-comment (fix)", "SMP002 long-paragraph", "SMP003 suppression-without-reason"])
    );
}

#[test]
fn rules_json_carries_explanations() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules", "--output-format", "json"]);
    let first = stdout.split("},").next().expect("one entry").to_owned();
    assert_eq!(
        (
            code,
            first.contains(r#""code": "SMP001""#),
            first.contains(r#""fix": "Always""#),
            first.contains("## What it does")
        ),
        (0, true, true, true)
    );
}

#[test]
fn rules_json_lists_parsed_examples() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules", "--output-format", "json"]);
    let rules: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("json");
    let first = &rules[0]["examples"]["parsed"][0];
    let imports = rules.iter().find(|rule| rule["code"] == "SMP301").expect("SMP301 is listed");
    assert_eq!(
        (
            code,
            first["section"].as_str(),
            first["line"].as_u64(),
            first["end_line"].as_u64(),
            first["blocks"][0]["role"].as_str(),
            first["blocks"][0]["line"].as_u64(),
            first["blocks"][0]["source"].as_str().map(|source| source.starts_with("# ---------- parsing")),
            imports["short_group"].as_str(),
        ),
        (0, Some("Example"), Some(11), Some(15), Some("report"), Some(11), Some(true), Some("Imports"))
    );
}

#[test_case("dataclass-kw-only", "# dataclass-kw-only (SMP703)", "Fix is always available." ; "by_name_always")]
#[test_case("SMP901", "# function-docstring (SMP901)", "Fix is sometimes available." ; "by_code_sometimes")]
#[test_case(
    "SMP002",
    "# long-paragraph (SMP002)",
    "Flags comment paragraphs longer than 100 words by default, since a paragraph that long usually mixes topics and gets" ;
    "by_code_never"
)]
fn rule_explains(selector: &str, title: &str, third: &str) {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", selector]);
    assert_eq!((code, stdout.lines().take(3).collect::<Vec<_>>()), (0, vec![title, "", third]));
}

#[test]
fn rule_leaves_out_diagram_source() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "SMP701"]);
    assert_eq!(
        (
            code,
            stdout.contains("mermaid"),
            stdout.contains("breaks it.\n\nA fix that adds `frozen`:"),
            stdout.contains("Has fields?")
        ),
        (0, false, true, false)
    );
}

#[test]
fn rule_shows_examples_as_plain_python() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "SMP601"]);
    assert_eq!(
        (
            code,
            stdout.contains("     Reported:\n     ```python\n     # new.py\n     TIMEOUT = 30\n     ```\n"),
            stdout.contains("```python report"),
        ),
        (0, true, false)
    );
}

#[test]
fn rule_rejects_unknown() {
    let root = project(&[]);
    assert_eq!(
        simplint(root.path(), &["rule", "NOPE"]),
        (
            2,
            String::new(),
            "error: invalid value 'NOPE' for '[RULE]'\n\nFor more information, try '--help'.\n".to_owned()
        )
    );
}

#[test_case("F401" ; "code")]
#[test_case("unused-import" ; "name")]
fn rule_points_ruff_rules_to_ruff(rule: &str) {
    let root = project(&[]);
    assert_eq!(
        simplint(root.path(), &["rule", rule]),
        (
            2,
            String::new(),
            format!(
                "error: invalid value '{rule}' for '[RULE]'\n\n  tip: `{rule}` is a Ruff rule; run `ruff rule {rule}`\n\nFor \
                 more information, try '--help'.\n"
            )
        )
    );
}

#[test]
fn rule_all_explains_every_rule() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "--all"]);
    let rules = simplint(root.path(), &["rules"]).1.lines().count();
    assert_eq!(
        (
            code,
            stdout.matches("\n## What it does").count(),
            stdout.starts_with("# section-comment (SMP001)\n\nFix is always available.\n\n")
        ),
        (0, rules, true)
    );
}

#[test]
fn rule_all_json_lists_every_rule() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "--all", "--output-format", "json"]);
    let rules: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("json");
    let listed = simplint(root.path(), &["rules"]).1.lines().count();
    assert_eq!((code, rules.len(), rules[0]["fix_availability"].as_str()), (0, listed, Some("Always")));
}

#[test]
fn rule_json_describes_the_rule() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "SMP002", "--output-format", "json"]);
    let rule: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(
        (code, rule["name"].as_str(), rule["linter"].as_str(), rule["fix"].as_str(), rule["preview"].as_bool()),
        (0, Some("long-paragraph"), Some("Comments"), Some("Fix is not available."), Some(false))
    );
}

#[test]
fn rule_needs_a_rule_or_all() {
    let root = project(&[]);
    let (code, _, stderr) = simplint(root.path(), &["rule"]);
    assert_eq!((code, stderr.contains("<RULE|--all>")), (2, true));
}

#[test_case(&["config"], "output-format\nfix\n" ; "list")]
#[test_case(&["config", "fix"], "Enable fix behavior by-default" ; "text")]
#[test_case(&["config", "fix", "--output-format", "json"], "{\n  \"doc\": \"Enable fix behavior" ; "json")]
#[test_case(&["config", "--output-format", "json"], "{\n  \"api-modules\": {" ; "list_json")]
#[test_case(&["config", "naming.keep-names"], "A list of module-scope names" ; "nested")]
fn config_describes_options(args: &[&str], start: &str) {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), args);
    assert_eq!((code, stdout.starts_with(start)), (0, true));
}

#[test]
fn config_rejects_unknown_options() {
    let root = project(&[]);
    let (code, _, stderr) = simplint(root.path(), &["config", "nope"]);
    assert_eq!((code, stderr.starts_with("error: invalid value 'nope' for '[OPTION]'")), (2, true));
}

#[test_case("text", "Comments: SMP001, SMP002, SMP003, SMP004\n" ; "text")]
#[test_case("json", "[\n  {\n    \"name\": \"Comments\",\n    \"codes\": [\n      \"SMP001\",\n      \"SMP002\",\n      \"SMP003\",\n      \"SMP004\"\n    ]\n  }," ; "json")]
fn linter_lists_rule_groups(format: &str, first_group: &str) {
    let root = project(&[]);
    let (code, stdout, stderr) = simplint(root.path(), &["linter", "--output-format", format]);
    let rules = simplint(root.path(), &["rules"]).1.lines().count();
    assert_eq!(
        (code, stdout.starts_with(first_group), stdout.matches("SMP").count(), stderr),
        (0, true, rules, String::new())
    );
}

#[test_case(&[], "Removing cache at: sub/.ruff_cache/simplint\n" ; "default")]
#[test_case(&["--quiet"], "" ; "quiet")]
fn clean_removes_caches(flags: &[&str], stderr: &str) {
    let root = project(&[("sub/.ruff_cache/simplint/0.1.0/1234", ""), ("sub/.ruff_cache/0.17.3/5678", "")]);
    let mut args = vec!["clean"];
    args.extend_from_slice(flags);
    let result = simplint(root.path(), &args);
    let cache = root.path().join("sub/.ruff_cache");
    assert_eq!(
        (result, cache.join("simplint").exists(), cache.join("0.17.3/5678").exists()),
        ((0, String::new(), common::native(stderr)), false, true)
    );
}

// A link where the cache or its simplint directory should be is not the run's to remove, nor what it points at.
#[cfg(unix)]
#[test_case(true ; "ruff_cache_is_a_link")]
#[test_case(false ; "simplint_is_a_link")]
fn clean_keeps_links_and_what_they_point_at(ruff_cache_is_link: bool) {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    fs::create_dir_all(outside.path().join("simplint/0.1.0")).expect("create");
    fs::write(outside.path().join("simplint/0.1.0/1234"), "").expect("write");
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let (link, target) = if ruff_cache_is_link {
        (root.path().join(".ruff_cache"), outside.path().to_path_buf())
    } else {
        fs::create_dir(root.path().join(".ruff_cache")).expect("create");
        (root.path().join(".ruff_cache/simplint"), outside.path().join("simplint"))
    };
    std::os::unix::fs::symlink(&target, &link).expect("symlink");
    let (code, _, stderr) = simplint(root.path(), &["clean"]);
    assert_eq!(
        (
            (code, stderr),
            link.symlink_metadata().is_ok_and(|metadata| metadata.is_symlink()),
            outside.path().join("simplint/0.1.0/1234").exists(),
        ),
        ((0, String::new()), true, true)
    );
}

#[test_case("text", &format!("simplint {}\n", env!("CARGO_PKG_VERSION")) ; "text")]
#[test_case("json", &format!("{{\n  \"version\": \"{}\",\n  \"commit_info\": null\n}}", env!("CARGO_PKG_VERSION")) ; "json")]
fn version_prints_the_version(format: &str, stdout: &str) {
    let root = project(&[]);
    assert_eq!(simplint(root.path(), &["version", "--output-format", format]), (0, stdout.to_owned(), String::new()));
}

#[test]
fn generate_shell_completion_writes_a_script() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["generate-shell-completion", "bash"]);
    assert_eq!((code, stdout.contains("_simplint()")), (0, true));
}

#[test]
fn cli_schema_describes_check_flags() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["cli-schema"]);
    let check = stdout.split(r#""name": "check""#).next().expect("check subcommand").to_owned();
    assert_eq!(
        (
            code,
            check.contains(r#""long": "unsafe-fixes""#),
            check.contains(r#""delimiter": ",""#),
            stdout.contains("cli-schema")
        ),
        (0, true, true, false)
    );
}

#[test_case("SMP202", true ; "project_index")]
#[test_case("SMP101", false ; "one_file")]
fn rule_says_whether_other_files_matter(selector: &str, cross_file: bool) {
    let root = project(&[]);
    let text = simplint(root.path(), &["rule", selector]).1;
    let json = simplint(root.path(), &["rule", selector, "--output-format", "json"]).1;
    assert_eq!(
        (text.contains("\nReads other files of the run.\n"), json.contains(&format!(r#""cross_file": {cross_file}"#))),
        (cross_file, true)
    );
}
