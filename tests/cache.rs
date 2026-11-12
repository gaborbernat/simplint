mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tempfile::TempDir;
use test_case::test_case;

use common::{
    CLEAN_MODULE, COMPARED, PASSED, RUFF_PLR2004, UNPARSED, check, check_in, empty_project, fix_unsafe, make_read_only,
    project, quoted, read, simplint, simplint_with,
};

#[test]
fn cache_is_kept_in_ruffs_cache_directory() {
    let root = project(&[("pyproject.toml", "[project]\nname = \"demo\"\n"), ("module.py", CLEAN_MODULE)]);
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (
            read(root.path(), ".ruff_cache/CACHEDIR.TAG"),
            read(root.path(), ".ruff_cache/.gitignore"),
            fs::read_dir(cache_dir(root.path())).expect("read").count(),
        ),
        (
            "Signature: 8a477f597d28d172789f06886806bc55".to_owned(),
            "# Automatically created by ruff.\n*\n".to_owned(),
            1
        )
    );
}

fn cache_dir(root: &Path) -> PathBuf {
    root.join(".ruff_cache/simplint").join(concat!(env!("CARGO_PKG_VERSION"), "-", env!("SIMPLINT_CACHE_FORMAT")))
}

#[test]
fn cache_leaves_ruffs_files_alone() {
    let ruff_files = [
        (".ruff_cache/CACHEDIR.TAG", "Signature: 8a477f597d28d172789f06886806bc55\n# by ruff\n"),
        (".ruff_cache/.gitignore", "# by ruff\n*\n"),
        (".ruff_cache/0.17.3/12345", "ruff's cache"),
    ];
    let mut files = vec![("module.py", CLEAN_MODULE)];
    files.extend_from_slice(&ruff_files);
    let root = project(&files);
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(ruff_files.map(|(name, _)| read(root.path(), name)), ruff_files.map(|(_, content)| content.to_owned()));
}

#[test]
fn no_cache_writes_nothing() {
    let root = project(&[("module.py", CLEAN_MODULE)]);
    check(root.path(), &["--select", "SMP101", "--no-cache"]);
    assert!(!root.path().join(".ruff_cache").exists());
}

#[test_case(&["--cache-dir", "elsewhere"], &[], &[] ; "flag")]
#[test_case(&[], &[], &[("RUFF_CACHE_DIR", "elsewhere")] ; "environment")]
#[test_case(&[], &[("pyproject.toml", "[tool.ruff]\ncache-dir = \"elsewhere\"\n")], &[] ; "pyproject")]
fn cache_dir_moves_the_cache(flags: &[&str], extra: &[(&str, &str)], env: &[(&str, &str)]) {
    let mut files = vec![("module.py", CLEAN_MODULE)];
    files.extend_from_slice(extra);
    let root = project(&files);
    simplint_with(root.path(), &[&["check", "--select", "SMP101"], flags].concat(), "", env);
    assert_eq!(
        (root.path().join("elsewhere/simplint").is_dir(), root.path().join(".ruff_cache").exists()),
        (true, false)
    );
}

#[test]
fn cache_is_not_written_when_nothing_changed() {
    let root = project(&[("module.py", "x = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    let cache =
        fs::read_dir(cache_dir(root.path())).expect("read").next().expect("a cache file").expect("entry").path();
    let old = SystemTime::UNIX_EPOCH + Duration::from_hours(24);
    set_mtime(&cache, old);
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(fs::metadata(&cache).expect("metadata").modified().expect("modified time"), old);
}

fn set_mtime(path: &Path, time: SystemTime) {
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(time)).expect("set modified time");
}

#[test]
fn unreadable_cache_is_replaced() {
    let root = project(&[("module.py", CLEAN_MODULE)]);
    check(root.path(), &["--select", "SMP101"]);
    for entry in fs::read_dir(cache_dir(root.path())).expect("read") {
        fs::write(entry.expect("entry").path(), "not a cache").expect("write");
    }
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    let (_, _, again) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("Failed to read cache file"), again), (0, true, String::new()));
}

#[test]
fn file_rewritten_under_the_same_modified_time_is_checked_again() {
    let root = project(&[("module.py", "# a comment that pads the file\nx = 1\n")]);
    check(root.path(), &["--select", "SMP101"]);
    rewrite_keeping_mtime(&root.path().join("module.py"), CLEAN_MODULE);
    assert_eq!(check(root.path(), &["--select", "SMP101"]), (0, PASSED.to_owned(), String::new()));
}

fn rewrite_keeping_mtime(path: &Path, content: &str) {
    let modified = fs::metadata(path).expect("metadata").modified().expect("modified time");
    fs::write(path, content).expect("write");
    set_mtime(path, modified);
}

#[test]
fn cache_directory_that_cannot_be_created_fails_the_run() {
    let root = project(&[("module.py", CLEAN_MODULE), ("blocked", "")]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101", "--cache-dir", "blocked"]);
    assert_eq!(
        (code, stderr.contains("Failed to initialize cache"), stderr.contains("Failed to create cache directory")),
        (2, true, true)
    );
}

// A file this run did not check keeps its cached result, as Ruff keeps files seen within the last 30 days.
#[test]
fn cache_keeps_files_other_runs_checked() {
    let root = project(&[("module.py", CLEAN_MODULE), ("other.py", "OTHER = 1\n")]);
    check(root.path(), &["--select", "SMP101", "module.py"]);
    check(root.path(), &["--select", "SMP101", "other.py"]);
    rewrite_keeping_mtime(&root.path().join("module.py"), "x = 12345\n");
    assert_eq!(check(root.path(), &["--select", "SMP101", "module.py"]).0, 0);
}

#[test]
fn cached_clean_file_is_checked_again_under_other_settings() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!((check(root.path(), &["--select", "SMP001"]).0, check(root.path(), &["--select", "SMP101"]).0), (0, 1));
}

// Selecting a Ruff rule that SMP903 defers to changes what the file reports, so the cached report is not reused. The
// selection comes from Ruff's configuration on the command line, which leaves simplint's own settings unchanged.
#[test]
fn cached_report_is_checked_again_when_ruff_starts_running_the_rule() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("module.py", "def run() -> None:\n    pass\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP903"]).0, 1);
    assert_eq!(check(root.path(), &["--select", "SMP903", "--config", "lint.select = [\"D103\"]"]).0, 0);
}

