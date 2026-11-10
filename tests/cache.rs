mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use test_case::test_case;

use common::{check, command, fix_unsafe, project, quoted, read, simplint_with};

#[test]
fn cache_is_kept_in_ruffs_cache_directory() {
    let root = project(&[("pyproject.toml", "[project]\nname = \"demo\"\n"), ("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    let cache = root.path().join(".ruff_cache");
    assert_eq!(
        (
            fs::read_to_string(cache.join("CACHEDIR.TAG")).expect("read"),
            fs::read_to_string(cache.join(".gitignore")).expect("read"),
            fs::read_dir(cache.join("simplint").join(concat!(
                env!("CARGO_PKG_VERSION"),
                "-",
                env!("SIMPLINT_CACHE_FORMAT")
            )))
            .expect("read")
            .count(),
        ),
        (
            "Signature: 8a477f597d28d172789f06886806bc55".to_owned(),
            "# Automatically created by ruff.\n*\n".to_owned(),
            1
        )
    );
}

#[test]
fn cache_leaves_ruffs_files_alone() {
    let ruff_files = [
        (".ruff_cache/CACHEDIR.TAG", "Signature: 8a477f597d28d172789f06886806bc55\n# by ruff\n"),
        (".ruff_cache/.gitignore", "# by ruff\n*\n"),
        (".ruff_cache/0.17.3/12345", "ruff's cache"),
    ];
    let mut files = vec![("module.py", "VALUE = 1\n")];
    files.extend_from_slice(&ruff_files);
    let root = project(&files);
    check(root.path(), &["--select", "SMP101"]);
    let contents: Vec<String> = ruff_files.iter().map(|(name, _)| read(root.path(), name)).collect();
    assert_eq!(contents, ruff_files.map(|(_, content)| content.to_owned()));
}

#[test]
fn no_cache_writes_nothing() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP101", "--no-cache"]);
    assert!(!root.path().join(".ruff_cache").exists());
}

#[test_case(&["--cache-dir", "elsewhere"], &[], &[] ; "flag")]
#[test_case(&[], &[], &[("RUFF_CACHE_DIR", "elsewhere")] ; "environment")]
#[test_case(&[], &[("pyproject.toml", "[tool.ruff]\ncache-dir = \"elsewhere\"\n")], &[] ; "pyproject")]
fn cache_dir_moves_the_cache(flags: &[&str], extra: &[(&str, &str)], env: &[(&str, &str)]) {
    let mut files = vec![("module.py", "VALUE = 1\n")];
    files.extend_from_slice(extra);
    let root = project(&files);
    let mut args = vec!["check", "--select", "SMP101"];
    args.extend_from_slice(flags);
    simplint_with(root.path(), &args, "", env);
    assert_eq!(
        (root.path().join("elsewhere/simplint").is_dir(), root.path().join(".ruff_cache").exists()),
        (true, false)
    );
}

#[test]
fn cache_is_not_written_when_nothing_changed() {
    let root = project(&[("module.py", "x = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    let cache = fs::read_dir(root.path().join(".ruff_cache/simplint").join(concat!(
        env!("CARGO_PKG_VERSION"),
        "-",
        env!("SIMPLINT_CACHE_FORMAT")
    )))
    .expect("read")
    .next()
    .expect("a cache file")
    .expect("entry")
    .path();
    let old = std::time::SystemTime::UNIX_EPOCH + Duration::from_hours(24);
    fs::File::options().write(true).open(&cache).expect("open").set_modified(old).expect("set modified time");
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(fs::metadata(&cache).expect("metadata").modified().expect("modified time"), old);
}

#[test]
fn unreadable_cache_is_replaced() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    let directory = root.path().join(".ruff_cache/simplint").join(concat!(
        env!("CARGO_PKG_VERSION"),
        "-",
        env!("SIMPLINT_CACHE_FORMAT")
    ));
    for entry in fs::read_dir(&directory).expect("read") {
        fs::write(entry.expect("entry").path(), "not a cache").expect("write");
    }
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    let (_, _, again) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("Failed to read cache file"), again), (0, true, String::new()));
}

#[test]
fn file_rewritten_under_the_same_modified_time_is_checked_again() {
    let root = project(&[("module.py", "# a comment that pads the file\nx = 1\n")]);
    let path = root.path().join("module.py");
    check(root.path(), &["--select", "SMP101"]);
    let modified = filetime::FileTime::from_last_modification_time(&fs::metadata(&path).expect("metadata"));
    fs::write(&path, "VALUE = 1\n").expect("write");
    filetime::set_file_mtime(&path, modified).expect("set modified time");
    assert_eq!(check(root.path(), &["--select", "SMP101"]), (0, "All checks passed!\n".to_owned(), String::new()));
}

#[test]
fn cache_directory_that_cannot_be_created_fails_the_run() {
    let root = project(&[("module.py", "VALUE = 1\n"), ("blocked", "")]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101", "--cache-dir", "blocked"]);
    assert_eq!(
        (code, stderr.contains("Failed to initialize cache"), stderr.contains("Failed to create cache directory")),
        (2, true, true)
    );
}

// A file this run did not check keeps its cached result, as Ruff keeps files seen within the last 30 days.
#[test]
fn cache_keeps_files_other_runs_checked() {
    let root = project(&[("module.py", "VALUE = 1\n"), ("other.py", "OTHER = 1\n")]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    check(root.path(), &["--select", "SMP101", "module.py"]);
    check(root.path(), &["--select", "SMP101", "other.py"]);
    fs::write(&path, "x = 12345\n").expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    assert_eq!(check(root.path(), &["--select", "SMP101", "module.py"]).0, 0);
}

#[test]
fn cached_clean_file_is_checked_again_when_another_file_starts_importing_it() {
    let root = project(&[("helpers.py", "def helper() -> None:\n    pass\n"), ("client.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP401"]).0;
    fs::write(root.path().join("client.py"), "from helpers import helper\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP401"]).0), (0, 1));
}

