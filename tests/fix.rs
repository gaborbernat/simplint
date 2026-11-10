mod common;

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use test_case::test_case;

use common::{check, fix_unsafe, project, read};

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
    "module.py:2:5: SMP901 Function docstring on internal code; let the name, signature and types document it\nFound 1 \
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
    "module.py:2:5: SMP901 Function docstring on internal code; let the name, signature and types document it\nFound 1 \
     error.\n",
    DOCUMENTED ;
    "hint_hidden"
)]
#[test_case(
    &["--unsafe-fixes"],
    "module.py:2:5: SMP901 [*] Function docstring on internal code; let the name, signature and types document it\nFound \
     1 error.\n[*] 1 fixable with `--fix`.\n",
    DOCUMENTED ;
    "fixable_when_enabled"
)]
fn check_unsafe_fixes(flags: &[&str], stdout: &str, content: &str) {
    let root = project(&[("module.py", DOCUMENTED)]);
    let result = check(root.path(), &[&["--select", "SMP901"], flags].concat()).1;
    assert_eq!((result, read(root.path(), "module.py")), (stdout.to_owned(), content.to_owned()));
}

const DOCUMENTED: &str = "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";

#[test]
fn check_safe_and_unsafe_fixes_are_counted_apart() {
    let root = project(&[("module.py", "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP901"]).1.lines().last(),
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
    assert_eq!(check(root.path(), &["--diff", "--select", "SMP101"]), (0, String::new(), String::new()));
}

#[test]
fn check_diff_prints_files_in_order() {
    let names: Vec<String> = (0..20).map(|index| format!("module{index:02}.py")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "# --- helpers ---\nVALUE = 1\n")).collect();
    let root = project(&files);
    let mut diffs = String::new();
    for name in &names {
        write!(diffs, "--- {name}\n+++ {name}\n@@ -1,2 +1 @@\n-# --- helpers ---\n VALUE = 1\n\n").expect("write diff");
    }
    assert_eq!(
        check(root.path(), &["--diff", "--select", "SMP001"]),
        (1, format!("{diffs}Would fix 20 errors.\n"), String::new())
    );
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
        check(root.path(), &["--fix-only", "--select", "SMP901"]),
        (0, "No errors fixed (1 fix available with `--unsafe-fixes`).\n".to_owned(), String::new())
    );
}