// Each case checks the file once, edits the file named, and checks again: the cache must check it again, so the second
// run reports what the first could not.
#[test_case(
    &[("pyproject.toml", "[tool.simplint.naming]\nkeep-names = [\"limit\"]\n"), ("module.py", "limit = 3\n")],
    "SMP102",
    Change::Write("pyproject.toml", "[tool.simplint]\n"),
    (0, 1) ; "keep_names_setting_removed"
)]
#[test_case(
    &[("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"), ("pkg/b.py", CLEAN_MODULE)],
    "SMP302",
    Change::Write("pkg/__init__.py", ""),
    (0, 1) ; "directory_becomes_a_package"
)]
// With the directory a namespace package, a file's package root stays the same when `__init__.py` appears beside it, so
// the cache must still notice: `absolute-sibling-import` reads that file.
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\nnamespace-packages = [\"pkg\"]\n"),
        ("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"),
        ("pkg/b.py", CLEAN_MODULE),
    ],
    "SMP302",
    Change::Write("pkg/__init__.py", ""),
    (0, 1) ; "namespace_package_gains_an_init_module"
)]
#[test_case(
    &[("cycle_a.py", "import cycle_b\n"), ("cycle_b.py", "import cycle_c\n"), ("cycle_c.py", CLEAN_MODULE)],
    "SMP308",
    Change::Write("cycle_c.py", "import cycle_a\n"),
    (0, 1) ; "import_closes_a_cycle_elsewhere"
)]
#[test_case(
    &[("first.py", REPEATED), ("second.py", "def run(items):\n    return items\n")],
    "SMP530",
    Change::Write("second.py", REPEATED),
    (0, 1) ; "another_file_repeats_its_statements"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"ws\"\n[tool.simplint]\n"),
        ("sub/pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("sub/demo/__init__.py", "from ._app import run as run\n"),
        ("sub/demo/_app.py", "def run() -> None:\n    \"\"\"Run.\"\"\"\n"),
    ],
    "SMP407",
    Change::Write("sub/pyproject.toml", "[project]\nname = \"demo\"\n"),
    (0, 1) ; "member_becomes_an_application"
)]
#[test_case(
    &[("helpers.py", "def helper() -> None:\n    pass\n"), ("client.py", CLEAN_MODULE)],
    "SMP401",
    Change::Write("client.py", "from helpers import helper\n"),
    (0, 1) ; "another_file_starts_importing_it"
)]
// A pre-commit hook for flake8-lazy turns the rule off for the project, so the cached report is not reused.
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/sub/sibling.py", CLEAN_MODULE),
        ("package/sub/module.py", "from package.sub import sibling\n"),
    ],
    "SMP302",
    Change::Write(".pre-commit-config.yaml", "repos:\n  - repo: local\n    hooks:\n      - id: flake8-lazy\n"),
    (1, 0) ; "flake8_lazy_configured"
)]
// A `hatch.toml` that starts configuring the custom build hook makes its class used, so the cached report is checked
// again.
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        (
            "hatch_build.py",
            "from hatchling.builders.hooks.plugin.interface import BuildHookInterface\n\n\n\
             class _Hook(BuildHookInterface):\n    pass\n",
        ),
    ],
    "SMP409",
    Change::Write("hatch.toml", "[build.hooks.custom]\n"),
    (1, 0) ; "hatch_hook_configured"
)]
// A `pyproject.toml` that appears in a test directory makes it a nested project, which its duplicates are compared
// within, so the cached duplicates of the run must not be reused.
#[test_case(
    &[("pkg/parser.py", REPEATED), ("roots/test-a/parser.py", REPEATED)],
    "SMP530",
    Change::Write("roots/test-a/pyproject.toml", "[project]\nname = \"nested\"\n"),
    (1, 0) ; "nested_project_marker_added"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff.format]\nskip-magic-trailing-comma = true\n"),
        ("module.py", "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n"),
    ],
    "SMP405",
    Change::Write("pyproject.toml", ""),
    (0, 1) ; "skip_magic_trailing_comma_removed"
)]
#[test_case(
    &[("module.py", CLEAN_MODULE)],
    "SMP101",
    Change::Write("module.py", "x = 1\n"),
    (0, 1)
    ; "clean_file_edited"
)]
// A name another file reads off a module counts as used; once it is a base class instead, the run's sets hold the same
// names in another order, and the cached clean result of the constant's module no longer holds.
#[test_case(
    &[
        ("module.py", COMPARED),
        ("reader.py", "import importlib\n\nloaded = importlib.import_module(\"module\")\nprint(loaded._LIMIT)\n"),
    ],
    "SMP202",
    Change::Write("reader.py", "class Limit(_LIMIT):\n    pass\n"),
    (0, 1) ; "name_moves_between_run_sets"
)]
#[test_case(
    &[("module.py", CLEAN_MODULE)],
    "SMP601",
    Change::Write("module.py", "from typing import Final\n\nVALUE: Final[int] = 1\n"),
    (1, 0) ; "findings_edited"
)]
// A relative import in a namespace package resolves under the search root, so a cached file's facts follow the roots.
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.ruff]\nsrc = [\"lib\"]\n"),
        ("src/ns/helper.py", "def used() -> int:\n    return 1\n"),
        ("src/ns/main.py", "from .helper import used\n\nprint(used())\n"),
    ],
    "SMP409",
    Change::Write("pyproject.toml", "[project]\nname = \"demo\"\n[tool.ruff]\nsrc = [\"src\"]\n"),
    (1, 0) ; "search_root_changed"
)]
fn cached_result_is_checked_again_after_an_edit(
    files: &[(&str, &str)],
    select: &str,
    change: Change,
    expected: (i32, i32),
) {
    let root = project(files);
    let first = check(root.path(), &["--select", select]).0;
    apply(root.path(), change);
    assert_eq!((first, check(root.path(), &["--select", select]).0), expected);
}

