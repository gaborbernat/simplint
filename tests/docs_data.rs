mod common;

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use similar::TextDiff;
use test_case::test_case;

use common::{empty_project, simplint};

// Read the Docs builds the pages from these files rather than from a binary it would need minutes to compile, and
// SchemaStore and editors check `[tool.simplint]` against the schema, so a change to a flag or an option fails here
// until the files are regenerated; the failing run rewrites them to commit.
#[test_case(&["cli-schema"], "docs/_data/cli-schema.json" ; "cli_schema")]
#[test_case(&["config-schema"], "simplint.schema.json" ; "config_schema")]
fn docs_data_matches_the_binary(args: &[&str], file: &str) {
    let root = empty_project();
    let (code, stdout, stderr) = simplint(root.path(), args);
    assert_eq!((code, stderr.as_str()), (0, ""));
    regenerate_if_stale(&Path::new(env!("CARGO_MANIFEST_DIR")).join(file), &stdout);
}

fn regenerate_if_stale(path: &Path, generated: &str) {
    let committed = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => panic!("read {}: {error}", path.display()),
    };
    if committed != generated {
        fs::write(path, generated).expect("write docs data");
        let diff =
            TextDiff::from_lines(&committed, generated).unified_diff().header("committed", "regenerated").to_string();
        panic!("{} was stale and is now regenerated; commit it\n{diff}", path.display());
    }
}

// The rule pages read the rule list that `docs/rules_data.py` derives from the sources the binary is built from, so
// the derivation must list every rule the binary lists, with the same fields.
#[test]
fn rules_data_matches_the_binary() {
    let root = empty_project();
    let (code, stdout, stderr) = simplint(root.path(), &["rules", "--output-format", "json"]);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs").join("rules_data.py");
    let binary: Vec<Value> = serde_json::from_str(&stdout).expect("the binary lists rules as JSON");
    let derived: Vec<Value> =
        serde_json::from_slice(&rules_data_json(&script)).expect("the derivation lists rules as JSON");
    assert_eq!(derived, binary, "docs/rules_data.py lists rules differently from `simplint rules`");
}

// The derivation needs Python 3.10 for its `match` statements, so an older `python3` falls through to `python`.
fn rules_data_json(script: &Path) -> Vec<u8> {
    let mut failures = Vec::new();
    for interpreter in ["python3", "python"] {
        match Command::new(interpreter).arg(script).output() {
            Ok(output) if output.status.success() => return output.stdout,
            Ok(output) => failures.push(String::from_utf8_lossy(&output.stderr).into_owned()),
            Err(error) => failures.push(format!("{interpreter}: {error}")),
        }
    }
    panic!(
        "the rule list needs Python 3.10 or later on PATH, as `python3` or `python`; they said:\n{}",
        failures.join("\n")
    );
}