#[test]
fn check_fix_only_reports_fixed_and_unsafe_fixes_left() {
    let root = project(&[("module.py", "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n")]);
    assert_eq!(
        check(root.path(), &["--fix-only", "--select", "SMP001,SMP901"]).1,
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

#[cfg(unix)]
#[test]
fn check_fix_keeps_the_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    common::set_mode(&root.path().join("module.py"), 0o751);
    check(root.path(), &["--fix", "--select", "SMP001"]);
    let mode = std::fs::metadata(root.path().join("module.py")).expect("metadata").permissions().mode();
    assert_eq!((mode & 0o777, read(root.path(), "module.py")), (0o751, "VALUE = 1\n".to_owned()));
}

#[cfg(unix)]
#[test]
fn check_fix_writes_through_a_symlink() {
    let root = project(&[("source.txt", "# --- helpers ---\nVALUE = 1\n")]);
    std::os::unix::fs::symlink("source.txt", root.path().join("module.py")).expect("symlink");
    check(root.path(), &["--fix", "--select", "SMP001", "module.py"]);
    let link = std::fs::symlink_metadata(root.path().join("module.py")).expect("metadata");
    assert_eq!((link.is_symlink(), read(root.path(), "source.txt")), (true, "VALUE = 1\n".to_owned()));
}

#[cfg(unix)]
#[test]
fn check_fix_keeps_the_file_and_its_hard_links() {
    use std::os::unix::fs::MetadataExt;
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    std::fs::hard_link(root.path().join("module.py"), root.path().join("link.txt")).expect("hard link");
    let inode = std::fs::metadata(root.path().join("module.py")).expect("metadata").ino();
    check(root.path(), &["--fix", "--select", "SMP001", "module.py"]);
    let after = std::fs::metadata(root.path().join("module.py")).expect("metadata").ino();
    assert_eq!((after, read(root.path(), "link.txt")), (inode, "VALUE = 1\n".to_owned()));
}

#[test_case(&[], true ; "fixable_by_default")]
#[test_case(&["--unfixable", "SMP001"], false ; "unfixable")]
#[test_case(&["--extend-unfixable", "SMP001"], false ; "extend_unfixable")]
#[test_case(&["--fixable", "SMP101"], false ; "fixable_leaves_it_out")]
#[test_case(&["--fixable", "SMP101", "--extend-fixable", "SMP001"], true ; "extend_fixable")]
#[test_case(&["--unfixable", "SMP0", "--extend-fixable", "SMP001"], true ; "specific_fixable_wins")]
#[test_case(&["--config", "unfixable = ['SMP001']"], false ; "inline_config")]
fn fixable_flags(flags: &[&str], fixable: bool) {
    let root = project(&[("module.py", "# --- helpers ---\nVALUE = 1\n")]);
    let mut args = vec!["--select", "SMP001"];
    args.extend_from_slice(flags);
    assert_eq!(check(root.path(), &args).1.contains("[*]"), fixable);
}

#[test_case("extend-safe-fixes = [\"SMP601\"]", "[*] 2 fixable with `--fix`." ; "safe")]
#[test_case("extend-unsafe-fixes = [\"SMP0\"]", "2 unsafe fixes available with `--fix --unsafe-fixes`." ; "unsafe")]
#[test_case("extend-safe-fixes = [\"SMP601\"]\nextend-unsafe-fixes = [\"SMP601\"]", "[*] 1 fixable with `--fix` (1 unsafe fix available with `--fix --unsafe-fixes`)." ; "unsafe_wins_a_tie")]
#[test_case("extend-safe-fixes = [\"SMP601\"]\nextend-unsafe-fixes = [\"SMP6\"]", "[*] 2 fixable with `--fix`." ; "specific_wins")]
fn fix_safety_options(options: &str, summary: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[tool.simplint]\n{options}\n")),
        ("module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP001,SMP601"]).1.lines().last(), Some(summary));
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
    fix_unsafe(root.path(), "SMP202");
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
    fix_unsafe(root.path(), "SMP202");
    assert_eq!(read(root.path(), "module.py"), "\n\ndef limit() -> int:\n    return 3 + 1\n");
}

// Ruff's PLR2004 would report the value the fix inlines into a comparison.
const RUFF_PLR2004: (&str, &str) = ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"PLR2004\"]\n");
const COMPARED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value > _LIMIT\n";
const SINGLE_USER: &str = "SMP202 `_LIMIT` is used by one function only; define it there";

#[test]
fn fix_is_withdrawn_when_ruff_would_report_the_result() {
    let root = project(&[RUFF_PLR2004, ("module.py", COMPARED)]);
    let stdout = fix_unsafe(root.path(), "SMP202").1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (format!("module.py:1:1: {SINGLE_USER}\nFound 1 error.\n"), COMPARED.to_owned())
    );
}

#[test]
fn fix_withdraws_only_the_rule_whose_result_ruff_would_report() {
    let source = COMPARED.replace("def", "# --- helpers ---\ndef");
    let root = project(&[RUFF_PLR2004, ("module.py", &source)]);
    let stdout = fix_unsafe(root.path(), "SMP001,SMP202").1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (format!("module.py:1:1: {SINGLE_USER}\nFound 2 errors (1 fixed, 1 remaining).\n"), COMPARED.to_owned())
    );
}

// Ruff's PLR2004 reports the value the `limit` fix inlines into a comparison, with the line length kept at 120 so the
// `gil` report stays.
const RUFF_PLR2004_WIDE: (&str, &str) =
    ("pyproject.toml", "[tool.ruff]\nline-length = 120\n[tool.ruff.lint]\nselect = [\"PLR2004\"]\n");

