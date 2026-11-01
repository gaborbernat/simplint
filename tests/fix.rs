mod common;

use test_case::test_case;

use common::{check, project, read};

#[test]
fn check_fix_writes_file() {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    let result = check(root.path(), &["--fix", "--select", "SMP001"]);
    assert_eq!(
        (result, read(root.path(), "module.py")),
        ((0, "Found 1 error (1 fixed, 0 remaining).\n".to_owned(), String::new()), "VALUE = 1\n".to_owned())
    );
}

#[test_case(
    &["--fix"],
    "module.py:2:5: SMP002 Function docstring on internal code; let the name, signature and types document it\nFound 1 \
     error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n",
    DOCUMENTED ;
    "hinted_by_default"
)]
#[test_case(
    &["--fix", "--unsafe-fixes"], "Found 1 error (1 fixed, 0 remaining).\n", "def run() -> None:\n    print()\n" ;
    "applied_when_enabled"
)]
#[test_case(
    &["--no-unsafe-fixes"],
    "module.py:2:5: SMP002 Function docstring on internal code; let the name, signature and types document it\nFound 1 \
     error.\n",
    DOCUMENTED ;
    "hint_hidden"
)]
#[test_case(
    &["--unsafe-fixes"],
    "module.py:2:5: SMP002 [*] Function docstring on internal code; let the name, signature and types document it\nFound \
     1 error.\n[*] 1 fixable with `--fix`.\n",
    DOCUMENTED ;
    "fixable_when_enabled"
)]
fn check_unsafe_fixes(flags: &[&str], stdout: &str, content: &str) {
    let root = project(&[("module.py", DOCUMENTED)]);
    let result = check(root.path(), &[&["--select", "SMP002"], flags].concat()).1;
    assert_eq!((result, read(root.path(), "module.py")), (stdout.to_owned(), content.to_owned()));
}

const DOCUMENTED: &str = "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";

#[test]
fn check_safe_and_unsafe_fixes_are_counted_apart() {
    let root = project(&[("module.py", "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP002"]).1.lines().last(),
        Some("[*] 1 fixable with `--fix` (1 unsafe fix available with `--fix --unsafe-fixes`).")
    );
}

#[test]
fn check_diff_prints_without_writing() {
    let source = "# --- helpers ---\nVALUE = 1\n";
    let root = project(&[("module.py", source)]);
    let (code, stdout, _) = check(root.path(), &["--diff", "--select", "SMP001"]);
    assert_eq!(
        (code, stdout, read(root.path(), "module.py")),
        (
            1,
            "--- module.py\n+++ module.py\n@@ -1,2 +1 @@\n-# --- helpers ---\n VALUE = 1\n\nWould fix 1 error.\n"
                .to_owned(),
            source.to_owned()
        )
    );
}

#[test]
fn check_diff_without_fixes_prints_nothing() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    assert_eq!(check(root.path(), &["--diff", "--select", "SMP010"]), (0, String::new(), String::new()));
}

#[test_case(&["--fix-only"], 0, "Fixed 1 error.\n" ; "fix_only")]
#[test_case(&["--fix-only", "--exit-non-zero-on-fix"], 1, "Fixed 1 error.\n" ; "fix_only_exit_non_zero")]
#[test_case(&["--fix", "--exit-non-zero-on-fix"], 1, "Found 1 error (1 fixed, 0 remaining).\n" ; "fix_exit_non_zero")]
fn check_fix_exit_status(flags: &[&str], code: i32, stdout: &str) {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    let mut args = vec!["--select", "SMP001"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args), (code, stdout.to_owned(), String::new()));
}