// A function whose statements another file repeats, so `duplicate-code` reports both.
const REPEATED: &str = "def run(items):\n    total = 0\n    for item in \
                        items:\n        total += item\n    count = len(items)\n    \
                        log(total)\n    log(count)\n    return total\n";

#[derive(Clone, Copy)]
enum Change<'a> {
    Write(&'a str, &'a str),
    Remove(&'a str),
    Move(&'a str, &'a str),
}

fn apply(root: &Path, change: Change) {
    match change {
        Change::Write(name, content) => write_file(root, name, content),
        Change::Remove(name) => fs::remove_file(root.join(name)).expect("remove"),
        Change::Move(from, to) => {
            write_file_dirs(root, to);
            fs::rename(root.join(from), root.join(to)).expect("move");
        }
    }
}

fn write_file_dirs(root: &Path, name: &str) {
    fs::create_dir_all(root.join(name).parent().expect("file has a parent")).expect("create parent");
}

fn write_file(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
    fs::write(path, content).expect("write");
}

// A back-import from another file closes an import cycle with the module, which withdraws the fix the cached report
// offered, and removing the back-import offers it again.
#[test]
fn import_closing_a_cycle_in_another_file_withdraws_the_cached_fix() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/a.py", "from . import b\n\nVALUE = 1\n\n\ndef run() -> int:\n    return b.helper()\n"),
        ("pkg/b.py", "def helper() -> int:\n    return 1\n"),
    ]);
    let fixable = |root: &Path| check(root, &["--select", "SMP304"]).1.contains("unsafe fix");
    let before = fixable(root.path());
    write_file(root.path(), "pkg/b.py", "from . import a\n\n\ndef helper() -> int:\n    return a.VALUE\n");
    let closed = fixable(root.path());
    write_file(root.path(), "pkg/b.py", "def helper() -> int:\n    return 1\n");
    assert_eq!((before, closed, fixable(root.path())), (true, false, true));
}

// Only a rule that reads other files of the run keys a file's result by the whole run, so an edit to another file keeps
// the result of a file checked by per-file rules alone, and invalidates one checked by a cross-file rule.
#[test_case("SMP101", 0 ; "per_file_rule_keeps_the_cached_result")]
#[test_case("SMP401", 1 ; "cross_file_rule_checks_the_file_again")]
fn other_file_edit_invalidates_only_cross_file_selections(select: &str, expected: i32) {
    let root = project(&[("module.py", "VALUE = 1\nNAME = 1\n"), ("other.py", "OTHER = 1\n")]);
    assert_eq!(check(root.path(), &["--select", select]).0, 0);
    write_file(root.path(), "other.py", "from module import VALUE\n");
    rewrite_keeping_mtime(&root.path().join("module.py"), "VALUE = 1\nx = 1234\n");
    assert_eq!(check(root.path(), &["--select", select]).0, expected);
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

// Like the files' facts, the documentation is read once and kept while its pages keep their modification times and
// sizes: an edit that keeps both goes unseen, which shows the run did not read the page again.
#[test]
fn documentation_is_reused_while_its_pages_are_unchanged() {
    let root = documented_project(&[("docs/api.rst", ":func:`app.core.helper`\n")]);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    rewrite_keeping_mtime(&root.path().join("docs/api.rst"), ":func:`app.core.helpex`\n");
    let cached = check(root.path(), &["--select", "SMP409"]).0;
    assert_eq!((first, cached, check(root.path(), &["--select", "SMP409", "--no-cache"]).0), (0, 0, 1));
}

// A project whose `helper` only its documentation names, which `unused-definition` reads.
fn documented_project(pages: &[(&'static str, &'static str)]) -> TempDir {
    let mut files = vec![
        ("pyproject.toml", "[project]\nname = \"app\"\n"),
        ("app/__init__.py", ""),
        ("app/core.py", "def helper() -> None:\n    pass\n"),
    ];
    files.extend_from_slice(pages);
    project(&files)
}

#[test_case(
    &[("docs/api.rst", ":func:`app.core.helper`\n")],
    Change::Write("docs/api.rst", ":func:`app.core.other`\n"),
    (0, 1) ; "edited"
)]
#[test_case(&[("docs/api.rst", ":func:`app.core.helper`\n")], Change::Remove("docs/api.rst"), (0, 1) ; "removed")]
#[test_case(
    &[("docs/other.rst", "Nothing here.\n")],
    Change::Write("docs/api.rst", ":func:`app.core.helper`\n"),
    (1, 0) ; "added"
)]
fn documentation_is_read_again_once_a_page_changes(
    pages: &[(&'static str, &'static str)],
    change: Change,
    expected: (i32, i32),
) {
    let root = documented_project(pages);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    apply(root.path(), change);
    assert_eq!((first, check(root.path(), &["--select", "SMP409"]).0), expected);
}

// An ignore file that comes to hide the documentation changes what the walk reads, though no page changed.
#[test]
fn documentation_is_read_again_once_an_ignore_file_hides_it() {
    let root =
        documented_project(&[(".git/HEAD", "ref: refs/heads/main\n"), ("docs/api.rst", ":func:`app.core.helper`\n")]);
    let first = check(root.path(), &["--select", "SMP409"]).0;
    write_file(root.path(), ".gitignore", "docs/\n");
    assert_eq!((first, check(root.path(), &["--select", "SMP409"]).0), (0, 1));
}

const DUPLICATED: &str = r#"class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
"#;

// Which copy of the duplicated parameter the rule reports, read from its concise message.
fn copy_reported(root: &Path) -> &'static str {
    let stdout = check(root, &["--select", "SMP918"]).1;
    if stdout.contains("`__init__`") {
        "init"
    } else if stdout.contains("`Reader`") {
        "class"
    } else {
        "none"
    }
}