#[test]
fn fix_withdraws_only_the_report_whose_result_ruff_would_report() {
    let root = project(&[RUFF_PLR2004_WIDE, ("module.py", SINGLE_USE_BUILDS)]);
    let stdout = fix_unsafe(root.path(), "SMP504").1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (
            "module.py:10:5: SMP504 `limit` is used once, right below; inline its value\nFound 2 errors (1 fixed, 1 remaining).\n"
                .to_owned(),
            GIL_INLINED.to_owned()
        )
    );
}

// The `limit` of `_printed` is valid to inline, as its report is the same as `_compared`'s, yet it is fixed.
#[test]
fn fix_withholds_only_the_identical_report_ruff_would_report() {
    let root = project(&[RUFF_PLR2004_WIDE, ("module.py", COLLIDING)]);
    let stdout = fix_unsafe(root.path(), "SMP504").1;
    assert_eq!(
        (stdout, read(root.path(), "module.py")),
        (
            "module.py:2:5: SMP504 `limit` is used once, right below; inline its value\nFound 2 errors (1 fixed, 1 remaining).\n"
                .to_owned(),
            COLLIDING.replace("    limit = 5\n    print(limit)\n", "    print(5)\n")
        )
    );
}

// Each valid fix settles with the rest, and a bad one is found by halving the fixes it is withheld from, so the time
// stays near-linear in the file, where one settle per fix took tens of seconds at 800 fixes.
#[test]
fn fix_finds_a_bad_fix_among_many_valid_ones_quickly() {
    let mut source = String::new();
    for index in 0..400 {
        write!(
            source,
            "def func_{index}(arg):\n    value_{index} = arg.compute({index})\n    return value_{index}\n\n\n"
        )
        .expect("write source");
    }
    source.push_str(UNCONTENDED);
    let root = project(&[RUFF_PLR2004_WIDE, ("module.py", &source)]);
    let started = Instant::now();
    let stdout = fix_unsafe(root.path(), "SMP504").1;
    assert_eq!(
        (stdout.ends_with("Found 401 errors (400 fixed, 1 remaining).\n"), started.elapsed() < Duration::from_secs(4)),
        (true, true)
    );
}

// The valid inlines have the same text, so a withheld report is told apart from them only by its occurrence, which the
// inlines applied before it change. The bad inline sits between them and must stay withheld as they apply.
#[test]
fn fix_withholds_a_bad_fix_among_identical_valid_ones() {
    let mut source = String::new();
    let mut inlined = String::new();
    for index in 0..100 {
        if index == 50 {
            source.push_str(UNCONTENDED);
            inlined.push_str(UNCONTENDED);
        }
        write!(source, "def valid_{index}():\n    limit = 5\n    print(limit)\n\n\n").expect("write source");
        write!(inlined, "def valid_{index}():\n    print(5)\n\n\n").expect("write source");
    }
    let root = project(&[RUFF_PLR2004_WIDE, ("module.py", &source)]);
    let started = Instant::now();
    let stdout = fix_unsafe(root.path(), "SMP504").1;
    assert_eq!(
        (
            stdout.ends_with("Found 101 errors (100 fixed, 1 remaining).\n"),
            read(root.path(), "module.py"),
            started.elapsed() < Duration::from_secs(4),
        ),
        (true, inlined, true)
    );
}

const UNCONTENDED: &str = "def _compared(value):\n    limit = 5\n\n    if value > limit:\n        print(value)\n";
const COLLIDING: &str = "def _compared(value):\n    limit = 5\n\n    if value > limit:\n        print(value)\n\n\ndef _printed():\n    limit = 5\n    print(limit)\n";