#[test]
fn cached_clean_file_is_checked_again_under_other_settings() {
    let root = project(&[("module.py", "x = 1\n")]);
    let first = check(root.path(), &["--select", "SMP001"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP101"]).0), (0, 1));
}

// Selecting a Ruff rule that SMP004 defers to changes what the file reports, so the cached report is not reused. The
// selection comes from Ruff's configuration on the command line, which leaves simplint's own settings unchanged.
#[test]
fn cached_report_is_checked_again_when_ruff_starts_running_the_rule() {
    let root = project(&[("module.py", "import os  # noqa\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP004"]).0, 1);
    assert_eq!(check(root.path(), &["--select", "SMP004", "--config", "lint.select = [\"PGH004\"]"]).0, 0);
}

// A pre-commit hook for flake8-lazy turns the rule off for the project, so the cached report is not reused.
#[test]
fn cached_absolute_sibling_import_report_is_checked_again_when_flake8_lazy_is_configured() {
    let root = project(&[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/sub/sibling.py", "VALUE = 1\n"),
        ("package/sub/module.py", "from package.sub import sibling\n"),
    ]);
    let first = check(root.path(), &["--select", "SMP302"]).0;
    fs::write(
        root.path().join(".pre-commit-config.yaml"),
        "repos:\n  - repo: local\n    hooks:\n      - id: flake8-lazy\n",
    )
    .expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP302"]).0), (1, 0));
}

// A function whose statements another file repeats, so `duplicate-code` reports both.
const REPEATED: &str = "def run(items):\n    total = 0\n    for item in items:\n        total += item\n    count = len(items)\n    \
                        log(total)\n    log(count)\n    return total\n";

// Each case checks the file once, edits the file named, and checks again: the cache must check it again, so the second
// run reports what the first could not.
#[test_case(
    &[("pyproject.toml", "[tool.simplint.naming]\nkeep-names = [\"limit\"]\n"), ("module.py", "limit = 3\n")],
    "SMP102", "pyproject.toml", "[tool.simplint]\n" ; "keep_names_setting_removed"
)]
#[test_case(
    &[("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"), ("pkg/b.py", "VALUE = 1\n")],
    "SMP302", "pkg/__init__.py", "" ; "directory_becomes_a_package"
)]
// With the directory a namespace package, a file's package root stays the same when `__init__.py` appears beside it, so
// the cache must still notice: `absolute-sibling-import` reads that file.
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\nnamespace-packages = [\"pkg\"]\n"),
        ("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"),
        ("pkg/b.py", "VALUE = 1\n"),
    ],
    "SMP302", "pkg/__init__.py", "" ; "namespace_package_gains_an_init_module"
)]
#[test_case(
    &[("cycle_a.py", "import cycle_b\n"), ("cycle_b.py", "import cycle_c\n"), ("cycle_c.py", "VALUE = 1\n")],
    "SMP308", "cycle_c.py", "import cycle_a\n" ; "import_closes_a_cycle_elsewhere"
)]
#[test_case(
    &[("first.py", REPEATED), ("second.py", "def run(items):\n    return items\n")],
    "SMP530", "second.py", REPEATED ; "another_file_repeats_its_statements"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"ws\"\n[tool.simplint]\n"),
        ("sub/pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("sub/demo/__init__.py", "from ._app import run as run\n"),
        ("sub/demo/_app.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n"),
    ],
    "SMP407", "sub/pyproject.toml", "[project]\nname = \"demo\"\n" ; "member_becomes_an_application"
)]
fn cached_clean_file_is_checked_again_after_an_edit(files: &[(&str, &str)], select: &str, edited: &str, content: &str) {
    let root = project(files);
    let first = check(root.path(), &["--select", select]).0;
    fs::write(root.path().join(edited), content).expect("write");
    assert_eq!((first, check(root.path(), &["--select", select]).0), (0, 1));
}

// A `hatch.toml` that starts configuring the custom build hook makes its class used, so the cached report is checked again.
#[test]
fn cached_hatch_hook_is_checked_again_once_hatch_toml_configures_it() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        (
            "hatch_build.py",
            "from hatchling.builders.hooks.plugin.interface import BuildHookInterface\n\n\nclass _Hook(BuildHookInterface):\n    pass\n",
        ),
    ]);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    fs::write(root.path().join("hatch.toml"), "[build.hooks.custom]\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP409"]).0), (1, 0));
}