// A `conf.py` that appears, goes, or moves into or out of a Sphinx documentation directory changes the copy the rule
// reports, though no page changed, and the warm cache must follow it.
#[test_case(&[], Change::Write("docs/conf.py", "autoclass_content = \"init\"\n"), ("class", "init") ; "appears")]
#[test_case(
    &[("docs/conf.py", "autoclass_content = \"init\"\n")],
    Change::Remove("docs/conf.py"),
    ("init", "class") ;
    "removed"
)]
#[test_case(
    &[("docs/conf.py", "autoclass_content = \"init\"\n")],
    Change::Move("docs/conf.py", "tools/conf.py"),
    ("init", "class") ;
    "moved_out_of_the_documentation"
)]
#[test_case(
    &[("tools/conf.py", "autoclass_content = \"init\"\n")],
    Change::Move("tools/conf.py", "docs/conf.py"),
    ("class", "init") ;
    "moved_into_the_documentation"
)]
#[test_case(
    &[("docs/conf.py", "autoclass_content = \"class\"\n")],
    Change::Write("docs/conf.py", "autoclass_content = \"init\"\n"),
    ("class", "init") ;
    "edited_at_the_same_size"
)]
fn conf_py_changes_are_seen_by_a_warm_cache(
    files: &[(&'static str, &'static str)],
    change: Change,
    expected: (&str, &str),
) {
    let mut all = vec![
        ("pyproject.toml", "[tool.simplint]\ndocstring-style = \"rst\"\n"),
        ("model.py", DUPLICATED),
        ("docs/index.rst", "Index\n=====\n"),
    ];
    all.extend_from_slice(files);
    let root = project(&all);
    let first = copy_reported(root.path());
    apply(root.path(), change);
    assert_eq!((first, copy_reported(root.path())), expected);
}

// Keeping the documentation is best effort, so a cache it cannot write is reported and the run goes on.
#[test]
fn documentation_cache_that_cannot_be_written_is_reported() {
    let root = documented_project(&[("docs/api.rst", ":func:`app.core.helper`\n"), ("blocked", "")]);
    let (_, _, stderr) = check(root.path(), &["--select", "SMP409", "--cache-dir", "blocked"]);
    assert!(stderr.contains("Failed to write the documentation cache"), "{stderr}");
}

// The cache trusts the modification time and size: an edit that keeps both goes unseen, which shows the run reused the
// cached result instead of linting again.
#[test]
fn cached_clean_result_is_reused() {
    let root = project(&[("module.py", CLEAN_MODULE)]);
    check(root.path(), &["--select", "SMP101"]);
    rewrite_keeping_mtime(&root.path().join("module.py"), "x = 12345\n");
    let cached = check(root.path(), &["--select", "SMP101"]).0;
    assert_eq!((cached, check(root.path(), &["--select", "SMP101", "--no-cache"]).0), (0, 1));
}

#[test]
fn ignore_noqa_bypasses_the_cache() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP101\n")]);
    let first = check(root.path(), &["--select", "SMP101"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP101", "--ignore-noqa"]).0), (0, 1));
}

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
    assert_eq!((&first, &cached), (&fresh_run(root.path(), &args), &first));
}

fn fresh_run(root: &Path, args: &[&str]) -> (i32, String, String) {
    check(root, &[args, &["--no-cache"][..]].concat())
}

// An edit that keeps the modification time and size goes unseen, so the cached withdrawal holds back a fix the loop
// would apply.
#[test]
fn settled_file_reuses_the_withdrawn_fixes() {
    let root = project(&[RUFF_PLR2004, ("module.py", COMPARED)]);
    fix_unsafe(root.path(), "SMP202");
    rewrite_keeping_mtime(&root.path().join("module.py"), ADDED);
    let cached = fix_unsafe(root.path(), "SMP202").0;
    let unchanged = read(root.path(), "module.py");
    let fresh = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP202", "--no-cache"]).0;
    assert_eq!((cached, unchanged, fresh), (1, ADDED.to_owned(), 0));
}

const ADDED: &str = "_LIMIT = 3\n\n\ndef over(value: int) -> bool:\n    return value + _LIMIT\n";

#[test_case(&[("module.py", ADDED)], &["--fix"], &[], &["--fix", "--unsafe-fixes"] ; "unsafe_fixes")]
#[test_case(
    &[RUFF_PLR2004, ("module.py", COMPARED)],
    &["--fix", "--unsafe-fixes"],
    &[Change::Write("pyproject.toml", "")],
    &["--fix", "--unsafe-fixes"] ; "ruff_settings"
)]
#[test_case(
    &[("pyproject.toml", "[tool.simplint]\nunfixable = [\"SMP202\"]\n"), ("module.py", ADDED)],
    &["--fix", "--unsafe-fixes"],
    &[Change::Write("pyproject.toml", "")],
    &["--fix", "--unsafe-fixes"] ; "fixable"
)]
#[test_case(
    &[("module.py", ADDED)],
    &["--fix"],
    &[Change::Write("pyproject.toml", "[tool.simplint]\nextend-safe-fixes = [\"SMP202\"]\n")],
    &["--fix"] ; "fix_safety"
)]
fn settled_file_is_fixed_again(files: &[(&str, &str)], first: &[&str], changes: &[Change], second: &[&str]) {
    let root = project(files);
    check(root.path(), &[first, &["--select", "SMP202"]].concat());
    for change in changes {
        apply(root.path(), *change);
    }
    let code = check(root.path(), &[second, &["--select", "SMP202"]].concat()).0;
    let fixed = read(root.path(), "module.py");
    assert_eq!((code, fixed.contains("_LIMIT")), (0, false));
}

#[test]
fn settled_file_is_fixed_again_once_no_other_file_imports_the_name() {
    let root = project(&[("helpers.py", "limit = 3\n"), ("client.py", "from helpers import limit\n\nprint(limit)\n")]);
    let first = fix_unsafe(root.path(), "SMP102").0;
    write_file(root.path(), "client.py", "print(1)\n");
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
    write_file(root.path(), "pkg/__init__.py", "from ._core import run\n\n__all__ = [\"run\"]\n");
    let second = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!((first, second.lines().next().map(|line| line.contains("`run`"))), (PASSED.to_owned(), Some(true)));
}