const SINGLE_USE_BUILDS: &str = "import sys\n\n\ndef _environment() -> str:\n    gil = getattr(sys, \"_is_gil_enabled\", lambda: True)()\n    build = \"gil\" if gil else \"free-threaded\"\n    return f\"x {build}\"\n\n\ndef _compared(value):\n    limit = 5\n\n    if value > limit:\n        print(value)\n";
const GIL_INLINED: &str = "import sys\n\n\ndef _environment() -> str:\n    build = \"gil\" if getattr(sys, \"_is_gil_enabled\", lambda: True)() else \"free-threaded\"\n    return f\"x {build}\"\n\n\ndef _compared(value):\n    limit = 5\n\n    if value > limit:\n        print(value)\n";

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
    let stdout = fix_unsafe(root.path(), "SMP202").1;
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
    fix_unsafe(root.path(), "SMP102");
    let renamed: Vec<String> = names.iter().map(|name| name.to_ascii_uppercase()).collect();
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
    fix_unsafe(root.path(), "SMP503");
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
    fix_unsafe(root.path(), "SMP303");
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
    fix_unsafe(root.path(), "SMP202,SMP502,SMP102");
    assert_eq!(
        read(root.path(), "module.py"),
        "import sys\n\n\nBASESTRING = str\n\n\ndef is_text(value: object) -> bool:\n    return isinstance(value, BASESTRING)\n"
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
    fix_unsafe(root.path(), "SMP401");
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
    fix_unsafe(root.path(), "SMP201,SMP202");
    assert_eq!(
        (read(root.path(), "ordered.py"), read(root.path(), "constant.py")),
        (
            "\u{feff}def main() -> None:\n    helper()\n\n\ndef helper() -> None: ...\n".to_owned(),
            "\u{feff}\n\ndef limit() -> int:\n    return 3\n".to_owned()
        )
    );
}

// SMP201 leaves a constructor where it is, and SMP203 moves it above the methods, so the two settle on one order: the
// constructor first, and the helper it calls below it.
#[test]
fn fix_settles_constructors_with_the_methods_that_call_them() {
    let root = project(&[(
        "module.py",
        "class Holder:\n    def helper(self) -> int:\n        return 1\n\n    def __init__(self) -> None:\n        self.value = self.helper()\n\n    def reset(self) -> None:\n        self.__init__()\n",
    )]);
    fix_unsafe(root.path(), "SMP201,SMP203");
    assert_eq!(
        (read(root.path(), "module.py"), check(root.path(), &["--select", "SMP201,SMP203"])),
        (
            "class Holder:\n    def __init__(self) -> None:\n        self.value = self.helper()\n\n    def helper(self) -> int:\n        return 1\n\n    def reset(self) -> None:\n        self.__init__()\n".to_owned(),
            (0, "All checks passed!\n".to_owned(), String::new())
        )
    );
}