// Only a rule that reads other files of the run keys a file's result by the whole run, so an edit to another file keeps
// the result of a file checked by per-file rules alone, and invalidates one checked by a cross-file rule.
#[test_case("SMP101", 0 ; "per_file_rule_keeps_the_cached_result")]
#[test_case("SMP401", 1 ; "cross_file_rule_checks_the_file_again")]
fn other_file_edit_invalidates_only_cross_file_selections(select: &str, expected: i32) {
    let root = project(&[("module.py", "VALUE = 1\nNAME = 1\n"), ("other.py", "OTHER = 1\n")]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    assert_eq!(check(root.path(), &["--select", select]).0, 0);
    fs::write(root.path().join("other.py"), "from module import VALUE\n").expect("write");
    fs::write(&path, "VALUE = 1\nx = 1234\n").expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    assert_eq!(check(root.path(), &["--select", select]).0, expected);
}

// A `pyproject.toml` that appears in a test directory makes it a nested project, which its duplicates are compared
// within, so the cached duplicates of the run must not be reused.
#[test]
fn nested_project_marker_added_is_checked_again() {
    let root = project(&[("pkg/parser.py", REPEATED), ("roots/test-a/parser.py", REPEATED)]);
    let first = check(root.path(), &["--select", "SMP530"]).0;
    fs::write(root.path().join("roots/test-a/pyproject.toml"), "[project]\nname = \"nested\"\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP530"]).0), (1, 0));
}

// A run without `duplicate-code` caches the facts without the statements it compares, so a later run that selects it
// collects them again.
#[test]
fn facts_cached_without_duplicate_code_are_collected_again_for_it() {
    let root = project(&[("first.py", REPEATED), ("second.py", REPEATED)]);
    let first = check(root.path(), &["--select", "SMP101"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP530"]).0), (0, 1));
}

// The fixture requests are a part of the facts only `unused-fixture` reads, so a run without it caches facts that lack
// them, and a later run that selects it collects them again rather than report a requested fixture.
#[test]
fn facts_cached_without_fixture_requests_are_collected_again_for_unused_fixture() {
    let root = project(&[
        ("conftest.py", "import pytest\n\n\n@pytest.fixture\ndef database():\n    return 1\n"),
        ("test_app.py", "def test_app(database):\n    assert database\n"),
    ]);
    let first = check(root.path(), &["--select", "SMP101"]).0;
    let cached = check(root.path(), &["--select", "SMP815"]).0;
    assert_eq!((first, cached, check(root.path(), &["--select", "SMP815", "--no-cache"]).0), (0, 0, 0));
}

// A project whose `helper` only its documentation names, which `unused-definition` reads.
fn documented_project(pages: &[(&'static str, &'static str)]) -> tempfile::TempDir {
    let mut files = vec![
        ("pyproject.toml", "[project]\nname = \"app\"\n"),
        ("app/__init__.py", ""),
        ("app/core.py", "def helper() -> None:\n    pass\n"),
    ];
    files.extend_from_slice(pages);
    project(&files)
}

// Like the files' facts, the documentation is read once and kept while its pages keep their modification times and
// sizes: an edit that keeps both goes unseen, which shows the run did not read the page again.
#[test]
fn documentation_is_reused_while_its_pages_are_unchanged() {
    let root = documented_project(&[("docs/api.rst", ":func:`app.core.helper`\n")]);
    let page = root.path().join("docs/api.rst");
    let modified = fs::metadata(&page).expect("metadata").modified().expect("modified time");
    let first = check(root.path(), &["--select", "SMP409"]).0;
    fs::write(&page, ":func:`app.core.helpex`\n").expect("write");
    fs::File::options().write(true).open(&page).expect("open").set_modified(modified).expect("set modified time");
    let cached = check(root.path(), &["--select", "SMP409"]).0;
    assert_eq!((first, cached, check(root.path(), &["--select", "SMP409", "--no-cache"]).0), (0, 0, 1));
}

#[test_case(&[("docs/api.rst", ":func:`app.core.helper`\n")], Some(":func:`app.core.other`\n"), (0, 1) ; "edited")]
#[test_case(&[("docs/api.rst", ":func:`app.core.helper`\n")], None, (0, 1) ; "removed")]
#[test_case(&[("docs/other.rst", "Nothing here.\n")], Some(":func:`app.core.helper`\n"), (1, 0) ; "added")]
fn documentation_is_read_again_once_a_page_changes(
    pages: &[(&'static str, &'static str)],
    api: Option<&str>,
    expected: (i32, i32),
) {
    let root = documented_project(pages);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    let page = root.path().join("docs/api.rst");
    match api {
        Some(text) => fs::write(&page, text).expect("write"),
        None => fs::remove_file(&page).expect("remove"),
    }
    assert_eq!((first, check(root.path(), &["--select", "SMP409"]).0), expected);
}

// An ignore file that comes to hide the documentation changes what the walk reads, though no page changed.
#[test]
fn documentation_is_read_again_once_an_ignore_file_hides_it() {
    let root =
        documented_project(&[(".git/HEAD", "ref: refs/heads/main\n"), ("docs/api.rst", ":func:`app.core.helper`\n")]);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    fs::write(root.path().join(".gitignore"), "docs/\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP409"]).0), (0, 1));
}

// Keeping the documentation is best effort, so a cache it cannot write is reported and the run goes on.
#[test]
fn documentation_cache_that_cannot_be_written_is_reported() {
    let root = documented_project(&[("docs/api.rst", ":func:`app.core.helper`\n"), ("blocked", "")]);
    let (_, _, stderr) = check(root.path(), &["--select", "SMP409", "--cache-dir", "blocked"]);
    assert!(stderr.contains("Failed to write the documentation cache"), "{stderr}");
}

#[test]
fn cached_clean_file_is_checked_again_once_edited() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP101"]).0;
    fs::write(root.path().join("module.py"), "x = 1\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP101"]).0), (0, 1));
}

