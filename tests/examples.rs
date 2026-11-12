mod common;
#[path = "../src/lint/examples.rs"]
mod examples;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseOptions, parse_unchecked};
use serde_json::Value;
use tempfile::TempDir;

use common::{empty_project, project, read, simplint, text};
use examples::{Block, Example, Fence, Role, parse};

// Each rule's examples are checked in one run, so a wrong example is reported along with every other one.
#[test]
fn rule_examples_hold() {
    let root = empty_project();
    let (code, stdout, stderr) = simplint(root.path(), &["rules", "--output-format", "json"]);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let rules: Vec<Value> = serde_json::from_str(&stdout).expect("json");
    let failures = example_failures(&rules);
    assert!(failures.is_empty(), "examples that do not hold:\n{}", failures.join("\n"));
}

fn example_failures(rules: &[Value]) -> Vec<String> {
    let mut failures = Vec::new();
    for rule in rules {
        let code = text(&rule["code"]);
        match parse(text(&rule["explanation"])) {
            Ok(examples) => {
                for (index, example) in examples.iter().enumerate() {
                    if let Err(reason) = check_target(example).and_then(|()| check_example(code, example)) {
                        failures.push(format!("{code} example {}: {reason}", index + 1));
                    }
                }
            }
            Err(reason) => failures.push(format!("{code}: {reason}")),
        }
    }
    failures
}

// Every block parses without syntax the example's target version lacks, so a reader on that version can run it.
fn check_target(example: &Example) -> Result<(), String> {
    let version = target_version(example.config.as_ref())?;
    let options = ParseOptions::from(PySourceType::Python).with_target_version(version);
    for block in example.blocks.iter().filter(|block| block.language == "python") {
        let parsed = parse_unchecked(&block.fence.source, options.clone());
        if let Some(error) = parsed.unsupported_syntax_errors().first() {
            return Err(format!("{} does not parse for {version}: {error}", block.file));
        }
    }
    Ok(())
}

// The version an example's config targets: `[tool.ruff] target-version`, else the lower bound of
// `[project] requires-python`, else Ruff's default.
fn target_version(config: Option<&Fence>) -> Result<PythonVersion, String> {
    let Some(config) = config else { return Ok(PythonVersion::PY311) };
    let table: toml::Table = toml::from_str(&config.source).map_err(|error| format!("config is not TOML: {error}"))?;
    let target = table.get("tool").and_then(|tool| tool.get("ruff")).and_then(|ruff| ruff.get("target-version"));
    if let Some(target) = target {
        let target = target.as_str().ok_or_else(|| format!("target-version {target} is not a string"))?;
        let minor = target.strip_prefix("py3").ok_or_else(|| format!("target-version {target} is not `py3X`"))?;
        return format!("3.{minor}").parse().map_err(|error| format!("target-version {target}: {error}"));
    }
    let requires = table.get("project").and_then(|project| project.get("requires-python"));
    let Some(requires) = requires.and_then(toml::Value::as_str) else { return Ok(PythonVersion::PY311) };
    let lower = requires.strip_prefix(">=").ok_or_else(|| format!("requires-python {requires} has no `>=` bound"))?;
    let lower = lower.split(',').next().unwrap_or_default();
    lower.parse().map_err(|error| format!("requires-python {requires}: {error}"))
}

fn check_example(code: &str, example: &Example) -> Result<(), String> {
    if cfg!(not(unix)) && example.blocks.iter().any(|block| block.executable) {
        return Ok(());
    }
    let isolated = if example.config.is_none() { vec!["--isolated"] } else { Vec::new() };
    let select = selection(code, example);
    let root = written_project(example);
    let diagnostics = diagnostics(root.path(), &select, &isolated)?;
    for block in example.blocks.iter().filter(|block| matches!(block.role, Role::Report | Role::Ok)) {
        let path = root.path().join(&block.file);
        let found: Vec<&Value> = diagnostics
            .iter()
            .filter(|diagnostic| Path::new(text(&diagnostic["filename"])) == path)
            .filter(|diagnostic| block.role == Role::Ok || text(&diagnostic["code"]) == code)
            .collect();
        match block.role {
            Role::Report if found.is_empty() => return Err(format!("{} is not reported", block.file)),
            Role::Ok if !found.is_empty() => {
                let message = text(&found[0]["message"]);
                return Err(format!("{} is reported: {message}", block.file));
            }
            _ => {}
        }
    }
    let fixed: Vec<&Block> = example.blocks.iter().filter(|block| block.role == Role::Fixed).collect();
    if fixed.is_empty() {
        return Ok(());
    }
    let root = written_project(example);
    // Fixes keep the `select=` codes too, since the example's suppression comments depend on them.
    let fix =
        ["check", "--no-cache", "--select", select.as_str(), "--fix", "--unsafe-fixes", "--output-format", "concise"];
    simplint(root.path(), &[&fix[..], &isolated[..]].concat());
    for block in fixed {
        let actual = read(root.path(), &block.file);
        if actual != block.fence.source {
            return Err(format!("after --fix, {} is {actual:?}, not {:?}", block.file, block.fence.source));
        }
    }
    Ok(())
}

// The rule under test, then the codes an example's `select=` attributes add, so a selector another rule needs is kept.
fn selection(code: &str, example: &Example) -> String {
    let selected = example.blocks.iter().flat_map(|block| &block.select).map(String::as_str);
    std::iter::once(code).chain(selected).collect::<Vec<_>>().join(",")
}

// A `fixed` block names the file its report wrote, so only the other blocks are written.
fn written_project(example: &Example) -> TempDir {
    let files: Vec<(&str, &str)> = example
        .blocks
        .iter()
        .filter(|block| block.role != Role::Fixed)
        .map(|block| (block.file.as_str(), block.fence.source.as_str()))
        .collect();
    let config = example.config.as_ref().map(|config| ("pyproject.toml", config.source.as_str()));
    let root = project(&with_config(&files, config));
    mark_executable(root.path(), example);
    root
}

#[cfg(unix)]
fn mark_executable(root: &Path, example: &Example) {
    for block in example.blocks.iter().filter(|block| block.executable) {
        fs::set_permissions(root.join(&block.file), fs::Permissions::from_mode(0o755)).expect("make executable");
    }
}

// Other platforms have no executable bit, so `check_example` skips these examples before any file is written.
#[cfg(not(unix))]
fn mark_executable(_root: &Path, _example: &Example) {}

fn with_config<'a>(files: &[(&'a str, &'a str)], config: Option<(&'a str, &'a str)>) -> Vec<(&'a str, &'a str)> {
    files.iter().copied().chain(config).collect()
}

fn diagnostics(root: &Path, select: &str, isolated: &[&str]) -> Result<Vec<Value>, String> {
    let check = ["check", "--no-cache", "--select", select, "--output-format", "json"];
    let (_, stdout, stderr) = simplint(root, &[&check[..], isolated].concat());
    serde_json::from_str(&stdout).map_err(|_| format!("simplint did not report: {stderr}"))
}