#[test]
fn check_fix_only_reports_unsafe_fixes_left() {
    let root = project(&[("module.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        check(root.path(), &["--fix-only", "--select", "SMP002"]),
        (0, "No errors fixed (1 fix available with `--unsafe-fixes`).\n".to_owned(), String::new())
    );
}

#[test]
fn check_fix_only_reports_fixed_and_unsafe_fixes_left() {
    let root = project(&[("module.py", "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        check(root.path(), &["--fix-only", "--select", "SMP001,SMP002"]).1,
        "Fixed 1 error (1 additional fix available with `--unsafe-fixes`).\n"
    );
}

#[test]
fn check_show_fixes_lists_what_was_fixed() {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    assert_eq!(
        check(root.path(), &["--fix", "--show-fixes", "--select", "SMP001"]).1,
        "\nFixed 1 error:\n- module.py:\n    1 × SMP001 (section-comment)\n\nFound 1 error (1 fixed, 0 remaining).\n"
    );
}

#[test]
fn check_fix_on_read_only_file_is_reported() {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    common::make_read_only(&root.path().join("module.py"));
    let stdout = check(root.path(), &["--fix", "--select", "SMP001"]).1;
    assert_eq!((stdout.starts_with("module.py:1:1: E902 writing"), stdout.contains("(os error")), (true, true));
}

#[test_case(&[], true ; "fixable_by_default")]
#[test_case(&["--unfixable", "SMP001"], false ; "unfixable")]
#[test_case(&["--extend-unfixable", "SMP001"], false ; "extend_unfixable")]
#[test_case(&["--fixable", "SMP010"], false ; "fixable_leaves_it_out")]
#[test_case(&["--fixable", "SMP010", "--extend-fixable", "SMP001"], true ; "extend_fixable")]
#[test_case(&["--unfixable", "SMP0", "--extend-fixable", "SMP001"], true ; "specific_fixable_wins")]
#[test_case(&["--config", "unfixable = ['SMP001']"], false ; "inline_config")]
fn fixable_flags(flags: &[&str], fixable: bool) {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    let mut args = vec!["--select", "SMP001"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).1.contains("[*]"), fixable);
}

#[test_case("extend-safe-fixes = [\"SMP101\"]", "[*] 2 fixable with `--fix`." ; "safe")]
#[test_case("extend-unsafe-fixes = [\"SMP0\"]", "2 unsafe fixes available with `--fix --unsafe-fixes`." ; "unsafe")]
#[test_case("extend-safe-fixes = [\"SMP101\"]\nextend-unsafe-fixes = [\"SMP101\"]", "[*] 1 fixable with `--fix` (1 unsafe fix available with `--fix --unsafe-fixes`)." ; "unsafe_wins_a_tie")]
#[test_case("extend-safe-fixes = [\"SMP101\"]\nextend-unsafe-fixes = [\"SMP1\"]", "[*] 2 fixable with `--fix`." ; "specific_wins")]
fn fix_safety_options(options: &str, summary: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[tool.simplint]\n{options}\n")),
        ("module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP001,SMP101"]).1.lines().last(), Some(summary));
}

#[test]
fn fix_leaves_formatting_to_ruff_format() {
    let root = project(&[("module.py", "import os\n\n\n# ----------\n\n\n\nVALUE   = os.sep\n")]);
    check(root.path(), &["--fix", "--select", "SMP001"]);
    assert_eq!(read(root.path(), "module.py"), "import os\n\n\n\n\n\nVALUE   = os.sep\n");
}

#[test]
fn fix_lets_ruff_clean_up_what_it_disturbed() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F401\", \"E501\"]\n"),
        ("module.py", "from typing import Final\n\n_LIMIT: Final = 3\n\n\ndef limit() -> int:\n    return _LIMIT\n"),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050"]);
    assert_eq!(read(root.path(), "module.py"), "\n\ndef limit() -> int:\n    return 3\n");
}

// Ruff applies a fix that starts where the last one ended in the next pass, as removing an import line ends where the
// next import starts.
#[test]
fn fix_lets_ruff_clean_up_over_several_passes() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F401\"]\n"),
        (
            "module.py",
            "from typing import Final\nimport typing as t\n\n_LIMIT: Final = 3\n_STEP: t.Final = 1\n\n\ndef limit() -> int:\n    \
             return _LIMIT + _STEP\n",
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050"]);
    assert_eq!(read(root.path(), "module.py"), "\n\ndef limit() -> int:\n    return 3 + 1\n");
}