// The cache trusts the modification time and size: an edit that keeps both goes unseen, which shows the run reused the
// cached result instead of linting again.
#[test]
fn cached_clean_result_is_reused() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    check(root.path(), &["--select", "SMP101"]);
    fs::write(&path, "x = 12345\n").expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    let cached = check(root.path(), &["--select", "SMP101"]).0;
    assert_eq!((cached, check(root.path(), &["--select", "SMP101", "--no-cache"]).0), (0, 1));
}

#[test]
fn ignore_noqa_bypasses_the_cache() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP101\n")]);
    let first = check(root.path(), &["--select", "SMP101"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP101", "--ignore-noqa"]).0), (0, 1));
}

#[test]
fn cache_tracks_skip_magic_trailing_comma() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.format]\nskip-magic-trailing-comma = true\n"),
        ("module.py", "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n"),
    ]);
    let quiet = check(root.path(), &["--select", "SMP405"]).0;
    fs::write(root.path().join("pyproject.toml"), "").expect("write");
    assert_eq!((quiet, check(root.path(), &["--select", "SMP405"]).0), (0, 1));
}

// Ruff's PLR2004 would report the value inlined into a comparison, so the fix loop withdraws SMP202's fix.
const RUFF_PLR2004: (&str, &str) = ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"PLR2004\"]\n");
const COMPARED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value > _LIMIT\n";
const ADDED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value + _LIMIT\n";

#[test_case(&["--fix", "--unsafe-fixes"] ; "fix")]
#[test_case(&["--fix"] ; "unsafe_fix_hint")]
#[test_case(&["--diff", "--unsafe-fixes"] ; "diff")]
#[test_case(&["--fix-only", "--unsafe-fixes"] ; "fix_only")]
#[test_case(&["--fix", "--unsafe-fixes", "--exit-non-zero-on-fix"] ; "exit_non_zero_on_fix")]
#[test_case(&["--fix", "--unsafe-fixes", "--statistics"] ; "statistics")]
fn settled_file_reports_the_same_from_the_cache(flags: &[&str]) {
    let root = project(&[RUFF_PLR2004, ("module.py", &format!("{COMPARED}\n\nx = 1\n"))]);
    let args = [&["--select", "SMP202,SMP101"], flags].concat();
    let (first, cached) = (check(root.path(), &args), check(root.path(), &args));
    assert_eq!((&first, &cached), (&check(root.path(), &[&args[..], &["--no-cache"]].concat()), &first));
}

// An edit that keeps the modification time and size goes unseen, so the cached withdrawal holds back a fix the loop would apply.
#[test]
fn settled_file_reuses_the_withdrawn_fixes() {
    let root = project(&[RUFF_PLR2004, ("module.py", COMPARED)]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    fix_unsafe(root.path(), "SMP202");
    fs::write(&path, ADDED).expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    let cached = fix_unsafe(root.path(), "SMP202").0;
    let unchanged = read(root.path(), "module.py");
    let fresh = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP202", "--no-cache"]).0;
    assert_eq!((cached, unchanged, fresh), (1, ADDED.to_owned(), 0));
}

#[test_case(&[("module.py", ADDED)], &["--fix"], None, &["--fix", "--unsafe-fixes"] ; "unsafe_fixes")]
#[test_case(&[RUFF_PLR2004, ("module.py", COMPARED)], &["--fix", "--unsafe-fixes"], Some(""), &["--fix", "--unsafe-fixes"] ; "ruff_settings")]
#[test_case(&[("pyproject.toml", "[tool.simplint]\nunfixable = [\"SMP202\"]\n"), ("module.py", ADDED)], &["--fix", "--unsafe-fixes"], Some(""), &["--fix", "--unsafe-fixes"] ; "fixable")]
#[test_case(&[("module.py", ADDED)], &["--fix"], Some("[tool.simplint]\nextend-safe-fixes = [\"SMP202\"]\n"), &["--fix"] ; "fix_safety")]
fn settled_file_is_fixed_again(files: &[(&str, &str)], first: &[&str], pyproject: Option<&str>, second: &[&str]) {
    let root = project(files);
    check(root.path(), &[first, &["--select", "SMP202"]].concat());
    if let Some(pyproject) = pyproject {
        fs::write(root.path().join("pyproject.toml"), pyproject).expect("write");
    }
    let code = check(root.path(), &[second, &["--select", "SMP202"]].concat()).0;
    let fixed = read(root.path(), "module.py");
    assert_eq!((code, fixed.contains("_LIMIT")), (0, false));
}

#[test]
fn settled_file_is_fixed_again_once_no_other_file_imports_the_name() {
    let root = project(&[("helpers.py", "limit = 3\n"), ("client.py", "from helpers import limit\n\nprint(limit)\n")]);
    let first = fix_unsafe(root.path(), "SMP102").0;
    fs::write(root.path().join("client.py"), "print(1)\n").expect("write");
    fix_unsafe(root.path(), "SMP102");
    assert_eq!((first, read(root.path(), "helpers.py")), (1, "LIMIT = 3\n".to_owned()));
}

// A cached clean result of a module stops holding once a package starts re-exporting one of its names.
#[test]
fn reexport_added_later_checks_the_source_again() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"pkg\"\n[build-system]\nrequires = []\n"),
        ("pkg/__init__.py", "\"\"\"Package.\"\"\"\n"),
        ("pkg/_core.py", "def run() -> None: ...\n"),
    ]);
    let first = check(root.path(), &["--select", "SMP903"]).1;
    fs::write(root.path().join("pkg/__init__.py"), "from ._core import run\n\n__all__ = [\"run\"]\n").expect("write");
    let second = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(
        (first, second.lines().next().map(|line| line.contains("`run`"))),
        ("All checks passed!\n".to_owned(), Some(true))
    );
}

