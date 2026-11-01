mod common;

use test_case::test_case;

use common::{project, simplint};

#[test]
fn rules_lists_codes() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rules"]);
    assert_eq!(
        (code, stdout.lines().take(3).collect::<Vec<_>>()),
        (
            0,
            vec![
                "SMP001 section-comment (fix)",
                "SMP002 function-docstring (fix sometimes)",
                "SMP003 multiline-module-docstring"
            ]
        )
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

#[test_case("dataclass-kw-only", "# dataclass-kw-only (SMP114)", "Fix is always available." ; "by_name_always")]
#[test_case("SMP002", "# function-docstring (SMP002)", "Fix is sometimes available." ; "by_code_sometimes")]
#[test_case("SMP003", "# multiline-module-docstring (SMP003)", "## What it does" ; "by_code_never")]
fn rule_explains(selector: &str, title: &str, third: &str) {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", selector]);
    assert_eq!((code, stdout.lines().take(3).collect::<Vec<_>>()), (0, vec![title, "", third]));
}

#[test]
fn rule_leaves_out_diagram_source() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "SMP020"]);
    assert_eq!((code, stdout.contains("mermaid"), stdout.contains("depth first")), (0, false, true));
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
    assert_eq!(
        (
            code,
            stdout.matches("\n## What it does").count(),
            stdout.starts_with("# section-comment (SMP001)\n\nFix is always available.\n\n")
        ),
        (0, 34, true)
    );
}

#[test]
fn rule_all_json_lists_every_rule() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "--all", "--output-format", "json"]);
    let rules: Vec<serde_json::Value> = serde_json::from_str(&stdout).expect("json");
    assert_eq!((code, rules.len(), rules[0]["fix_availability"].as_str()), (0, 34, Some("Always")));
}

#[test]
fn rule_json_describes_the_rule() {
    let root = project(&[]);
    let (code, stdout, _) = simplint(root.path(), &["rule", "SMP003", "--output-format", "json"]);
    let rule: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(
        (code, rule["name"].as_str(), rule["linter"].as_str(), rule["fix"].as_str(), rule["preview"].as_bool()),
        (
            0,
            Some("multiline-module-docstring"),
            Some("Comments, docstrings and structure"),
            Some("Fix is not available."),
            Some(false)
        )
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
#[test_case(&["config", "--output-format", "json"], "{\n  \"extend-fixable\": {" ; "list_json")]
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

#[test_case("text", "SMP0 Comments, docstrings and structure\nSMP1 Modules, typing and exports\nSMP2 Tests\n" ; "text")]
#[test_case("json", "[\n  {\n    \"prefix\": \"SMP0\",\n    \"name\": \"Comments, docstrings and structure\"\n  },\n  {\n    \"prefix\": \"SMP1\",\n    \"name\": \"Modules, typing and exports\"\n  },\n  {\n    \"prefix\": \"SMP2\",\n    \"name\": \"Tests\"\n  }\n]\n" ; "json")]
fn linter_lists_rule_groups(format: &str, stdout: &str) {
    let root = project(&[]);
    assert_eq!(simplint(root.path(), &["linter", "--output-format", format]), (0, stdout.to_owned(), String::new()));
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
