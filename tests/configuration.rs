mod common;

use std::fs;
use std::path::Path;

use test_case::test_case;

use common::{SECTION, SINGLE_X, check, project, read, simplint};

#[test]
fn pyproject_configures_rules_and_target() {
    let root = project(&[
        (
            "pyproject.toml",
            r#"
[project]
requires-python = ">=3.14"
dependencies = ["Requests>=2"]
scripts = { tool = "cli:main" }

[tool.ruff]
line-length = 100
extend-exclude = ["generated"]

[tool.simplint]
select = ["SMP0", "SMP108", "SMP113"]
extend-select = ["SMP040"]
ignore = ["SMP001"]
per-file-ignores = { "legacy.py" = ["SMP010"] }
"#,
        ),
        ("cli.py", "def main() -> None:\n    pass\n"),
        (
            "module.py",
            "# --- helpers ---\nfrom __future__ import annotations\ntry:\n    import requests\nexcept ImportError:\n    requests = None\n",
        ),
        ("legacy.py", "x = 1\n"),
        ("generated/output.py", "x = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &[]).1,
        "cli.py:1:5: SMP113 `main` is imported by other modules; list it in `__all__`\n\
         module.py:2:24: SMP108 [*] Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`\n\
         module.py:3:1: SMP040 Import guard around a required dependency; import it directly and let a broken install fail\n\
         Found 3 errors.\n[*] 1 fixable with `--fix` (2 unsafe fixes available with `--fix --unsafe-fixes`).\n"
    );
}

#[test]
fn pyproject_run_options_apply() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nselect = [\"SMP001\"]\nfix = true\nshow-fixes = true\noutput-format = \"concise\"\n",
        ),
        ("module.py", "# --- helpers ---\nVALUE = 1\n"),
    ]);
    assert_eq!(
        simplint(root.path(), &["check"]).1,
        "\nFixed 1 error:\n- module.py:\n    1 × SMP001 (section-comment)\n\nFound 1 error (1 fixed, 0 remaining).\n"
    );
}

#[test]
fn command_line_flags_override_pyproject_run_options() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nselect = [\"SMP001\"]\nfix = true\nunsafe-fixes = true\nfix-only = true\n",
        ),
        ("module.py", "# --- helpers ---\nVALUE = 1\n"),
    ]);
    let result = check(root.path(), &["--no-fix-only", "--no-fix", "--no-unsafe-fixes", "--no-show-fixes"]);
    assert_eq!((result.0, read(root.path(), "module.py")), (1, "# --- helpers ---\nVALUE = 1\n".to_owned()));
}

#[test]
fn pyproject_unknown_simplint_key_fails() {
    let root = project(&[("pyproject.toml", "[tool.simplint]\nnot-an-option = 1\n"), ("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("unknown field `not-an-option`")), (2, true));
}

#[test_case("[tool.simplint]\nselect = 1\n" ; "wrong_type")]
#[test_case("[project\n" ; "not_toml")]
fn pyproject_invalid_fails(content: &str) {
    let root = project(&[("pyproject.toml", content), ("module.py", "VALUE = 1\n")]);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("Failed to parse")), (2, true));
}

#[test]
fn pyproject_extend_per_file_ignores_adds_to_per_file_ignores() {
    let root = project(&[
        (
            "pyproject.toml",
            "[tool.simplint]\nper-file-ignores = { \"other.py\" = [\"SMP001\"] }\nextend-per-file-ignores = { \"module.py\" = [\"SMP010\"] }\n",
        ),
        ("module.py", "x = 1\n"),
        ("other.py", "# --- helpers ---\nx = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP001,SMP010"]),
        (1, format!("other.py:2:1: {SINGLE_X}\nFound 1 error.\n"), String::new())
    );
}

#[test]
fn dependency_guard_maps_import_names_and_skips_markers() {
    let root = project(&[
        ("pyproject.toml", "[project]\ndependencies = [\"PyYAML>=6\", \"tomli>=2; python_version < '3.11'\"]\n"),
        (
            "module.py",
            "try:\n    import yaml\nexcept ImportError:\n    yaml = None\ntry:\n    import tomli\nexcept ImportError:\n    tomli = None\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP040"]).1,
        "module.py:1:1: SMP040 Import guard around a required dependency; import it directly and let a broken install \
         fail\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    );
}

#[test]
fn nested_pyproject_configures_its_own_files() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP010\"]\n"),
        ("sub/pyproject.toml", "[tool.simplint]\nselect = [\"SMP001\"]\n"),
        ("module.py", "x = 1\n"),
        ("sub/module.py", "x = 1\n"),
        ("sub/other.py", "y = 1\n"),
    ]);
    assert_eq!(check(root.path(), &[]).1, format!("module.py:1:1: {SINGLE_X}\nFound 1 error.\n"));
}

