mod common;
#[path = "../src/lint/examples.rs"]
mod examples;

use std::path::Path;

use serde_json::Value;

use common::{project, read, simplint};
use examples::{Block, Example, Role, parse};

// Each rule's examples are checked in one run, so a wrong example is reported along with every other one.
#[test]
fn rule_examples_hold() {
    let root = project(&[]);
    let (code, stdout, stderr) = simplint(root.path(), &["rules", "--output-format", "json"]);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let rules: Vec<Value> = serde_json::from_str(&stdout).expect("json");
    let mut failures = Vec::new();
    for rule in &rules {
        let code = text(&rule["code"]);
        match parse(text(&rule["explanation"])) {
            Ok(examples) => {
                for (index, example) in examples.iter().enumerate() {
                    if let Err(reason) = check_example(code, example) {
                        failures.push(format!("{code} example {}: {reason}", index + 1));
                    }
                }
            }
            Err(reason) => failures.push(format!("{code}: {reason}")),
        }
    }
    assert!(failures.is_empty(), "examples that do not hold:\n{}", failures.join("\n"));
}

// Checks the reports, the clean files and the fix of one example, each in a project of its own.
fn check_example(code: &str, example: &Example) -> Result<(), String> {
    // A `fixed` block names the file its report wrote, so only the other blocks are written.
    let files: Vec<(&str, &str)> = example
        .blocks
        .iter()
        .filter(|block| block.role != Role::Fixed)
        .map(|block| (block.file.as_str(), block.fence.source.as_str()))
        .collect();
    let config = example.config.as_ref().map(|config| ("pyproject.toml", config.source.as_str()));
    let isolated = if example.config.is_none() { vec!["--isolated"] } else { Vec::new() };
    let root = project(&with_config(&files, config));
    let diagnostics = diagnostics(root.path(), code, &isolated)?;
    for block in example.blocks.iter().filter(|block| matches!(block.role, Role::Report | Role::Ok)) {
        let path = root.path().join(&block.file);
        let found: Vec<&Value> =
            diagnostics.iter().filter(|diagnostic| Path::new(text(&diagnostic["filename"])) == path).collect();
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
    let root = project(&with_config(&files, config));
    let fix = ["check", "--no-cache", "--select", code, "--fix", "--unsafe-fixes", "--output-format", "concise"];
    simplint(root.path(), &[&fix[..], &isolated[..]].concat());
    for block in fixed {
        let actual = read(root.path(), &block.file);
        if actual != block.fence.source {
            return Err(format!("after --fix, {} is {actual:?}, not {:?}", block.file, block.fence.source));
        }
    }
    Ok(())
}

fn with_config<'a>(files: &[(&'a str, &'a str)], config: Option<(&'a str, &'a str)>) -> Vec<(&'a str, &'a str)> {
    files.iter().copied().chain(config).collect()
}

fn diagnostics(root: &Path, code: &str, isolated: &[&str]) -> Result<Vec<Value>, String> {
    let check = ["check", "--no-cache", "--select", code, "--output-format", "json"];
    let (_, stdout, stderr) = simplint(root, &[&check[..], isolated].concat());
    serde_json::from_str(&stdout).map_err(|_| format!("simplint did not report: {stderr}"))
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}