#[test]
fn caches_of_other_builds_expire_after_thirty_days() {
    let root = project(&[
        ("module.py", CLEAN_MODULE),
        (".ruff_cache/simplint/0.0.1-00000000000000a1/1", "old"),
        (".ruff_cache/simplint/0.0.2-00000000000000b2/1", "new"),
    ]);
    set_mtime(&root.path().join(".ruff_cache/simplint/0.0.1-00000000000000a1/1"), thirty_one_days_ago());
    check(root.path(), &["--select", "SMP101"]);
    let simplint_dir = root.path().join(".ruff_cache/simplint");
    assert_eq!(
        (simplint_dir.join("0.0.1-00000000000000a1").exists(), simplint_dir.join("0.0.2-00000000000000b2").exists()),
        (false, true)
    );
}

fn thirty_one_days_ago() -> SystemTime {
    SystemTime::now() - Duration::from_hours(24 * 31)
}

// Another build's run creates its directory before it writes a cache there, so an empty directory goes by its own age.
#[test]
fn empty_caches_of_other_builds_expire_by_their_own_age() {
    let root = project(&[("module.py", CLEAN_MODULE)]);
    let simplint_dir = root.path().join(".ruff_cache/simplint");
    let (stale, fresh) = (simplint_dir.join("0.0.1-00000000000000a1"), simplint_dir.join("0.0.2-00000000000000b2"));
    fs::create_dir_all(&stale).expect("create");
    fs::create_dir_all(&fresh).expect("create");
    set_mtime(&stale, thirty_one_days_ago());
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!((stale.exists(), fresh.exists()), (false, true));
}

// A `cache-dir` from the repository's configuration stays inside the project, so a PR cannot make simplint write its
// cache, or clean another directory, outside the run; the default location serves instead.
#[test]
fn cache_dir_outside_the_project_falls_back_to_the_default() {
    let outside = empty_project();
    let root = project(&[
        ("pyproject.toml", &format!("[tool.ruff]\ncache-dir = {}\n", quoted(outside.path()))),
        ("module.py", CLEAN_MODULE),
    ]);
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!(
        (root.path().join(".ruff_cache/simplint").is_dir(), fs::read_dir(outside.path()).expect("read").count()),
        (true, 0)
    );
}

#[cfg(unix)]
#[test]
fn cache_dir_through_a_link_outside_the_project_falls_back_to_the_default() {
    let outside = empty_project();
    let root = project(&[("pyproject.toml", "[tool.ruff]\ncache-dir = \"link/cache\"\n"), ("module.py", CLEAN_MODULE)]);
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
    let outside = empty_project();
    let root = project(&[extra, &[("module.py", CLEAN_MODULE)]].concat());
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
    let outside = empty_project();
    let root = project(&[("module.py", CLEAN_MODULE)]);
    std::os::unix::fs::symlink(outside.path(), root.path().join(".ruff_cache")).expect("symlink");
    assert!(check(root.path(), &["--show-settings"]).1.contains("\ncache_dir = none\n"));
}

// The command line and `RUFF_CACHE_DIR` name the directory the user chose, so they may point anywhere.
#[test]
fn cache_dir_flag_may_point_outside_the_project() {
    let outside = empty_project();
    let root = project(&[("module.py", CLEAN_MODULE)]);
    check(root.path(), &["--select", "SMP101", "--cache-dir", outside.path().to_str().expect("utf-8 path")]);
    assert_eq!((outside.path().join("simplint").is_dir(), root.path().join(".ruff_cache").exists()), (true, false));
}

// A stale build directory is removed only when it has the shape of one and holds nothing but cache files; anything
// else in the `simplint` directory stays, however old.
#[test_case("notes", "notes.txt" ; "name_is_not_a_build")]
#[test_case("0.0.1-00000000000000a1", "notes.txt" ; "holds_a_file_other_than_a_cache")]
fn stale_directories_that_are_not_caches_of_other_builds_stay(name: &str, file: &str) {
    let path = format!(".ruff_cache/simplint/{name}/{file}");
    let root = project(&[("module.py", CLEAN_MODULE), (path.as_str(), "kept")]);
    set_mtime(&root.path().join(&path), thirty_one_days_ago());
    check(root.path(), &["--select", "SMP101"]);
    assert!(root.path().join(&path).exists());
}

// A link named like a build directory is not one, and what it points at is not the run's to remove.
#[cfg(unix)]
#[test]
fn link_in_the_cache_directory_is_kept_with_its_target() {
    let outside = empty_project();
    let target = outside.path().join("1");
    fs::write(&target, "kept").expect("write");
    set_mtime(&target, thirty_one_days_ago());
    let root = project(&[("module.py", CLEAN_MODULE)]);
    let simplint_dir = root.path().join(".ruff_cache/simplint");
    fs::create_dir_all(&simplint_dir).expect("create");
    let link = simplint_dir.join("0.0.1-00000000000000a1");
    std::os::unix::fs::symlink(outside.path(), &link).expect("symlink");
    check(root.path(), &["--select", "SMP101"]);
    assert_eq!((link.symlink_metadata().is_ok(), read(outside.path(), "1")), (true, "kept".to_owned()));
}

// Reports cached without fixes come back as a run without the cache reports them: with their fixes, help notes and
// suppression comments applied as they are in a fresh run.
#[test_case(&[] ; "concise")]
#[test_case(&["--output-format", "full"] ; "full")]
#[test_case(&["--output-format", "json"] ; "json")]
fn cached_findings_report_the_same_as_a_fresh_run(flags: &[&str]) {
    let root = project(&[("module.py", FINDINGS)]);
    let args = [&["--select", "SMP003,SMP101,SMP404,SMP503,SMP601"], flags].concat();
    let fresh = fresh_run(root.path(), &args);
    let (first, cached) = (check(root.path(), &args), check(root.path(), &args));
    assert_eq!((&first, &cached), (&fresh, &fresh));
}

// Findings with fixes, help notes, a suppression comment and one without a fix, as a run without fixes reports them.
const FINDINGS: &str = "import os\n\nVALUE = 1\n\n\ndef f(x):\n    y = 2\n    return x\n\n\ndef \
                        g(n):\n    for i in range(n):\n        pass\n    z = 3  # noqa: \
                        SMP101\n    return \"a\" + \"b\"\n";