// A name another file reads off a module counts as used; once it is a base class instead, the run's sets hold the same
// names in another order, and the cached clean result of the constant's module no longer holds.
#[test]
fn cached_clean_file_is_checked_again_once_a_name_moves_between_run_sets() {
    let root = project(&[
        ("module.py", COMPARED),
        ("reader.py", "import importlib\n\nloaded = importlib.import_module(\"module\")\nprint(loaded._LIMIT)\n"),
    ]);
    let first = check(root.path(), &["--select", "SMP202"]).0;
    fs::write(root.path().join("reader.py"), "class Limit(_LIMIT):\n    pass\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP202"]).0), (0, 1));
}

#[test]
fn caches_of_other_builds_expire_after_thirty_days() {
    let root = project(&[
        ("module.py", "VALUE = 1\n"),
        (".ruff_cache/simplint/0.0.1-00000000000000a1/1", "old"),
        (".ruff_cache/simplint/0.0.2-00000000000000b2/1", "new"),
    ]);
    let stale = root.path().join(".ruff_cache/simplint/0.0.1-00000000000000a1/1");
    let old = std::time::SystemTime::now() - Duration::from_hours(24 * 31);
    fs::File::options().write(true).open(&stale).expect("open").set_modified(old).expect("set modified time");
    check(root.path(), &["--select", "SMP101"]);
    let simplint = root.path().join(".ruff_cache/simplint");
    assert_eq!(
        (simplint.join("0.0.1-00000000000000a1").exists(), simplint.join("0.0.2-00000000000000b2").exists()),
        (false, true)
    );
}

// Another build's run creates its directory before it writes a cache there, so an empty directory goes by its own age.
#[test]
fn empty_caches_of_other_builds_expire_by_their_own_age() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let simplint = root.path().join(".ruff_cache/simplint");
    let (stale, fresh) = (simplint.join("0.0.1-00000000000000a1"), simplint.join("0.0.2-00000000000000b2"));
    fs::create_dir_all(&stale).expect("create");
    fs::create_dir_all(&fresh).expect("create");
    let old = filetime::FileTime::from_system_time(std::time::SystemTime::now() - Duration::from_hours(24 * 31));
    filetime::set_file_mtime(&stale, old).expect("set modified time");
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!((stale.exists(), fresh.exists()), (false, true));
}

// A `cache-dir` from the repository's configuration stays inside the project, so a PR cannot make simplint write its
// cache, or clean another directory, outside the run; the default location serves instead.
#[test]
fn cache_dir_outside_the_project_falls_back_to_the_default() {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let pyproject = format!("[tool.ruff]\ncache-dir = {}\n", quoted(outside.path()));
    let root = project(&[("pyproject.toml", pyproject.as_str()), ("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (root.path().join(".ruff_cache/simplint").is_dir(), fs::read_dir(outside.path()).expect("read").count()),
        (true, 0)
    );
}

#[cfg(unix)]
#[test]
fn cache_dir_through_a_link_outside_the_project_falls_back_to_the_default() {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let root =
        project(&[("pyproject.toml", "[tool.ruff]\ncache-dir = \"link/cache\"\n"), ("module.py", "VALUE = 1\n")]);
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).expect("symlink");
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (root.path().join(".ruff_cache/simplint").is_dir(), fs::read_dir(outside.path()).expect("read").count()),
        (true, 0)
    );
}

// A link committed in place of the default cache directory, or of simplint's directory in it, cannot point the cache
// out of the project either; with no directory inside it left, the run does not cache.
#[cfg(unix)]
#[test_case(&[], ".ruff_cache" ; "default_without_configuration")]
#[test_case(&[("pyproject.toml", "[tool.ruff]\n")], ".ruff_cache" ; "default")]
#[test_case(&[("pyproject.toml", "[tool.ruff]\n"), (".ruff_cache/keep", "")], ".ruff_cache/simplint" ; "simplint")]
fn cache_directory_linked_outside_the_project_is_not_used(extra: &[(&str, &str)], link: &str) {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let root = project(&[extra, &[("module.py", "VALUE = 1\n")]].concat());
    std::os::unix::fs::symlink(outside.path(), root.path().join(link)).expect("symlink");
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (code, stderr.contains("Not caching"), fs::read_dir(outside.path()).expect("read").count()),
        (0, true, 0)
    );
}

#[cfg(unix)]
#[test]
fn show_settings_names_no_cache_directory_when_it_leads_outside_the_project() {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let root = project(&[("module.py", "VALUE = 1\n")]);
    std::os::unix::fs::symlink(outside.path(), root.path().join(".ruff_cache")).expect("symlink");
    assert!(check(root.path(), &["--show-settings"]).1.contains("\ncache_dir = none\n"));
}

