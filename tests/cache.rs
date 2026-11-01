mod common;

use std::fs;
use std::time::Duration;

use test_case::test_case;

use common::{check, project, read, simplint_with};

#[test]
fn cache_is_kept_in_ruffs_cache_directory() {
    let root = project(&[("pyproject.toml", "[project]\nname = \"demo\"\n"), ("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP010"]);
    let cache = root.path().join(".ruff_cache");
    assert_eq!(
        (
            fs::read_to_string(cache.join("CACHEDIR.TAG")).expect("read"),
            fs::read_to_string(cache.join(".gitignore")).expect("read"),
            fs::read_dir(cache.join("simplint").join(env!("CARGO_PKG_VERSION"))).expect("read").count(),
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
    check(root.path(), &["--select", "SMP010"]);
    let contents: Vec<String> = ruff_files.iter().map(|(name, _)| read(root.path(), name)).collect();
    assert_eq!(contents, ruff_files.map(|(_, content)| content.to_owned()));
}

#[test]
fn no_cache_writes_nothing() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP010", "--no-cache"]);
    assert!(!root.path().join(".ruff_cache").exists());
}

#[test_case(&["--cache-dir", "elsewhere"], &[], &[] ; "flag")]
#[test_case(&[], &[], &[("RUFF_CACHE_DIR", "elsewhere")] ; "environment")]
#[test_case(&[], &[("pyproject.toml", "[tool.ruff]\ncache-dir = \"elsewhere\"\n")], &[] ; "pyproject")]
fn cache_dir_moves_the_cache(flags: &[&str], extra: &[(&str, &str)], env: &[(&str, &str)]) {
    let mut files = vec![("module.py", "VALUE = 1\n")];
    files.extend_from_slice(extra);
    let root = project(&files);
    let mut args = vec!["check", "--select", "SMP010"];
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
    check(root.path(), &["--select", "SMP010"]);
    let cache = fs::read_dir(root.path().join(".ruff_cache/simplint").join(env!("CARGO_PKG_VERSION")))
        .expect("read")
        .next()
        .expect("a cache file")
        .expect("entry")
        .path();
    let old = std::time::SystemTime::UNIX_EPOCH + Duration::from_hours(24);
    fs::File::options().write(true).open(&cache).expect("open").set_modified(old).expect("set modified time");
    check(root.path(), &["--select", "SMP010"]);
    assert_eq!(fs::metadata(&cache).expect("metadata").modified().expect("modified time"), old);
}

#[test]
fn unreadable_cache_is_replaced() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    check(root.path(), &["--select", "SMP010"]);
    let directory = root.path().join(".ruff_cache/simplint").join(env!("CARGO_PKG_VERSION"));
    for entry in fs::read_dir(&directory).expect("read") {
        fs::write(entry.expect("entry").path(), "not a cache").expect("write");
    }
    let (code, _, stderr) = check(root.path(), &["--select", "SMP010"]);
    let (_, _, again) = check(root.path(), &["--select", "SMP010"]);
    assert_eq!((code, stderr.contains("Failed to read cache file"), again), (0, true, String::new()));
}

#[test]
fn cache_directory_that_cannot_be_created_fails_the_run() {
    let root = project(&[("module.py", "VALUE = 1\n"), ("blocked", "")]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP010", "--cache-dir", "blocked"]);
    assert_eq!(
        (code, stderr.contains("Failed to initialize cache"), stderr.contains("Failed to create temporary file")),
        (2, true, true)
    );
}

// A file this run did not check keeps its cached result, as Ruff keeps files seen within the last 30 days.
#[test]
fn cache_keeps_files_other_runs_checked() {
    let root = project(&[("module.py", "VALUE = 1\n"), ("other.py", "OTHER = 1\n")]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    check(root.path(), &["--select", "SMP010", "module.py"]);
    check(root.path(), &["--select", "SMP010", "other.py"]);
    fs::write(&path, "x = 1\n").expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    assert_eq!(check(root.path(), &["--select", "SMP010", "module.py"]).0, 0);
}

#[test]
fn cached_clean_file_is_checked_again_when_another_file_starts_importing_it() {
    let root = project(&[("helpers.py", "def helper() -> None:\n    pass\n"), ("client.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP113"]).0;
    fs::write(root.path().join("client.py"), "from helpers import helper\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP113"]).0), (0, 1));
}

#[test]
fn cached_clean_file_is_checked_again_under_other_settings() {
    let root = project(&[("module.py", "x = 1\n")]);
    let first = check(root.path(), &["--select", "SMP001"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP010"]).0), (0, 1));
}

#[test]
fn cached_clean_file_is_checked_again_once_its_directory_becomes_a_package() {
    let root = project(&[("pkg/a.py", "from pkg.b import VALUE\n\nprint(VALUE)\n"), ("pkg/b.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP110"]).0;
    fs::write(root.path().join("pkg/__init__.py"), "").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP110"]).0), (0, 1));
}

#[test]
fn cached_clean_file_is_checked_again_once_edited() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let first = check(root.path(), &["--select", "SMP010"]).0;
    fs::write(root.path().join("module.py"), "x = 1\n").expect("write");
    assert_eq!((first, check(root.path(), &["--select", "SMP010"]).0), (0, 1));
}

// Like Ruff's, the cache trusts the modification time: an edit that keeps it goes unseen, which shows the run reused the
// cached result instead of linting again.
#[test]
fn cached_clean_result_is_reused() {
    let root = project(&[("module.py", "VALUE = 1\n")]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    check(root.path(), &["--select", "SMP010"]);
    fs::write(&path, "x = 1\n").expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    let cached = check(root.path(), &["--select", "SMP010"]).0;
    assert_eq!((cached, check(root.path(), &["--select", "SMP010", "--no-cache"]).0), (0, 1));
}

#[test]
fn ignore_noqa_bypasses_the_cache() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP010\n")]);
    let first = check(root.path(), &["--select", "SMP010"]).0;
    assert_eq!((first, check(root.path(), &["--select", "SMP010", "--ignore-noqa"]).0), (0, 1));
}

#[test]
fn cache_tracks_skip_magic_trailing_comma() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.format]\nskip-magic-trailing-comma = true\n"),
        ("module.py", "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n"),
    ]);
    let quiet = check(root.path(), &["--select", "SMP115"]).0;
    fs::write(root.path().join("pyproject.toml"), "").expect("write");
    assert_eq!((quiet, check(root.path(), &["--select", "SMP115"]).0), (0, 1));
}