#[test]
fn nested_pyproject_without_simplint_table_keeps_the_parent_settings() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP0\"]\nignore = [\"SMP001\"]\n"),
        ("sub/pyproject.toml", "[project]\nname = \"sub\"\n"),
        ("sub/module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    let stdout = format!("{}:2:1: {SINGLE_X}\nFound 1 error.\n", common::native("sub/module.py"));
    assert_eq!(check(root.path(), &[]).1, stdout);
}

#[test]
fn pyproject_without_simplint_table_still_gives_project_metadata() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"root\"\n"),
        ("sub/pyproject.toml", "[project]\nname = \"sub\"\ndependencies = [\"PyYAML\"]\n"),
        ("sub/module.py", "try:\n    import yaml\nexcept ImportError:\n    yaml = None\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP040"]).0, 1);
}

#[cfg(unix)]
#[test]
fn unreadable_pyproject_fails() {
    let root = project(&[("pyproject.toml", "[project]\n"), ("module.py", "VALUE = 1\n")]);
    common::set_mode(&root.path().join("pyproject.toml"), 0o000);
    let (code, _, stderr) = check(root.path(), &[]);
    assert_eq!((code, stderr.contains("Failed to read")), (2, true));
}

#[test_case(&[] ; "discovered")]
#[test_case(&["--config", "sub/pyproject.toml"] ; "config_file")]
fn unknown_selector_names_its_pyproject(flags: &[&str]) {
    let root = project(&[
        ("sub/pyproject.toml", "[tool.simplint]\nselect = [\"SMP999\"]\n"),
        ("sub/module.py", "VALUE = 1\n"),
    ]);
    let pyproject = common::canonical(root.path()).join("sub").join("pyproject.toml");
    assert_eq!(
        check(root.path(), &[flags, &["sub"]].concat()),
        (
            2,
            String::new(),
            format!(
                "simplint failed\n  Cause: Unknown rule selector `SMP999` in `select` from `{}`\n",
                pyproject.display()
            )
        )
    );
}

#[test]
fn isolated_ignores_configuration() {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nignore = [\"SMP010\"]\n[tool.ruff]\nexclude = [\"module.py\"]\n"),
        ("module.py", "x = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--isolated", "--select", "SMP010"]).0, 1);
}

#[test]
fn isolated_rejects_a_config_file() {
    let root = project(&[("ruff.toml", "line-length = 100\n"), ("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--isolated", "--config", "ruff.toml"]);
    assert_eq!(
        (code, stderr.contains("The argument `--config=ruff.toml` cannot be used with `--isolated`")),
        (2, true)
    );
}

// The configuration replaces the project's, which selects only SMP001; a `ruff.toml` holds no simplint options, so the
// defaults apply.
#[test_case(
    "pyproject.toml",
    "[tool.simplint]\nselect = [\"SMP010\"]\n",
    "module.py:1:1: SMP010 Single-letter name `x`; only loop targets and comprehensions may use one\nFound 1 error.\n" ;
    "pyproject"
)]
#[test_case(
    "ruff.toml",
    "target-version = \"py310\"\n",
    "module.py:1:1: SMP010 Single-letter name `x`; only loop targets and comprehensions may use one\nmodule.py:1:1: \
     SMP101 `x` is never reassigned; annotate it `Final`\nmodule.py:1:1: SMP102 Module-scope variable `x` should be \
     named `_X`\nFound 3 errors.\n2 unsafe fixes available with `--fix --unsafe-fixes`.\n" ;
    "ruff_toml"
)]
fn config_file_is_used_for_every_file(name: &str, content: &str, stdout: &str) {
    let path = format!("config/{name}");
    let root = project(&[
        (&path, content),
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP001\"]\n"),
        ("module.py", "x = 1\n"),
    ]);
    assert_eq!(check(root.path(), &["--config", &path]), (1, stdout.to_owned(), String::new()));
}