// The command line and `RUFF_CACHE_DIR` name the directory the user chose, so they may point anywhere.
#[test]
fn cache_dir_flag_may_point_outside_the_project() {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let root = project(&[("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP101", "--cache-dir", outside.path().to_str().expect("utf-8 path")]);
    assert_eq!((outside.path().join("simplint").is_dir(), root.path().join(".ruff_cache").exists()), (true, false));
}

// A stale build directory is removed only when it has the shape of one and holds nothing but cache files; anything
// else in the `simplint` directory stays, however old.
#[test_case("notes", "notes.txt" ; "name_is_not_a_build")]
#[test_case("0.0.1-00000000000000a1", "notes.txt" ; "holds_a_file_other_than_a_cache")]
fn stale_directories_that_are_not_caches_of_other_builds_stay(name: &str, file: &str) {
    let path = format!(".ruff_cache/simplint/{name}/{file}");
    let root = project(&[("module.py", "VALUE = 1\n"), (path.as_str(), "kept")]);
    let old = filetime::FileTime::from_system_time(std::time::SystemTime::now() - Duration::from_hours(24 * 31));
    filetime::set_file_mtime(root.path().join(&path), old).expect("set modified time");
    check(root.path(), &["--select", "SMP101"]);
    assert!(root.path().join(&path).exists());
}

// A link named like a build directory is not one, and what it points at is not the run's to remove.
#[cfg(unix)]
#[test]
fn link_in_the_cache_directory_is_kept_with_its_target() {
    let outside = tempfile::TempDir::new().expect("temporary directory");
    let target = outside.path().join("1");
    fs::write(&target, "kept").expect("write");
    let old = filetime::FileTime::from_system_time(std::time::SystemTime::now() - Duration::from_hours(24 * 31));
    filetime::set_file_mtime(&target, old).expect("set modified time");
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let simplint = root.path().join(".ruff_cache/simplint");
    fs::create_dir_all(&simplint).expect("create");
    let link = simplint.join("0.0.1-00000000000000a1");
    std::os::unix::fs::symlink(outside.path(), &link).expect("symlink");
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (link.symlink_metadata().is_ok(), fs::read_to_string(&target).expect("read")),
        (true, "kept".to_owned())
    );
}

// Findings with fixes, help notes, a suppression comment and one without a fix, as a run without fixes reports them.
const FINDINGS: &str = "import os\n\nVALUE = 1\n\n\ndef f(x):\n    y = 2\n    return x\n\n\ndef g(n):\n    for i in range(n):\n        pass\n    z = 3  # noqa: SMP101\n    return \"a\" + \"b\"\n";

// Reports cached without fixes come back as a run without the cache reports them: with their fixes, help notes and
// suppression comments applied as they are in a fresh run.
#[test_case(&[] ; "concise")]
#[test_case(&["--output-format", "full"] ; "full")]
#[test_case(&["--output-format", "json"] ; "json")]
fn cached_findings_report_the_same_as_a_fresh_run(flags: &[&str]) {
    let root = project(&[("module.py", FINDINGS)]);
    let args = [&["--select", "SMP003,SMP101,SMP404,SMP503,SMP601"], flags].concat();
    let fresh = check(root.path(), &[&args[..], &["--no-cache"]].concat());
    let (first, cached) = (check(root.path(), &args), check(root.path(), &args));
    assert_eq!((&first, &cached), (&fresh, &fresh));
}

#[test]
fn cached_findings_are_checked_again_once_edited() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP601"]).0;
    fs::write(root.path().join("module.py"), "from typing import Final\n\nVALUE: Final[int] = 1\n").expect("write");
    let second = check(root.path(), &["--select", "SMP601"]).0;
    assert_eq!((first, second), (1, 0));
}

#[test]
fn cached_findings_are_reported_again_under_other_settings() {
    let root = project(&[("module.py", FINDINGS)]);
    check(root.path(), &["--select", "SMP601"]);
    let args = ["--select", "SMP101"];
    let cached = check(root.path(), &args);
    assert_eq!(cached, check(root.path(), &[&args[..], &["--no-cache"]].concat()));
}

const HELPER: &str = "def helper() -> None:\n    pass\n";
const CLIENT: &str = "from helpers import helper\n";

#[derive(Clone, Copy)]
enum Change<'a> {
    Write(&'a str, &'a str),
    Remove(&'a str),
}

// The index a warm run reuses its reports from is built from the run's files and settings. Once any of those change,
// the next warm run must answer as a run without the cache does, so each change to an index input is checked against
// a fresh run.
fn warm_run_after_change_matches_a_fresh_run(files: &[(&str, &str)], change: Change, select: &str, targets: &[&str]) {
    let root = project(files);
    let args = [&["--select", select], targets].concat();
    check(root.path(), &args);
    match change {
        Change::Write(name, content) => {
            let path = root.path().join(name);
            fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
            fs::write(path, content).expect("write");
        }
        Change::Remove(name) => fs::remove_file(root.path().join(name)).expect("remove"),
    }
    assert_eq!(check(root.path(), &args), check(root.path(), &[&args[..], &["--no-cache"]].concat()));
}