#[test]
fn cached_findings_are_reported_again_under_other_settings() {
    let root = project(&[("module.py", FINDINGS)]);
    check(root.path(), &["--select", "SMP601"]);
    let cached = check(root.path(), &["--select", "SMP101"]);
    assert_eq!(cached, check(root.path(), &["--select", "SMP101", "--no-cache"]));
}

// A file with a syntax error or bytes that are not UTF-8 gets no rule report, yet its result is cached, so a warm run
// answers every file from the cache and does not index the project again.
#[test_case(b"def broken(:\n", &["--output-format", "concise"] ; "syntax_error_concise")]
#[test_case(b"def broken(:\n", &["--output-format", "full"] ; "syntax_error_full")]
#[test_case(b"def broken(:\n", &["--output-format", "json"] ; "syntax_error_json")]
#[test_case(b"name = \"\xff\"\n", &["--output-format", "concise"] ; "not_utf_8_concise")]
#[test_case(b"name = \"\xff\"\n", &["--output-format", "full"] ; "not_utf_8_full")]
#[test_case(b"name = \"\xff\"\n", &["--output-format", "json"] ; "not_utf_8_json")]
fn warm_run_reuses_the_reports_of_a_file_no_rule_reports(source: &[u8], flags: &[&str]) {
    let root = project(&[("module.py", CLEAN_MODULE)]);
    fs::write(root.path().join("broken.py"), source).expect("write");
    let run = |extra: &[&str]| simplint(root.path(), &[&["check", "--select", "SMP401"], flags, extra].concat());
    let fresh = run(&["--no-cache"]);
    let first = run(&["-v"]);
    let warm = run(&["-v"]);
    assert_eq!(
        (first.2.contains("Indexed "), warm.2.contains("Indexed "), first.1 == fresh.1, warm.1 == fresh.1),
        (true, false, true, true)
    );
}

// A file that does not parse is cached as having nothing to report, yet its warning shows on every run, cached or
// not.
#[test_case(&["--output-format", "concise"] ; "concise")]
#[test_case(&["--output-format", "full"] ; "full")]
#[test_case(&["--output-format", "json"] ; "json")]
fn warm_run_warns_again_about_a_file_that_does_not_parse(flags: &[&str]) {
    let root = project(&[("module.py", "x = 1\n"), ("broken.py", "def broken(:\n")]);
    let args = [&["check", "--select", "SMP101"], flags].concat();
    let fresh = simplint(root.path(), &[&args, &["--no-cache"][..]].concat());
    let (first, warm) = (simplint(root.path(), &args), simplint(root.path(), &args));
    assert_eq!((&first, &warm, warm.2.as_str()), (&fresh, &fresh, UNPARSED));
}

#[test]
fn file_that_parses_again_is_checked_again() {
    let root = project(&[("module.py", "x = 1\n"), ("broken.py", "def broken(:\n")]);
    let args = ["--select", "SMP101"];
    check(root.path(), &args);
    write_file(root.path(), "broken.py", "y = 2\n");
    assert_eq!(check(root.path(), &args), fresh_run(root.path(), &args));
}

// A read error is reported only while `io-error` is selected: without it the run logs a warning, which a cached clean
// result would not log again.
#[test_case("F", (false, true) ; "logged_without_io_error")]
#[test_case("E902", (true, false) ; "reported_with_io_error")]
fn unreadable_file_is_reported_again_on_a_warm_run(select: &str, expected: (bool, bool)) {
    let root = project(&[("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [\"{select}\"]\n"))]);
    fs::write(root.path().join("latin.py"), b"name = \"\xff\"\n").expect("write");
    check(root.path(), &[]);
    let (_, stdout, stderr) = check(root.path(), &[]);
    assert_eq!((stdout.contains("E902"), stderr.contains("Failed to lint latin.py")), expected);
}

// The index a warm run reuses its reports from is built from the run's files and settings. Once any of those change,
// the next warm run must answer as a run without the cache does, so each change to an index input is checked against
// a fresh run.
//
// `helpers.py` is reported while nothing imports it, so each change to the importer flips its reports, and a warm run
// that reused them from the first run would miss the flip.
#[test_case(
    &[("helpers.py", HELPER), ("client.py", CLEAN_MODULE)],
    Change::Write("client.py", CLIENT),
    "SMP401",
    &["."] ; "importer_edited"
)]
#[test_case(&[("helpers.py", HELPER)], Change::Write("client.py", CLIENT), "SMP401", &["."] ; "importer_added")]
#[test_case(
    &[("helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Remove("client.py"),
    "SMP401",
    &["."] ; "importer_removed"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\nnamespace-packages = [\"pkg\"]\n"),
        ("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"),
        ("pkg/b.py", "VALUE = 1\n"),
    ],
    Change::Write("pkg/__init__.py", ""),
    "SMP302",
    &["."] ; "namespace_package_gains_an_init_module"
)]
#[test_case(
    &[("lib/helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Write("pyproject.toml", "[tool.ruff]\nsrc = [\"lib\"]\n"),
    "SMP401",
    &["."] ; "source_root_changed"
)]
#[test_case(
    &[("helpers.py", HELPER), ("client.py", CLIENT)],
    Change::Write("pyproject.toml", "[tool.ruff]\nsrc = [\"missing\"]\n"),
    "SMP401",
    &["."] ; "source_root_removed"
)]
#[test_case(
    &[("client.py", CLIENT), ("helpers.py", HELPER)],
    Change::Remove("client.py"),
    "SMP401",
    &["client.py"] ; "linted_file_removed"
)]
// A file outside the linted paths that the index resolves an import to: adding it changes the index though the run's
// files stay the same.
#[test_case(
    &[("src/client.py", CLIENT), ("src/helpers.py", "")],
    Change::Write("src/helpers.py", HELPER),
    "SMP401",
    &["src/client.py"] ; "unlinted_module_appears"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\nsrc = [\"lib\", \".\"]\nextend-exclude = [\"lib\"]\n"),
        ("lib/notes.txt", ""),
        ("helpers.py", HELPER),
        ("client.py", CLIENT),
    ],
    Change::Write("lib/helpers.py", HELPER),
    "SMP401",
    &["."] ; "excluded_module_shadows_an_import"
)]
#[test_case(
    &[
        ("pyproject.toml", "[project]\nname = \"proj\"\n\n[tool.ruff]\nextend-exclude = [\"scripts-dir/helpers\"]\n"),
        ("scripts-dir/run.py", "from helpers.util import thing\n\nprint(thing)\n"),
        ("other/helpers.py", "def thing() -> int:\n    return 1\n"),
        ("scripts-dir/helpers/util.py", "thing = 1\n"),
    ],
    Change::Remove("scripts-dir/helpers/util.py"),
    "SMP409",
    &["."] ; "module_below_an_unimportable_directory_removed"
)]
fn warm_run_after_index_input_changes_matches_a_fresh_run(
    files: &[(&str, &str)],
    change: Change,
    select: &str,
    targets: &[&str],
) {
    let root = project(files);
    let args = [&["--select", select], targets].concat();
    check(root.path(), &args);
    apply(root.path(), change);
    assert_eq!(check(root.path(), &args), fresh_run(root.path(), &args));
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
    assert_eq!(check(&project_root, &args), fresh_run(&project_root, &args));
}