// Ruff's PLR2004 would report the value the fix inlines into a comparison.
const RUFF_PLR2004: (&str, &str) = ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"PLR2004\"]\n");
const COMPARED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value > _LIMIT\n";
const SINGLE_USER: &str = "SMP050 `_LIMIT` is used by one function only; define it there";

#[test]
fn fix_is_withdrawn_when_ruff_would_report_the_result() {
    let root = project(&[RUFF_PLR2004, ("module.py", COMPARED)]);
    let stdout = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050"]).1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (format!("module.py:1:1: {SINGLE_USER}\nFound 1 error.\n"), COMPARED.to_owned())
    );
}

#[test]
fn fix_withdraws_only_the_rule_whose_result_ruff_would_report() {
    let source = COMPARED.replace("def", "# --- helpers ---\ndef");
    let root = project(&[RUFF_PLR2004, ("module.py", &source)]);
    let stdout = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP001,SMP050"]).1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (format!("module.py:1:1: {SINGLE_USER}\nFound 2 errors (1 fixed, 1 remaining).\n"), COMPARED.to_owned())
    );
}

#[test]
fn ruff_violations_already_present_are_not_fixed_by_simplint() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F401\"]\n"),
        ("module.py", "import os\n\n\n# ----------\n\n\nVALUE = 1\n"),
    ]);
    check(root.path(), &["--fix", "--select", "SMP001"]);
    assert_eq!(read(root.path(), "module.py"), "import os\n\n\n\n\nVALUE = 1\n");
}

#[test]
fn fix_is_withdrawn_when_ruff_already_reports_the_same_rule_elsewhere() {
    let source = format!("def first(value: int) -> bool:\n    return value > 7\n\n\n{COMPARED}");
    let root = project(&[RUFF_PLR2004, ("module.py", &source)]);
    let stdout = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050"]).1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (format!("module.py:5:1: {SINGLE_USER}\nFound 1 error.\n"), source)
    );
}

#[test]
fn fix_renames_more_names_than_fix_passes() {
    let names: Vec<String> = (0..150).map(|index| format!("value{index}")).collect();
    let source = |names: &[String]| {
        format!("{} = 1\n\n\ndef total() -> int:\n    return {}\n", names.join(" = 1\n"), names.join(" + "))
    };
    let root = project(&[("module.py", &source(&names))]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    let renamed: Vec<String> = names.iter().map(|name| format!("_{}", name.to_ascii_uppercase())).collect();
    assert_eq!(read(root.path(), "module.py"), source(&renamed));
}

#[test]
fn fix_lets_ruff_apply_a_fix_it_made_possible() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"ANN201\"]\n"),
        (
            "module.py",
            "def label(count: int):\n    if count:\n        return None\n    return \"count \" + str(count)\n",
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP112"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "def label(count: int) -> str | None:\n    if count:\n        return None\n    return f\"count {count!s}\"\n"
    );
}

#[test]
fn fix_lets_ruff_clean_up_after_its_own_fixes() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"E402\", \"I001\"]\n"),
        (
            "module.py",
            "import os\nimport sys\n\nLIMIT = 3\n\ntry:\n    import uuid\nexcept ImportError:\n    uuid = None\n\nprint(os, \
             sys, uuid, LIMIT)\n",
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP040"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "import os\nimport sys\nimport uuid\n\nLIMIT = 3\n\n\nprint(os, sys, uuid, LIMIT)\n"
    );
}

#[test]
fn fix_takes_the_work_ruff_cleanup_leaves_for_the_rules() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py314\"\n[tool.ruff.lint]\nselect = [\"UP036\"]\n"),
        (
            "module.py",
            "import sys\n\nPY3 = sys.version_info[0] >= 3\n\nif PY3:\n    basestring = str\n\n\ndef is_text(value: object) \
             -> bool:\n    return isinstance(value, basestring)\n",
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050,SMP100,SMP102"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "import sys\n\n\n_BASESTRING = str\n\n\ndef is_text(value: object) -> bool:\n    return isinstance(value, _BASESTRING)\n"
    );
}