// `helpers.py` is reported while nothing imports it, so each change to the importer flips its reports, and a warm run
// that reused them from the first run would miss the flip.
#[test_case(&[("helpers.py", HELPER), ("client.py", "VALUE = 1\n")], Change::Write("client.py", CLIENT), "SMP401", &["."] ; "importer_edited")]
#[test_case(&[("helpers.py", HELPER)], Change::Write("client.py", CLIENT), "SMP401", &["."] ; "importer_added")]
#[test_case(&[("helpers.py", HELPER), ("client.py", CLIENT)], Change::Remove("client.py"), "SMP401", &["."] ; "importer_removed")]
#[test_case(
    &[("pyproject.toml", "[tool.ruff]\nnamespace-packages = [\"pkg\"]\n"), ("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"), ("pkg/b.py", "VALUE = 1\n")],
    Change::Write("pkg/__init__.py", ""), "SMP302", &["."] ; "namespace_package_gains_an_init_module"
)]
#[test_case(
    &[("lib/helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Write("pyproject.toml", "[tool.ruff]\nsrc = [\"lib\"]\n"), "SMP401", &["."] ; "source_root_changed"
)]
#[test_case(
    &[("helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Write("pyproject.toml", "[tool.ruff]\nsrc = [\"missing\"]\n"), "SMP401", &["."] ; "source_root_removed"
)]
#[test_case(&[("client.py", CLIENT), ("helpers.py", HELPER)], Change::Remove("client.py"), "SMP401", &["client.py"] ; "linted_file_removed")]
#[test_case(
    &[("pyproject.toml", "[tool.ruff]\nsrc = [\"lib\", \".\"]\nextend-exclude = [\"lib\"]\n"), ("lib/notes.txt", ""), ("helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Write("lib/helpers.py", HELPER), "SMP401", &["."] ; "excluded_module_shadows_an_import"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"proj\"\n\n[tool.ruff]\nextend-exclude = [\"scripts-dir/helpers\"]\n"),
        ("scripts-dir/run.py", "from helpers.util import thing\n\nprint(thing)\n"),
        ("other/helpers.py", "def thing() -> int:\n    return 1\n"),
        ("scripts-dir/helpers/util.py", "thing = 1\n"),
    ],
    Change::Remove("scripts-dir/helpers/util.py"), "SMP409", &["."] ; "module_below_an_unimportable_directory_removed"
)]
fn warm_run_after_index_input_changes_matches_a_fresh_run(
    files: &[(&str, &str)],
    change: Change,
    select: &str,
    targets: &[&str],
) {
    warm_run_after_change_matches_a_fresh_run(files, change, select, targets);
}

// A file outside the linted paths that the index resolves an import to: adding it changes the index though the run's
// files stay the same.
#[test]
fn warm_run_after_an_unlinted_module_appears_matches_a_fresh_run() {
    warm_run_after_change_matches_a_fresh_run(
        &[("src/client.py", CLIENT), ("src/helpers.py", "")],
        Change::Write("src/helpers.py", HELPER),
        "SMP401",
        &["src/client.py"],
    );
}

// A directory a link names resolves through the link, so a module removed behind it changes the index as one removed
// in place does.
#[cfg(unix)]
#[test]
fn warm_run_after_a_module_behind_a_link_is_removed_matches_a_fresh_run() {
    let root = project(&[
        ("proj/pyproject.toml", "[project]\nname = \"proj\"\n\n[tool.ruff]\nextend-exclude = [\"vendor\"]\n"),
        ("proj/app.py", "from vendor.util import thing\n\nprint(thing)\n"),
        ("proj/other/vendor.py", "def thing() -> int:\n    return 1\n"),
        ("shared/vendor/util.py", "thing = 1\n"),
    ]);
    std::os::unix::fs::symlink(root.path().join("shared/vendor"), root.path().join("proj/vendor")).expect("link");
    let project_root = root.path().join("proj");
    let args = ["--select", "SMP409"];
    check(&project_root, &args);
    fs::remove_file(root.path().join("shared/vendor/util.py")).expect("remove");
    assert_eq!(check(&project_root, &args), check(&project_root, &[&args[..], &["--no-cache"]].concat()));
}

// A run that reuses every file's result skips the index; after an edit, the files reused unchanged are marked with
// the new index too, so the run after it skips the index again.
#[test]
fn warm_run_skips_the_index_again_after_an_edit() {
    let root = project(&[("helpers.py", HELPER), ("client.py", CLIENT), ("other.py", "VALUE = 1\n")]);
    let indexed = |root: &std::path::Path| check(root, &["--select", "SMP401", "-v"]).2.contains("Indexed ");
    let before = [indexed(root.path()), indexed(root.path())];
    fs::write(root.path().join("other.py"), "VALUE = 2\n").expect("write");
    let after = [indexed(root.path()), indexed(root.path())];
    assert_eq!((before, after), ([true, false], [true, false]));
}

#[test_case(":func:`app.core.helper`\n", ":func:`app.core.other`\n" ; "page_edited")]
fn warm_run_after_documentation_changes_matches_a_fresh_run(before: &'static str, after: &str) {
    let root = documented_project(&[("docs/api.rst", before)]);
    let args = ["--select", "SMP409"];
    check(root.path(), &args);
    fs::write(root.path().join("docs/api.rst"), after).expect("write");
    assert_eq!(check(root.path(), &args), check(root.path(), &[&args[..], &["--no-cache"]].concat()));
}

// A safe fix and an unsafe one, so the `full` text shows the fix marker and the diff of each.
const FIXABLE: &str = "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";

fn full_report(root: &Path, flags: &[&str], env: &[(&str, &str)]) -> String {
    let args = [&["check", "--output-format", "full", "--select", "SMP001,SMP901", "--exit-zero"], flags].concat();
    simplint_with(root, &args, "", env).1
}

fn cache_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root.join(".ruff_cache/simplint").join(concat!(
        env!("CARGO_PKG_VERSION"),
        "-",
        env!("SIMPLINT_CACHE_FORMAT")
    )))
    .expect("read")
    .map(|entry| entry.expect("entry").path())
    .collect()
}