const HELPER: &str = "def helper() -> None:\n    pass\n";
const CLIENT: &str = "from helpers import helper\n";

#[test]
fn warm_run_skips_the_index_again_after_an_edit() {
    let root = project(&[("helpers.py", HELPER), ("client.py", CLIENT), ("other.py", CLEAN_MODULE)]);
    let indexed = |root: &Path| check(root, &["--select", "SMP401", "-v"]).2.contains("Indexed ");
    let before = [indexed(root.path()), indexed(root.path())];
    write_file(root.path(), "other.py", "VALUE = 2\n");
    let after = [indexed(root.path()), indexed(root.path())];
    assert_eq!((before, after), ([true, false], [true, false]));
}

#[test]
fn warm_run_after_documentation_changes_matches_a_fresh_run() {
    let root = documented_project(&[("docs/api.rst", ":func:`app.core.helper`\n")]);
    let args = ["--select", "SMP409"];
    check(root.path(), &args);
    write_file(root.path(), "docs/api.rst", ":func:`app.core.other`\n");
    assert_eq!(check(root.path(), &args), fresh_run(root.path(), &args));
}

// A safe fix and an unsafe one, so the `full` text shows the fix marker and the diff of each.
const FIXABLE: &str = "# --- helpers ---\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";

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
        set_mtime(&file, old);
    }
    full_report(root.path(), &[], &[]);
    let all_unchanged = cache_files(root.path())
        .iter()
        .all(|file| fs::metadata(file).expect("metadata").modified().expect("modified time") == old);
    assert_eq!((with_text > reports_only, all_unchanged), (true, true));
}

fn cache_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(cache_dir(root)).expect("read").map(|entry| entry.expect("entry").path()).collect()
}

fn full_report(root: &Path, flags: &[&str], env: &[(&str, &str)]) -> String {
    let args = [&["check", "--output-format", "full", "--select", "SMP001,SMP901", "--exit-zero"], flags].concat();
    simplint_with(root, &args, "", env).1
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
        let args = [&["--output-format", "full", "--select", "SMP001,SMP901", "--exit-zero"], args].concat();
        check_in(root.path(), dir, &args).1
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
    write_file(root.path(), "other.py", CLEAN_MODULE);
    let warm = full_report(root.path(), &[], &[]);
    assert_eq!(warm, full_report(root.path(), &["--no-cache"], &[]));
}

// A fix the read-only file cannot take leaves the file as it is, so the file keeps its reports, and a later run reports
// them instead of reusing a clean result.
#[test]
fn unwritten_fix_is_not_cached_as_clean() {
    let source = "__all__ = (\"f\",)\n\n\ndef f() -> None:\n    \"\"\"Do.\"\"\"\n";
    let root = project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F\"]\n"), ("mod.py", source)]);
    make_read_only(&root.path().join("mod.py"));
    let (code, stdout, stderr) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP405"]);
    let (warm, warm_stdout, _) = check(root.path(), &["--select", "SMP405"]);
    assert_eq!(
        (
            code,
            stdout.contains("SMP405"),
            stderr.contains("Failed to lint mod.py: writing"),
            warm,
            warm_stdout.contains("SMP405")
        ),
        (1, true, true, 1, true)
    );
}

// A `--diff` run keeps its fixes in memory. A file linted after a fix in another file sees that fix, so its reports are
// not the disk's and a later run must not reuse them: `writer.py` repeats `reader.py` only until `reader.py`'s fix
// changes its statements.
#[test]
fn diff_run_does_not_cache_reports_computed_from_fixes_it_kept_in_memory() {
    let body = r#"def parse_lines(text):
    lines = text.splitlines()
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
"#;
    let tail =
        "    count = len(values)\n    if count == 0:\n        raise ValueError(\"no values\")\n    return values\n";
    let root = project(&[
        ("reader.py", &[body, tail].concat()),
        ("writer.py", &[body, tail].concat().replace("len(values)\n", "len(values)  # noqa: SMP502\n")),
    ]);
    let args = ["--select", "SMP502,SMP530"];
    check(root.path(), &args);
    check(root.path(), &[&args[..], &["--diff"]].concat());
    let warm = check(root.path(), &args);
    let fresh = fresh_run(root.path(), &args);
    assert_eq!((warm.1.matches("SMP530").count(), warm.1.contains("SMP502"), warm), (2, true, fresh));
}

// Ruff's `src` may leave out the directory the mirrored source sits in, yet the paired-test lookup reads it, so a
// source deleted from there is checked again even though no run names it.
#[test]
fn private_import_is_checked_again_once_its_source_outside_the_search_roots_is_deleted() {
    checked_again_once_its_source_is_deleted(|root, _| {
        write_file(root, "src/pkg/_api.py", CLEAN_MODULE);
        root.join("src/pkg/_api.py")
    });
}

