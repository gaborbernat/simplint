use super::*;
use crate::common::{check, project};
use test_case::test_case;

// Only `coverage_init` is called by coverage, so another function of the plugin module is still reported.
#[test]
fn coverage_plugin_keeps_its_other_functions_reported() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}def helper() -> None:\n    pass\n\n\n__all__ = []\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP409"]).1,
        common::native(
            "tasks/coverage_pragmas.py:5:5: SMP409 `helper` is never used in the run; remove it or give it a caller\n\
             Found 1 error.\n"
        )
    );
}

// A plugin resolves from the directories pytest's `pythonpath` and ty's `extra-paths` put on the project's import path.
#[test_case("[tool.pytest.ini_options]\npythonpath = [\"tasks\"]\n" ; "pytest_ini_options")]
#[test_case("[tool.pytest]\npythonpath = [\"tasks\"]\n" ; "pytest")]
#[test_case("[tool.ty.environment]\nextra-paths = [\"tasks\"]\n" ; "ty")]
#[test_case("[tool.ty.environment]\nextra-paths = [\"typestubs\", \"tasks\"]\n" ; "ty_with_a_missing_directory")]
fn coverage_plugin_resolves_from_import_paths(paths: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"cov\"]\n{paths}")),
        ("tasks/cov.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP402,SMP403,SMP409"]).1, common::PASSED);
    let without_paths = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"cov\"]\n"),
        ("tasks/cov.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert!(check(without_paths.path(), &["--select", "SMP409"]).1.contains("`coverage_init`"));
}

// A plugin coverage does not load is judged as any other module, so its `coverage_init` is reported: one not listed,
// and one whose module no import path names, which coverage cannot find.
#[test_case("plugins = [\"tasks.other\"]\n", "tasks/coverage_pragmas.py" ; "not_listed")]
#[test_case("plugins = [\"cov\"]\n", "tasks/cov.py" ; "outside_its_import_paths")]
fn unloaded_coverage_plugin_init_is_reported(plugins: &str, file: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\n[tool.coverage.run]\n{plugins}")),
        ("tasks/__init__.py", ""),
        (file, &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403,SMP409"]).1,
        common::native(&format!(
            "{file}:1:5: SMP403 `coverage_init` is not in `__all__`; name it `_coverage_init`\n\
             {file}:1:5: SMP409 `coverage_init` is never used in the run; remove it or give it a caller\n\
             Found 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        ))
    );
}

// A test module that imports a plugin's `coverage_init` uses it outside the tests, as coverage does, so it is not
// reported as test-only code.
#[test]
fn coverage_plugin_init_imported_by_a_test_is_not_test_only() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
        (
            "tests/test_coverage.py",
            "from tasks.coverage_pragmas import coverage_init\n\n\n\
             def test_init() -> None:\n    coverage_init(object(), {})\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, common::PASSED);
    let unconfigured = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
        (
            "tests/test_coverage.py",
            concat!(
                "from tasks.coverage_pragmas import coverage_init\n",
                "\n",
                "\n",
                "def test_init() -> None:\n",
                "    coverage_init(object(), {})\n"
            ),
        ),
    ]);
    assert!(check(unconfigured.path(), &["--select", "SMP408"]).1.contains("`coverage_init`"));
}