#[test]
fn relative_config_anchors_per_file_ignores_at_its_directory() {
    let root = project(&[
        ("sub/pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"pkg/*\" = [\"SMP010\"] }\n"),
        ("sub/pkg/pyproject.toml", "[tool.simplint]\n"),
        ("sub/pkg/module.py", "x = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--config", "sub/pyproject.toml", "--select", "SMP010"]),
        (0, "All checks passed!\n".to_owned(), String::new())
    );
}

#[test_case(&["--config", "select = ['SMP001']"] ; "after_the_subcommand")]
#[test_case(&["--config", "select = ['SMP001']", "--config", "select = ['SMP010']"] ; "first_override_wins")]
fn inline_config_overrides_simplint_options(flags: &[&str]) {
    let root = project(&[
        ("pyproject.toml", "[tool.simplint]\nselect = [\"SMP010\"]\n"),
        ("module.py", "# --- helpers ---\nx = 1\n"),
    ]);
    let mut args = vec!["check", "--output-format", "concise"];
    args.extend_from_slice(flags);
    assert_eq!(
        simplint(root.path(), &args).1,
        format!("module.py:1:1: {SECTION}\nFound 1 error.\n[*] 1 fixable with `--fix`.\n")
    );
}

#[test]
fn inline_config_goes_before_the_subcommand() {
    let root = project(&[("module.py", "# --- helpers ---\nx = 1\n")]);
    assert_eq!(
        simplint(root.path(), &["--config", "select = ['SMP001']", "check", "--output-format", "concise"]),
        (1, format!("module.py:1:1: {SECTION}\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"), String::new())
    );
}

#[test]
fn inline_config_splits_simplint_and_ruff_keys() {
    let root = project(&[
        ("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"),
        ("module.py", "from __future__ import annotations\nx = 1\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--config", "select = ['SMP108']\ntarget-version = 'py313'"]);
    assert_eq!((code, stdout), (0, "All checks passed!\n".to_owned()));
}

#[test]
fn inline_config_reports_ruff_errors() {
    let root = project(&[("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--config", "line-length = 'x'"]);
    assert_eq!(
        (code, stderr.contains("Could not parse the supplied argument as a `ruff.toml` configuration option")),
        (2, true)
    );
}

#[test]
fn ruff_excludes_apply() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff]\nextend-exclude = [\"generated\"]\n"),
        ("generated/output.py", "x = 1\n"),
        (".gitignore", "ignored.py\n"),
        ("ignored.py", "x = 1\n"),
        ("module.py", "VALUE = 1\n"),
    ]);
    fs::create_dir(root.path().join(".git")).expect("git directory");
    assert_eq!(check(root.path(), &["--select", "SMP010"]).0, 0);
}

#[test]
fn ruff_lint_exclude_applies() {
    let root =
        project(&[("pyproject.toml", "[tool.ruff.lint]\nexclude = [\"module.py\"]\n"), ("module.py", "x = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP010"]).0, 0);
}

const LAZY_ANNOTATIONS: &str = "module.py:1:24: SMP108 [*] Annotations are evaluated lazily from Python 3.14; drop `from \
                                 __future__ import annotations`\nFound 1 error.\n[*] 1 fixable with `--fix`.\n";
const PASSED: &str = "All checks passed!\n";

#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python")]
#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \"~=3.14\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python_compatible")]
#[test_case(&[("pyproject.toml", "[project]\nrequires-python = \"==3.14.*\"\n")], &[], LAZY_ANNOTATIONS ; "requires_python_wildcard")]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &["--target-version", "py313"], PASSED ;
    "flag_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], &["--config", "target-version = 'py313'"], PASSED ;
    "inline_config_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n[tool.ruff]\ntarget-version = \"py312\"\n")], &[], PASSED ;
    "pyproject_ruff_target_version_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"), (".ruff.toml", "target-version = \"py313\"\n")],
    &[],
    PASSED ;
    "dot_ruff_toml_wins"
)]
#[test_case(
    &[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"), ("ruff.toml", "target-version = \"py313\"\n")],
    &[],
    PASSED ;
    "ruff_toml_wins"
)]
#[test_case(
    &[(".config/ruff/ruff.toml", "target-version = \"py312\"\n")], &[], PASSED ;
    "user_configuration_without_a_project_configuration"
)]
#[test_case(
    &[
        (".config/ruff/ruff.toml", "target-version = \"py312\"\n"),
        ("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n"),
    ],
    &[],
    LAZY_ANNOTATIONS ;
    "requires_python_beats_the_user_configuration"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py314\"\nper-file-target-version = { \"legacy.py\" = \"py310\" }\n"),
        ("legacy.py", "from __future__ import annotations\n"),
    ],
    &[],
    LAZY_ANNOTATIONS ;
    "per_file_target_version"
)]
fn target_version_resolution(files: &[(&str, &str)], flags: &[&str], stdout: &str) {
    let root = project(&[files, &[("module.py", "from __future__ import annotations\n")]].concat());
    assert_eq!(check(root.path(), &[&["--select", "SMP108"], flags].concat()).1, stdout);
}

#[test]
fn show_settings_describes_the_first_file() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nrequires-python = \">=3.12\"\ndependencies = [\"requests\"]\nscripts = { tool = \"cli:main\" }\n[tool.simplint]\nselect = [\"SMP001\"]\nper-file-ignores = { \"!b.py\" = [\"SMP001\"] }\nextend-safe-fixes = [\"SMP001\"]\n",
        ),
        ("b.py", ""),
        ("a.py", ""),
    ]);
    let (code, stdout, _) = check(root.path(), &["--show-settings", "--unfixable", "ALL"]);
    let path = |name: &str| common::quoted(&root.path().join(name));
    let (file, pyproject) = (path("a.py"), path("pyproject.toml"));
    // The directories print quoted as they display, the files with escaped backslashes.
    let cache_dir = format!("\"{}\"", root.path().join(".ruff_cache").join("simplint").display());
    let project_root = format!("\"{}\"", root.path().display());
    let ignored = common::quoted(Path::new(&format!("!{}", root.path().join("b.py").display())));
    assert_eq!(
        (code, stdout),
        (
            0,
            format!(
                "Resolved settings for: {file}\nSettings path: {pyproject}\n\n# General Settings\n\
                 cache_dir = {cache_dir}\nfix = false\nfix_only = false\noutput_format = concise\n\
                 show_fixes = false\nunsafe_fixes = hint\n\n# Linter Settings\nlinter.project_root = {project_root}\n\
                 linter.target_version = 3.12\nlinter.rules.enabled = [\n\tsection-comment (SMP001),\n]\n\
                 linter.rules.should_fix = []\nlinter.per_file_ignores = [\n\t{ignored} = [\"section-comment (SMP001)\"],\n]\n\
                 linter.safety_table.forced_safe = [\n\tsection-comment (SMP001),\n]\nlinter.safety_table.forced_unsafe = []\n\
                 linter.dependencies = [\n\t\"requests\",\n]\nlinter.entry_points = [\n\t\"cli:main\",\n]\n"
            )
        )
    );
}

#[test]
fn show_settings_without_a_pyproject_has_no_settings_path() {
    let root = project(&[("module.py", "")]);
    let (code, stdout, _) = check(root.path(), &["--show-settings"]);
    let file = common::quoted(&root.path().join("module.py"));
    assert_eq!(
        (code, stdout.lines().take(3).collect::<Vec<_>>()),
        (0, vec![format!("Resolved settings for: {file}").as_str(), "", "# General Settings"])
    );
}

#[test]
fn show_settings_without_files_fails() {
    let root = project(&[("notes.txt", "")]);
    let (code, _, stderr) = check(root.path(), &["--show-settings"]);
    assert_eq!((code, stderr.contains("No files found under the given path")), (2, true));
}