#[test]
fn fix_keeps_suppressions_of_rules_the_cleanup_does_not_fix() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\npreview = true\n[tool.ruff.lint]\nselect = [\"F401\", \"RUF100\"]\n"),
        (
            "a.py",
            "from b import (\n    _hidden,  # ruff: ignore[unused-import]\n    shown,  # ruff: ignore[unused-import]\n)\n",
        ),
        ("b.py", "shown = 1\n_hidden = 2\n"),
        ("c.py", "from a import shown\n\nprint(shown)\n"),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP113"]);
    assert_eq!(
        read(root.path(), "a.py"),
        "from b import (\n    _hidden,  # ruff: ignore[unused-import]\n    shown,\n)\n\n\n__all__ = [\n    \"shown\",\n]\n"
    );
}

#[test]
fn fix_handles_a_byte_order_mark() {
    let root = project(&[
        ("ordered.py", "\u{feff}def helper() -> None: ...\n\n\ndef main() -> None:\n    helper()\n"),
        ("constant.py", "\u{feff}_LIMIT = 3\n\n\ndef limit() -> int:\n    return _LIMIT\n"),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP020,SMP050"]);
    assert_eq!(
        (read(root.path(), "ordered.py"), read(root.path(), "constant.py")),
        (
            "\u{feff}def main() -> None:\n    helper()\n\n\ndef helper() -> None: ...\n".to_owned(),
            "\u{feff}\n\ndef limit() -> int:\n    return 3\n".to_owned()
        )
    );
}

// Ruff's COM819 takes the comma off again: with the comment after the first name, the list looks like one line to it.
#[test]
fn fix_that_ruff_undoes_is_not_a_fix() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"COM819\"]\n"),
        ("module.py", "__all__ = [\"alpha\",  # first\n           \"delta\"]\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--diff", "--select", "SMP115"]);
    assert_eq!((code, stdout), (0, String::new()));
}

// Ruff's cleanup of an inlined constant turns `%` formatting into an f-string, which SMP112 then joins with the text
// before it; without another run that would be left to the second round.
#[test]
fn fix_runs_again_on_what_ruff_cleaned_up() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"UP031\", \"UP032\"]\n"),
        (
            "module.py",
            "_NOTICE = \"\\n%sFrozen.\"\n\n\ndef describe(doc: str, indent: str) -> str:\n    return doc + _NOTICE % \
             indent\n",
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP050,SMP112"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "def describe(doc: str, indent: str) -> str:\n    return f\"{doc}\\n{indent}Frozen.\"\n"
    );
}

// Suppressions the fixes leave unused are Ruff's to remove, but a line length one only looks unused to the cleanup.
#[test]
fn fix_keeps_a_line_length_noqa_when_ruff_removes_unused_ones() {
    let long = format!("_LONG = \"{}\"  # noqa: E501\n", "x".repeat(40));
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.ruff]\nline-length = 40\n[tool.ruff.lint]\nselect = [\"E501\", \"F401\", \"RUF100\"]\n",
        ),
        ("a.py", &format!("from b import shown  # noqa: F401\n\n{long}")),
        ("b.py", "shown = 1\n"),
        ("c.py", "from a import shown\n\nprint(shown)\n"),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP113"]);
    assert_eq!(read(root.path(), "a.py"), format!("from b import shown\n\n{long}\n\n__all__ = [\n    \"shown\",\n]\n"));
}

// Ruff sorts an import block only once `ruff format` moves the statement after its `;` to a line of its own.
#[test]
fn fix_lets_ruff_sort_an_import_block_it_joins_across_a_semicolon() {
    let source = "import os\n\n__all__ = [\"run\"]\n\nimport sys; sys.setrecursionlimit(100)\n\n\ndef run() -> str:\n    return os.sep\n";
    let root = project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"I001\"]\n"), ("module.py", source)]);
    check(root.path(), &["--fix", "--select", "SMP104"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "import os\nimport sys\n\nsys.setrecursionlimit(100)\n\n\ndef run() -> str:\n    return os.sep\n\n\n__all__ = [\"run\"]\n"
    );
}

