use tempfile::TempDir;
use test_case::test_case;

use crate::common::{self, check, read};

#[test_case("definition-before-use", "definition_before_use.py", "py314" ; "definition_before_use")]
#[test_case(
    "definition-before-use", "definition_before_use_floating.py", "py314" ; "definition_before_use_floating_comment"
)]
#[test_case(
    "definition-before-use", "definition_before_use_nonl.py", "py314" ; "definition_before_use_without_trailing_newline"
)]
#[test_case(
    "definition-before-use", "definition_before_use_classbody.py", "py314"
    ; "definition_before_use_class_body_reads_module_name"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", "py314" ; "definition_before_use_lazy_annotations"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", "py310" ; "definition_before_use_eager_annotations"
)]
#[test_case("forwarding-function", "forwarding_function.py", "py314" ; "forwarding_function")]
#[test_case("single-user-constant", "single_user_constant.py", "py314" ; "single_user_constant")]
#[test_case("single-user-constant", "single_user_constant_all.py", "py314" ; "single_user_constant_with_all")]
#[test_case("single-user-constant", "single_user_constant_dynamic.py", "py314" ; "single_user_constant_dynamic_all")]
#[test_case("single-user-constant", "single_user_constant_spacing.py", "py314" ; "single_user_constant_spacing")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py314" ; "walrus_candidate")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py37" ; "walrus_candidate_before_python38")]
#[test_case("walrus-candidate", "walrus_candidate_dynamic.py", "py314" ; "walrus_candidate_computed_all")]
#[test_case("walrus-candidate", "walrus_candidate_listed.py", "py314" ; "walrus_candidate_listed_in_all")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// Ruff's cleanup merges the `if` the fix leaves alone in an outer `if`, whatever the layout and line length.
#[test_case("if flag:", "        size = len(text)", "if flag and len(text) > 3:" ; "fits")]
#[test_case("if flag:", "        size = len(text                )", "if flag and len(text                ) > 3:" ; "spaced")]
#[test_case(
    "if not text:\n        return \"\"\n    elif flag:",
    "        size = len(text)",
    "if not text:\n        return \"\"\n    elif flag and len(text) > 3:"
    ; "elif"
)]
fn walrus_candidate_leaving_collapsible_if_is_merged(outer: &str, assignment: &str, merged: &str) {
    let source =
        format!("def run(text, flag):\n    {outer}\n{assignment}\n        if size > 3:\n            return text\n");
    let root = collapsible_if_project(&source);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP100", "--fix"]);
    let result = read(root.path(), "module.py");
    assert_eq!(
        (code, stdout.as_str(), result),
        (
            0,
            "Found 1 error (1 fixed, 0 remaining).\n",
            format!("def run(text, flag):\n    {merged}\n        return text\n")
        )
    );
}

#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n        else:\n            return \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n        else:\n            return \"\"\n"
    ; "inner_else"
)]
#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n    else:\n        return \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n    else:\n        return \"\"\n"
    ; "outer_else"
)]
fn walrus_candidate_keeping_if_branches_is_fixed(body: &str, fixed: &str) {
    let root = collapsible_if_project(&format!("def run(text, flag):\n{body}"));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP100", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (0, "Found 1 error (1 fixed, 0 remaining).\n", format!("def run(text, flag):\n{fixed}"))
    );
}

// Ruff collapses the nested `if` the walrus leaves only when the first line of the result fits the line length, so the
// unformatted line would decide what a second run, over the formatted one, does.
#[test]
fn walrus_candidate_leaving_collapsible_if_is_merged_whatever_the_layout() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff]\nline-length = 60\n[tool.ruff.lint]\nselect = [\"SIM102\"]\n"),
        (
            "module.py",
            "import re\n\n_PATTERN = re.compile(\"descendant-or-self::label[@for]\")\n\n\ndef find(text: str, flag: bool) \
             -> list[str] | None:\n    if flag:\n        found = _PATTERN.findall(text)\n        if found:\n            \
             return found\n    return None\n",
        ),
    ]);
    let select = ["--unsafe-fixes", "--select", "SMP050,SMP100"];
    check(root.path(), &[&["--fix"], &select[..]].concat());
    let second = check(root.path(), &[&["--diff"], &select[..]].concat()).0;
    assert_eq!(
        (read(root.path(), "module.py"), second),
        (
            "import re\n\n\ndef find(text: str, flag: bool) -> list[str] | None:\n    if flag and (found := \
             re.compile(\"descendant-or-self::label[@for]\").findall(text)):\n        return found\n    return None\n"
                .to_owned(),
            0
        )
    );
}

#[test]
fn walrus_candidate_lets_ruff_collapse_the_ifs_its_lines_now_fit() {
    let root = collapsible_if_project(
        "def run(text, flag):\n    size = len(text)\n    if size > 3 and flag:\n        if text.isupper():\n            \
         return text\n",
    );
    check(root.path(), &["--fix", "--select", "SMP100"]);
    let second = check(root.path(), &["--diff", "--select", "SMP100"]).0;
    assert_eq!(
        (read(root.path(), "module.py"), second),
        (
            "def run(text, flag):\n    if len(text) > 3 and flag and text.isupper():\n        return text\n".to_owned(),
            0
        )
    );
}

fn collapsible_if_project(source: &str) -> TempDir {
    let ruff = "[tool.ruff]\nline-length = 40\n\n[tool.ruff.lint]\nselect = [\"SIM102\"]\n";
    common::project(&[("pyproject.toml", ruff), ("module.py", source)])
}