// The `full` text is kept beside the package's cache file on the first `full` run after a concise one, and a later run
// that shows the same text writes no cache file.
#[test]
fn full_text_is_stored_once_and_reused() {
    let root = project(&[("module.py", FIXABLE)]);
    check(root.path(), &["--select", "SMP001,SMP901"]);
    let size = |files: &[PathBuf]| files.iter().map(|file| fs::metadata(file).expect("metadata").len()).sum::<u64>();
    let reports_only = size(&cache_files(root.path()));
    full_report(root.path(), &[], &[]);
    let with_text = size(&cache_files(root.path()));
    let old = SystemTime::UNIX_EPOCH + Duration::from_hours(24);
    for file in cache_files(root.path()) {
        fs::File::options().write(true).open(&file).expect("open").set_modified(old).expect("set modified time");
    }
    full_report(root.path(), &[], &[]);
    let modified: Vec<SystemTime> = cache_files(root.path())
        .iter()
        .map(|file| fs::metadata(file).expect("metadata").modified().expect("modified time"))
        .collect();
    assert_eq!((with_text > reports_only, modified.iter().all(|time| *time == old)), (true, true));
}

// Each display setting the `full` text depends on invalidates the text a run stored under the setting before.
#[test_case(&[], &[], &["--preview"], &[] ; "preview")]
#[test_case(&[], &[], &["--color", "always"], &[] ; "color")]
#[test_case(
    &["--color", "always"],
    &[("FORCE_HYPERLINK", "0")],
    &["--color", "always"],
    &[("FORCE_HYPERLINK", "1")] ;
    "hyperlinks"
)]
#[test_case(&[], &[], &["--unsafe-fixes"], &[] ; "unsafe_fixes")]
fn full_text_follows_the_display_settings(
    prior: &[&str],
    prior_env: &[(&str, &str)],
    flags: &[&str],
    env: &[(&str, &str)],
) {
    let root = project(&[("module.py", FIXABLE)]);
    let prior_fresh = full_report(root.path(), &[prior, &["--no-cache"]].concat(), prior_env);
    full_report(root.path(), prior, prior_env);
    full_report(root.path(), prior, prior_env);
    let warm = full_report(root.path(), flags, env);
    let fresh = full_report(root.path(), &[flags, &["--no-cache"]].concat(), env);
    assert_eq!((warm == fresh, warm != prior_fresh), (true, true));
}

// The text names the file the way the run named it, so a run that names it another way renders it again.
#[test]
fn full_text_follows_the_path_it_names() {
    let root = project(&[("module.py", FIXABLE)]);
    full_report(root.path(), &["."], &[]);
    full_report(root.path(), &["."], &[]);
    let warm = full_report(root.path(), &["module.py"], &[]);
    assert_eq!(warm, full_report(root.path(), &["module.py", "--no-cache"], &[]));
}

// Ruff shows an absolute path relative to the working directory, so one name shows differently from another directory.
#[test]
fn full_text_follows_the_working_directory() {
    let root = project(&[("module.py", FIXABLE), ("sub/keep.txt", "")]);
    let file = root.path().join("module.py");
    let file = file.to_str().expect("utf-8 path");
    let full_in = |dir: &Path, args: &[&str]| {
        let args = [&["check", "--output-format", "full", "--select", "SMP001,SMP901", "--exit-zero"], args].concat();
        String::from_utf8(command(dir).args(&args).output().expect("runs").stdout).expect("utf-8 stdout")
    };
    full_in(root.path(), &[file]);
    full_in(root.path(), &[file]);
    let warm = full_in(&root.path().join("sub"), &[file]);
    assert_eq!(warm, full_in(&root.path().join("sub"), &[file, "--no-cache"]));
}

// The broken symlinks have no cache entry, so their text is never stored and their groups are rendered when the output
// is written, around the file whose stored text is written as it is.
#[cfg(unix)]
#[test]
fn full_text_is_rendered_for_a_file_without_a_cache_entry() {
    let root = project(&[("module.py", FIXABLE)]);
    std::os::unix::fs::symlink("missing.py", root.path().join("a_broken.py")).expect("symlink");
    std::os::unix::fs::symlink("missing.py", root.path().join("z_broken.py")).expect("symlink");
    full_report(root.path(), &[], &[]);
    let warm = full_report(root.path(), &[], &[]);
    assert_eq!(warm, full_report(root.path(), &["--no-cache"], &[]));
}

// A file that joins a run changes the run's index, so the files already reported are checked again, and those whose
// reports the fingerprint still matches keep their text.
#[test]
fn full_text_is_kept_when_another_file_joins_the_run() {
    let root = project(&[("module.py", FIXABLE)]);
    full_report(root.path(), &[], &[]);
    fs::write(root.path().join("other.py"), "VALUE = 1\n").expect("write");
    let warm = full_report(root.path(), &[], &[]);
    assert_eq!(warm, full_report(root.path(), &["--no-cache"], &[]));
}

// A relative import in a namespace package resolves under the search root, so a cached file's facts follow the roots.
#[test]
fn cached_facts_follow_a_changed_search_root() {
    let pyproject = "[project]\nname = \"demo\"\n[tool.ruff]\nsrc = [\"{root}\"]\n";
    let root = project(&[
        ("pyproject.toml", &pyproject.replace("{root}", "lib")),
        ("src/ns/helper.py", "def used() -> int:\n    return 1\n"),
        ("src/ns/main.py", "from .helper import used\n\nprint(used())\n"),
    ]);
    let before = check(root.path(), &["--select", "SMP409"]).0;
    fs::write(root.path().join("pyproject.toml"), pyproject.replace("{root}", "src")).expect("write");
    assert_eq!((before, check(root.path(), &["--select", "SMP409"]).0), (1, 0));
}