// A helper follows the first constructor that calls it, as SMP201 places it, and SMP203 moves that constructor above the
// methods. Both rules settle on one order, and a second fix changes nothing.
#[test]
fn fix_places_a_helper_after_the_constructor_that_calls_it() {
    let root = project(&[(
        "module.py",
        "class Holder:\n    def run(self) -> int:\n        return 1\n\n    @classmethod\n    def compile(cls) -> None:\n        pass\n\n    def __init_subclass__(cls) -> None:\n        cls.compile()\n",
    )]);
    fix_unsafe(root.path(), "SMP201,SMP203");
    assert_eq!(
        (read(root.path(), "module.py"), check(root.path(), &["--select", "SMP201,SMP203"])),
        (
            "class Holder:\n    def __init_subclass__(cls) -> None:\n        cls.compile()\n\n    @classmethod\n    def compile(cls) -> None:\n        pass\n\n    def run(self) -> int:\n        return 1\n".to_owned(),
            (0, "All checks passed!\n".to_owned(), String::new())
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
    let (code, stdout, _) = check(root.path(), &["--diff", "--select", "SMP405"]);
    assert_eq!((code, stdout), (0, String::new()));
}

// Ruff's cleanup of an inlined constant turns `%` formatting into an f-string, which SMP503 then joins with the text
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
    fix_unsafe(root.path(), "SMP202,SMP503");
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
    fix_unsafe(root.path(), "SMP401");
    assert_eq!(read(root.path(), "a.py"), format!("from b import shown\n\n{long}\n\n__all__ = [\n    \"shown\",\n]\n"));
}

// Ruff sorts an import block only once `ruff format` moves the statement after its `;` to a line of its own.
#[test]
fn fix_lets_ruff_sort_an_import_block_it_joins_across_a_semicolon() {
    let source = "import os\n\n__all__ = [\"run\"]\n\nimport sys; sys.setrecursionlimit(100)\n\n\ndef run() -> str:\n    return os.sep\n";
    let root = project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"I001\"]\n"), ("module.py", source)]);
    check(root.path(), &["--fix", "--select", "SMP406"]);
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
    fix_unsafe(root.path(), "SMP102");
    assert_eq!(read(root.path(), "module.py"), "SETTING = None\nprint(SETTING == None)\n");
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
    fix_unsafe(root.path(), "SMP201");
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
    let fixed = check(root.path(), &["--fix", "--select", "SMP701,SMP702,SMP805"]);
    assert_eq!(
        (fixed.1, read(root.path(), "model.py"), check(root.path(), &["--select", "SMP701,SMP702,SMP805"]).1),
        (
            "Found 3 errors (3 fixed, 0 remaining).\n".to_owned(),
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
        ("pyproject.toml", "[tool.simplint]\nextend-safe-fixes = [\"SMP403\"]\n"),
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass(kw_only=True)\nclass Lit:\n    value: int\n"),
        (
            "other.py",
            "class Lit:\n    def __init__(self) -> None:\n        self.extra = 1\n\n\ndef run() -> int:\n    return \
             Lit().extra\n\n\n__all__ = [\n    \"run\",\n]\n",
        ),
    ]);
    let select = ["--select", "SMP403,SMP701,SMP702", "--target-version", "py314"];
    let fixed = check(root.path(), &[&["--fix"], &select[..]].concat());
    assert_eq!(
        (fixed.1, read(root.path(), "models.py").lines().nth(3).map(str::to_owned)),
        (
            "Found 3 errors (3 fixed, 0 remaining).\n".to_owned(),
            Some("@dataclass(kw_only=True, frozen=True, slots=True)".to_owned())
        )
    );
}

// Ruff's C419 would turn a comprehension passed to `all` into a generator once it is inlined there, so a comprehension
// whose elements call something stays a list, and the calls all run. One of names only has no call to run.
#[test_case(
    "[convert(domain) for domain in domains]",
    "    results = [convert(domain) for domain in domains]\n    return all(results)\n" ;
    "calls_stay"
)]
#[test_case("[domain for domain in domains]", "    return all(domain for domain in domains)\n" ; "names_inline")]
fn single_use_variable_inlines_a_comprehension_into_all(comprehension: &str, body: &str) {
    let source = format!("def check(domains):\n    results = {comprehension}\n    return all(results)\n");
    let root = project(&[("module.py", &source)]);
    let (code, _, _) = fix_unsafe(root.path(), "SMP504");
    assert_eq!((code, read(root.path(), "module.py")), (0, format!("def check(domains):\n{body}")));
}

// The statement that uses the constant shares its line with the one before it, so the fix splits that line, as the
// formatter would, and a second run finds nothing to move.
#[test]
fn early_assignment_splits_a_shared_line() {
    let source = "def build(ffi):\n    name = \"struct ab\"\n    ffi2 = ffi.FFI(); ffi2.cdef(name)\n    return ffi2\n";
    let root = project(&[("module.py", source)]);
    let (code, _, _) = fix_unsafe(root.path(), "SMP526");
    let fixed =
        "def build(ffi):\n    ffi2 = ffi.FFI()\n    name = \"struct ab\"\n    ffi2.cdef(name)\n    return ffi2\n";
    let (again, _, _) = check(root.path(), &["--select", "SMP526"]);
    assert_eq!((code, again, read(root.path(), "module.py")), (0, 0, fixed.to_owned()));
}