// Deletes the file `setup` writes, which the run's cache must not hide: the mirrored source the test reads is gone.
fn checked_again_once_its_source_is_deleted(setup: impl FnOnce(&Path, &TempDir) -> PathBuf) {
    let outside = empty_project();
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nsrc = [\"lib\"]\n"),
        ("lib/.keep", ""),
        ("src/pkg/__init__.py", ""),
        (
            "tests/pkg/test_api.py",
            "from pkg._api import VALUE\n\n\ndef test_value(value: VALUE) -> None:\n    assert value == 1\n",
        ),
    ]);
    let deleted = setup(root.path(), &outside);
    let select = ["check", "--select", "SMP310", "tests"];
    let cold = simplint(root.path(), &select).0;
    let kept = simplint(root.path(), &[&select[..], &["-v"]].concat());
    fs::remove_file(deleted).expect("remove");
    let warm = simplint(root.path(), &select);
    let fresh = simplint(root.path(), &[&select[..], &["--no-cache"]].concat());
    assert_eq!((cold, kept.0, kept.2.contains("Indexed "), warm.0, warm == fresh), (0, 0, false, 1, true));
}

// A link keys the cache by whether its target still exists, so the linked source is deleted through its target.
#[cfg(unix)]
#[test]
fn private_import_is_checked_again_once_its_linked_source_is_deleted() {
    checked_again_once_its_source_is_deleted(|root, outside| {
        let target = outside.path().join("_api.py");
        fs::write(&target, CLEAN_MODULE).expect("write");
        std::os::unix::fs::symlink(&target, root.join("src/pkg/_api.py")).expect("symlink");
        target
    });
}

// Reports that name a path relative to the working directory are cached with the directory they were made in, so a run
// from another directory names its paths as a run without the cache does.
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\n\n[tool.simplint]\n"),
        ("app/__init__.py", ""),
        ("app/a.py", "from app import b\n\nVALUE = 1\n"),
        ("app/b.py", "from app import a\n\nOTHER = 2\n"),
    ],
    "SMP308" ; "import_cycle"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\n\n[tool.simplint]\n"),
        ("app/__init__.py", ""),
        ("app/a.py", REPEATED),
        ("app/b.py", REPEATED),
    ],
    "SMP530" ; "duplicate_code"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\n\n[tool.simplint]\nproject-kind = \"library\"\n"),
        ("app/__init__.py", "from app._core import fetch\n\n__all__ = [\"fetch\"]\n"),
        ("app/_core.py", "def fetch() -> None:\n    print()\n"),
    ],
    "SMP903" ; "undocumented_public_api"
)]
fn cached_report_names_the_paths_of_the_directory_it_runs_in(files: &[(&str, &str)], select: &str) {
    let root = project(files);
    let run = |dir: &Path, extra: &[&str]| {
        check_in(root.path(), dir, &[&["--output-format", "concise", "--select", select], extra].concat()).1
    };
    let app = root.path().join("app");
    run(root.path(), &[]);
    let warm = run(&app, &[]);
    assert_eq!(warm, run(&app, &["--no-cache"]));
}

// A private import in a test is judged by the source file the test mirrors, which is on disk, so a clean result of an
// earlier run does not stand once that file is gone.
#[test]
fn private_import_is_checked_again_once_its_mirrored_module_is_deleted() {
    let root = project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/_helper.py", CLEAN_MODULE),
        (
            "tests/pkg/test_helper.py",
            "from pkg._helper import VALUE\n\n\ndef test_value() -> None:\n    assert VALUE == 1\n",
        ),
    ]);
    let select = ["check", "--select", "SMP310", "src", "tests"];
    let cold = simplint(root.path(), &select).0;
    fs::remove_file(root.path().join("src/pkg/_helper.py")).expect("remove");
    let warm = simplint(root.path(), &select);
    let fresh = simplint(root.path(), &[&select[..], &["--no-cache"]].concat());
    assert_eq!((cold, warm), (0, fresh));
}

// A sibling added to a directory changes the count the other private modules of its prefix share, so a cached clean
// result for them is checked again.
#[test]
fn flattened_package_is_checked_again_once_a_sibling_is_added() {
    let root = project(&[("pkg/_html_a.py", CLEAN_MODULE), ("pkg/_html_b.py", CLEAN_MODULE)]);
    let select = ["check", "--select", "SMP311", "pkg"];
    let cold = simplint(root.path(), &select).0;
    write_file(root.path(), "pkg/_html_c.py", CLEAN_MODULE);
    let warm = simplint(root.path(), &select);
    let fresh = simplint(root.path(), &[&select[..], &["--no-cache"]].concat());
    assert_eq!((cold, warm.0, warm.1.matches("SMP311").count(), warm == fresh), (0, 1, 3, true));
}

// An `__init__.py` written into a directory that already exists changes that directory alone, not the run's files, so
// the cached reports are checked again only through the layout the disk check reads.
#[test]
fn flattened_package_is_checked_again_once_a_package_appears() {
    let root = project(&[
        ("pkg/_h_a.py", CLEAN_MODULE),
        ("pkg/_h_b.py", CLEAN_MODULE),
        ("pkg/_h_c.py", CLEAN_MODULE),
        ("pkg/_h/notes.txt", "notes\n"),
    ]);
    let select = ["check", "--select", "SMP311", "pkg"];
    let cold = simplint(root.path(), &select);
    write_file(root.path(), "pkg/_h/__init__.py", "");
    let warm = simplint(root.path(), &select);
    let fresh = simplint(root.path(), &[&select[..], &["--no-cache"]].concat());
    assert_eq!((cold.0, warm.0, warm == fresh), (1, 0, true));
}

// A submodule that appears beside a private import changes what the hint names, so the cached report is checked again.
#[test]
fn private_import_is_checked_again_once_its_submodule_appears() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/_a/__init__.py", ""),
        ("pkg/b.py", "from ._a import _x\n\n_x.what()\n"),
    ]);
    let select = ["check", "--select", "SMP310", "pkg"];
    let cold = simplint(root.path(), &select);
    write_file(root.path(), "pkg/_a/_x.py", "def what() -> None: ...\n");
    let warm = simplint(root.path(), &select);
    let fresh = simplint(root.path(), &[&select[..], &["--no-cache"]].concat());
    assert_eq!(
        (cold.0, warm.0, warm.1.contains("re-export the names you use in `_a/__init__.py`"), warm == fresh),
        (1, 1, true, true)
    );
}