// Ruff's D103 would report the public function once its docstring goes, so the fix loop withdraws SMP002's fix.
const RUFF_D103: (&str, &str) = ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"D103\"]\n");
const PUBLIC: &str = "def run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";
const PRIVATE: &str = "def _run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n";

#[test_case(&["--fix", "--unsafe-fixes"] ; "fix")]
#[test_case(&["--fix"] ; "unsafe_fix_hint")]
#[test_case(&["--diff", "--unsafe-fixes"] ; "diff")]
#[test_case(&["--fix-only", "--unsafe-fixes"] ; "fix_only")]
#[test_case(&["--fix", "--unsafe-fixes", "--exit-non-zero-on-fix"] ; "exit_non_zero_on_fix")]
#[test_case(&["--fix", "--unsafe-fixes", "--statistics"] ; "statistics")]
fn settled_file_reports_the_same_from_the_cache(flags: &[&str]) {
    let root = project(&[RUFF_D103, ("module.py", &format!("{PUBLIC}\n\nx = 1\n"))]);
    let args = [&["--select", "SMP002,SMP010"], flags].concat();
    let (first, cached) = (check(root.path(), &args), check(root.path(), &args));
    assert_eq!((&first, &cached), (&check(root.path(), &[&args[..], &["--no-cache"]].concat()), &first));
}

// An edit that keeps the modification time goes unseen, so the cached withdrawal holds back a fix the loop would apply.
#[test]
fn settled_file_reuses_the_withdrawn_fixes() {
    let root = project(&[RUFF_D103, ("module.py", PUBLIC)]);
    let path = root.path().join("module.py");
    let modified = fs::metadata(&path).expect("metadata").modified().expect("modified time");
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP002"]);
    fs::write(&path, PRIVATE).expect("write");
    fs::File::options().write(true).open(&path).expect("open").set_modified(modified).expect("set modified time");
    let cached = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP002"]).0;
    let unchanged = read(root.path(), "module.py");
    let fresh = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP002", "--no-cache"]).0;
    assert_eq!((cached, unchanged, fresh), (1, PRIVATE.to_owned(), 0));
}

#[test_case(&[("module.py", PRIVATE)], &["--fix"], None, &["--fix", "--unsafe-fixes"] ; "unsafe_fixes")]
#[test_case(&[RUFF_D103, ("module.py", PUBLIC)], &["--fix", "--unsafe-fixes"], Some(""), &["--fix", "--unsafe-fixes"] ; "ruff_settings")]
#[test_case(&[("pyproject.toml", "[tool.simplint]\nunfixable = [\"SMP002\"]\n"), ("module.py", PRIVATE)], &["--fix", "--unsafe-fixes"], Some(""), &["--fix", "--unsafe-fixes"] ; "fixable")]
#[test_case(&[("module.py", PRIVATE)], &["--fix"], Some("[tool.simplint]\nextend-safe-fixes = [\"SMP002\"]\n"), &["--fix"] ; "fix_safety")]
fn settled_file_is_fixed_again(files: &[(&str, &str)], first: &[&str], pyproject: Option<&str>, second: &[&str]) {
    let root = project(files);
    check(root.path(), &[first, &["--select", "SMP002"]].concat());
    if let Some(pyproject) = pyproject {
        fs::write(root.path().join("pyproject.toml"), pyproject).expect("write");
    }
    let code = check(root.path(), &[second, &["--select", "SMP002"]].concat()).0;
    let fixed = read(root.path(), "module.py");
    assert_eq!((code, fixed.contains("Run.")), (0, false));
}

#[test]
fn settled_file_is_fixed_again_once_no_other_file_imports_the_name() {
    let root = project(&[("helpers.py", "limit = 3\n"), ("client.py", "from helpers import limit\n\nprint(limit)\n")]);
    let first = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]).0;
    fs::write(root.path().join("client.py"), "print(1)\n").expect("write");
    check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    assert_eq!((first, read(root.path(), "helpers.py")), (1, "_LIMIT = 3\n".to_owned()));
}