// A fix that rewrites the file's comment or docstring leaves it alone when a `noqa` silences a line the rewrite would
// change, so the directive stays used and the file keeps its text.
#[test_case(
    "tests/test_module.py",
    "SMP902",
    "\"\"\"Tests for a module whose long docstring line is silenced by the file-level noqa directive below.\"\"\"\n# ruff: noqa: E501\n\n\ndef test_a() -> None:\n    pass\n",
    1 ; "test_module_docstring_under_file_noqa"
)]
#[test_case(
    "module.py",
    "SMP901",
    "# ruff: noqa: E501\n\n\nclass Box:\n    \"\"\"Holds one value, with a tie-break rule the code does not show.\"\"\"\n\n    value: int = 0\n",
    1 ; "function_docstring_under_file_noqa"
)]
#[test_case(
    "tests/test_box.py",
    "SMP801",
    "class TestBox:\n    def test_value(self) -> None:\n        value = 1  # noqa: E501\n        assert value\n",
    1 ; "test_class_with_noqa_on_a_long_line"
)]
#[test_case(
    "tests/test_box.py",
    "SMP801",
    "class TestBox:\n    def test_value(self) -> None:\n        value = 1  # ruff: ignore[E501]\n        assert value\n",
    1 ; "test_class_with_ruff_ignore_on_a_long_line"
)]
fn fix_leaves_a_file_its_noqa_directive_depends_on(path: &str, select: &str, source: &str, code: i32) {
    let root = project(&[(path, source)]);
    let (actual, _, _) = fix_unsafe(root.path(), select);
    assert_eq!((actual, read(root.path(), path)), (code, source.to_owned()));
}

// The statement that uses the constant shares its line with one after it, which the fix splits off as well.
#[test]
fn early_assignment_splits_a_line_the_use_shares_with_a_later_statement() {
    let source = "def build(ffi):\n    name = \"struct ab\"\n    ffi.reset()\n    ffi.cdef(name); ffi.verify(name)\n";
    let root = project(&[("module.py", source)]);
    let (code, _, _) = fix_unsafe(root.path(), "SMP526");
    let fixed =
        "def build(ffi):\n    ffi.reset()\n    name = \"struct ab\"\n    ffi.cdef(name)\n    ffi.verify(name)\n";
    assert_eq!((code, read(root.path(), "module.py")), (0, fixed.to_owned()));
}

// The words of a statement that spans lines count whole, since `ruff format` joins such a statement onto one line.
#[test]
fn restating_comment_counts_a_statement_across_its_lines() {
    let source = "def build(base):\n    # recursive selector\n    selector = make(\n        base, recursive=True\n    )\n    return selector\n";
    let root = project(&[("module.py", source)]);
    let (code, _, _) = fix_unsafe(root.path(), "SMP005");
    let fixed = "def build(base):\n    selector = make(\n        base, recursive=True\n    )\n    return selector\n";
    assert_eq!((code, read(root.path(), "module.py")), (0, fixed.to_owned()));
}

// The index holds the import that closes the cycle as a range of the source it read. The fix shortens the file, so the
// lint of the fixed source in the fix loop must not report that range, which now runs past its end.
#[test]
fn import_cycle_survives_a_fix_before_the_import() {
    let source = "def run():\n    value = \"a value long enough to move the import well back\"\n    print(value)\n\n\nimport b\n";
    let root = project(&[("a.py", source), ("b.py", "import a\n")]);
    let (code, stdout, _) = check(root.path(), &["--fix", "--select", "SMP308,SMP504"]);
    assert_eq!((code, stdout.contains("a.py:5:1: SMP308")), (1, true));
}