// `ruff check --fix` runs first, so a fix Ruff offers at the line length yet left in place is one that fails on this
// code: a fix touching its line does not hand it back to Ruff.
#[test]
fn fix_leaves_findings_ruff_would_fix_at_its_line_length_to_ruff() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"E711\"]\n"),
        ("module.py", "setting = None\nprint(setting == None)\n"),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    assert_eq!(read(root.path(), "module.py"), "_SETTING = None\nprint(_SETTING == None)\n");
}

// Ruff's findings runs leave out the layout rules, yet a suppression comment that may name one counts as used only
// while it runs: moving `main` up removes its blank line finding, so Ruff's cleanup drops the comment.
#[test_case("# noqa" ; "blanket_noqa")]
#[test_case("# noqa: E302" ; "noqa_code")]
#[test_case("# ruff: ignore[blank-lines-top-level]" ; "ignore_rule_name")]
fn fix_lets_ruff_drop_a_layout_suppression_the_fix_leaves_unused(suppression: &str) {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\npreview = true\nselect = [\"E302\", \"RUF100\"]\n"),
        (
            "module.py",
            &format!("def helper() -> int:\n    return 1\ndef main() -> int:  {suppression}\n    return helper()\n"),
        ),
    ]);
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP020"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "def main() -> int:\n    return helper()\n\n\ndef helper() -> int:\n    return 1\n"
    );
}

// Folding `user.uuid = 7` into the mock's constructor leaves no assignment to a `uuid` attribute, so the dataclass with
// that field can freeze in the same run.
#[test]
fn fix_lints_again_the_files_another_fix_changed_the_facts_of() {
    let root = project(&[
        ("model.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass User:\n    uuid: int\n"),
        (
            "test_model.py",
            "from unittest.mock import MagicMock\n\n\ndef test_user() -> None:\n    user = MagicMock()\n    user.uuid = \
             7\n    assert user.uuid == 7\n",
        ),
    ]);
    let fixed = check(root.path(), &["--fix", "--select", "SMP111,SMP203"]);
    assert_eq!(
        (fixed.1, read(root.path(), "model.py"), check(root.path(), &["--select", "SMP111,SMP203"]).1),
        (
            "Found 2 errors (2 fixed, 0 remaining).\n".to_owned(),
            "from dataclasses import dataclass\n\n\n@dataclass(frozen=True, slots=True)\nclass User:\n    uuid: int\n"
                .to_owned(),
            "All checks passed!\n".to_owned()
        )
    );
}

// Renaming `other.Lit` to `_Lit` leaves the dataclass `Lit` no namesake whose instances get attributes, so its fix turns
// safe and applies in the same run.
#[test]
fn fix_lints_again_the_dataclasses_whose_class_writes_changed() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nextend-safe-fixes = [\"SMP103\"]\n"),
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass(kw_only=True)\nclass Lit:\n    value: int\n"),
        (
            "other.py",
            "class Lit:\n    def __init__(self) -> None:\n        self.extra = 1\n\n\ndef run() -> Lit:\n    return \
             Lit()\n\n\n__all__ = [\n    \"run\",\n]\n",
        ),
    ]);
    let select = ["--select", "SMP103,SMP111", "--target-version", "py314"];
    let fixed = check(root.path(), &[&["--fix"], &select[..]].concat());
    assert_eq!(
        (fixed.1, read(root.path(), "models.py").lines().nth(3).map(str::to_owned)),
        (
            "Found 2 errors (2 fixed, 0 remaining).\n".to_owned(),
            Some("@dataclass(kw_only=True, frozen=True, slots=True)".to_owned())
        )
    );
}
