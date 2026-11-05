mod common;

use std::fs;
use std::path::Path;

use test_case::test_case;

use common::{project, simplint};

// Read the Docs builds the pages from these files rather than from a binary it would need minutes to compile, and
// SchemaStore and editors check `[tool.simplint]` against the schema, so a change to a rule, a flag or an option fails
// here until the files are regenerated; the failing run rewrites them to commit.
#[test_case(&["rules", "--output-format", "json"], "docs/_data/rules.json" ; "rules")]
#[test_case(&["cli-schema"], "docs/_data/cli-schema.json" ; "cli_schema")]
#[test_case(&["config-schema"], "simplint.schema.json" ; "config_schema")]
fn docs_data_matches_the_binary(args: &[&str], file: &str) {
    let root = project(&[]);
    let (code, stdout, stderr) = simplint(root.path(), args);
    assert_eq!((code, stderr.as_str()), (0, ""));
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    if fs::read_to_string(&path).ok().as_deref() != Some(stdout.as_str()) {
        fs::write(&path, &stdout).expect("write docs data");
        panic!("{} was stale and is now regenerated; commit it", path.display());
    }
}
