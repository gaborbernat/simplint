use super::*;
use crate::common::{check, empty_project, project};
use test_case::test_case;

// A nested project under a `tests` directory is not a test module of the outer project, as its files are its own, while
// a directory of the tests without a marker of its own is.
#[test_case("tests/roots/test-c", true, false ; "nested_project_is_not_a_test_module")]
#[test_case("tests/roots/fixtures", false, true ; "test_directory_without_a_marker_is_a_test_module")]
fn test_module_docstring_follows_the_project_of_the_file(directory: &str, marked: bool, reported: bool) {
    let mut files = vec![(format!("{directory}/parser.py"), "\"\"\"Parser.\"\"\"\n".to_owned())];
    if marked {
        files.push((format!("{directory}/conf.py"), String::new()));
    }
    let files: Vec<(&str, &str)> = files.iter().map(|(name, content)| (name.as_str(), content.as_str())).collect();
    let stdout = check(project(&files).path(), &["--select", "SMP902"]).1;
    assert_eq!(stdout.contains(" SMP902 "), reported, "{stdout}");
}

// A nested project takes the kind of the project around it, as a library's nested roots are library modules.
#[test_case("library", false ; "library")]
#[test_case("application", true ; "application")]
fn nested_project_takes_the_project_kind_around_it(kind: &str, reported: bool) {
    let root = project(&[
        (
            "pyproject.toml",
            &format!("[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"{kind}\"\n"),
        ),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/mod.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains(&common::native("roots/test-a/mod.py")), reported, "{stdout}");
}

// A `[tool.simplint]` of a nested project's own replaces the project kind around it.
#[test_case("[tool.simplint]\nproject-kind = \"application\"\n", true ; "own_application_kind")]
#[test_case("[tool.simplint]\nproject-kind = \"library\"\n", false ; "own_library_kind")]
fn nested_project_with_its_own_settings_takes_them(nested: &str, reported: bool) {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        ("roots/test-a/pyproject.toml", nested),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/mod.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains(&common::native("roots/test-a/mod.py")), reported, "{stdout}");
}

// A nested project's documentation names its own definitions as used, and the documentation of the run's project does
// not reach them, nor the other way round.
#[test]
fn nested_project_names_its_own_documentation() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("index.rst", ".. autofunction:: support.helper\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/index.rst", ".. autofunction:: support.helper\n"),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
        ("roots/test-b/conf.py", ""),
        ("roots/test-b/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    let reported: Vec<&str> = stdout.lines().filter(|line| line.contains(" SMP409 ")).collect();
    assert_eq!(
        reported,
        [format!(
            "{}:1:5: SMP409 `helper` is never used in the run; remove it or give it a caller",
            common::native("roots/test-b/support.py")
        )]
    );
}

// A nested project is indexed on its own, so the run's stores to an attribute reach its names as its reads do: a
// `missing-final` there leaves a name the run assigns alone.
#[test_case("        self.ffi = object()\n", false ; "outer_store_assigns_the_name")]
#[test_case("        pass\n", true ; "without_a_store_the_name_is_reported")]
fn nested_project_sees_the_attributes_the_run_stores_to(store: &str, reported: bool) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/owner.py", &format!("class Owner:\n    def __init__(self) -> None:\n{store}")),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/snip.py", "ffi = object()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP601"]).1;
    assert_eq!(stdout.contains(" SMP601 "), reported, "{stdout}");
}

// A nested project's `pyproject.toml` that does not parse is an error, as the one of the project around it is.
#[test_case("[project\n" ; "invalid_toml")]
#[test_case("[project]\nname = 1\n" ; "invalid_metadata")]
fn invalid_pyproject_of_a_nested_project_is_an_error(content: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/pyproject.toml", content),
        ("roots/test-a/mod.py", "x = 1\n"),
    ]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("Failed to parse")), (2, true), "{stderr}");
}

// A run from a directory whose name is not UTF-8 cannot resolve its imports, whether it checks a file or standard
// input.
#[cfg(unix)]
#[test_case(false ; "linted_file")]
#[test_case(true ; "standard_input")]
fn working_directory_that_is_not_utf8_is_an_error(stdin: bool) {
    use std::os::unix::ffi::OsStrExt;
    let root = empty_project();
    let directory = root.path().join(std::ffi::OsStr::from_bytes(b"bad\xff"));
    std::fs::create_dir(&directory).expect("create directory");
    let (code, _, stderr) = if stdin {
        common::check_with_stdin(&directory, &["--select", "SMP101", "--stdin-filename", "mod.py", "-"], "x = 1\n")
    } else {
        std::fs::write(directory.join("mod.py"), "x = 1\n").expect("write file");
        check(&directory, &["--select", "SMP101"])
    };
    assert_eq!((code, stderr.contains("is not a UTF-8 path")), (2, true), "{stderr}");
}

// A name the run's project reads counts as used in a nested project, so the test module of the run that imports a
// function of a nested project does not leave it unused.
#[test]
fn name_read_by_the_outer_project_is_used_in_a_nested_project() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("use.py", "def run(widget):\n    return widget.helper()\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(!stdout.contains(&common::native("roots/test-a/support.py")), "{stdout}");
    let unread = project(&[
        ("pyproject.toml", DEMO),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    assert!(check(unread.path(), &["--select", "SMP409"]).1.contains(&common::native("roots/test-a/support.py")));
}

// A name a nested project reads does not count as used in the run's project, which does not see the nested project.
#[test]
fn name_read_by_a_nested_project_is_unused_in_the_outer_project() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/support.py", "def helper() -> None:\n    pass\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/use.py", "def run(widget):\n    return widget.helper()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(stdout.contains(&common::native("pkg/support.py:1:5: SMP409 `helper`")), "{stdout}");
}

// What `automodule` with `:members:` documents: the module's public names when it has no `__all__`, the names its
// `__all__` lists when it has one, and with the directive's options the names it excludes or takes in. A directive
// without `:members:` documents the module alone.
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members:\n", &["_hidden"] ; "members_are_the_public_names")]
#[test_case(
    &format!("__all__ = [\"public\"]\n\n\n{FUNCTIONS}"),
    ".. automodule:: pkg.api\n   :members:\n", &["_hidden", "other"] ; "members_are_the_names_of_all"
)]
#[test_case(
    FUNCTIONS,
    concat!(
        ".. automodule:: pkg.api\n",
        "   :members:\n",
        "   :private-members:\n"
    ),
    &[] ; "private_members_are_taken_in"
)]
#[test_case(
    FUNCTIONS,
    concat!(
        ".. automodule:: pkg.api\n",
        "   :members:\n",
        "   :exclude-members: other\n"
    ),
    &["_hidden", "other"] ; "excluded_members_are_left_out"
)]
#[test_case(
    &format!("{FUNCTIONS}\n\ndef __special__() -> None:\n    pass\n"),
    concat!(
        ".. automodule:: pkg.api\n",
        "   :members:\n"
    ),
    &["_hidden"] ; "special_names_are_not_reported_without_the_flag"
)]
#[test_case(
    &format!("{FUNCTIONS}\n\ndef __special__() -> None:\n    pass\n"),
    concat!(
        ".. automodule:: pkg.api\n",
        "   :members:\n",
        "   :special-members:\n"
    ),
    &["_hidden"] ; "special_names_are_not_reported_with_the_flag"
)]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members: public\n", &["_hidden", "other"] ; "listed_members_only")]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n", &["_hidden", "other", "public"] ; "module_without_members")]
fn automodule_members_are_used_as_sphinx_renders_them(source: &str, directive: &str, expected: &[&str]) {
    let root =
        project(&[("pyproject.toml", DEMO), ("pkg/__init__.py", ""), ("pkg/api.py", source), ("index.rst", directive)]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), expected, "{stdout}");
}

// Files of a nested test project are test input for the rules that judge their docstrings and the visibility of their
// names, so an internal docstring, an undocumented public function and a name `__all__` omits are left alone there,
// while another rule still checks them. The same code outside a nested project is reported.
#[test_case("roots/test-a", false ; "nested_project")]
#[test_case("roots/fixtures", true ; "directory_outside_a_nested_project")]
fn nested_project_is_not_judged_for_docstrings_or_exports(directory: &str, reported: bool) {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        (&format!("{directory}/conf.py"), ""),
        (
            &format!("{directory}/mod.py"),
            "__all__ = [\"keep\"]\n\nbadName = 1\n\n\ndef keep() -> None:\n    pass\n\n\ndef unlisted() -> None:\n    \
            pass\n\n\ndef _internal() -> None:\n    \"\"\"Internal doc.\"\"\"\n\n\ndef f(x: int) -> int:\n    return \
            x\n",
        ),
    ]);
    let stdout = check(root.path(), &["--select", "SMP101,SMP401,SMP403,SMP901,SMP903"]).1;
    assert!(stdout.contains(" SMP101 "), "{stdout}");
    let judged = [" SMP901 ", " SMP403 ", " SMP903 "].map(|code| stdout.contains(code));
    assert_eq!(judged, [reported; 3], "{stdout}");
}

#[test]
fn docstring_and_visibility_rules_leave_a_nested_project_alone() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        ("roots/test-a/conf.py", ""),
        (
            "roots/test-a/mod.py",
            "\"\"\"Module.\"\"\"\n\n__all__ = [\"keep\"]\n\nbadName = 1\n\n\ndef keep(x: int) -> int:\n    \"\"\"Keep \
            `x`.\n\n    :param y: missing.\n    :returns: nothing.\n    \"\"\"\n    return x\n\n\ndef unlisted() -> \
            None:\n    pass\n\n\ndef _internal() -> None:\n    \"\"\"Internal doc.\"\"\"\n\n\nclass Holder:\n    \
            \"\"\"Holds.\n\n    :ivar count: a count.\n    \"\"\"\n\n    count: int = 0\n",
        ),
    ]);
    let stdout = check(root.path(), &["--select", "SMP9,SMP102,SMP401,SMP402,SMP403,SMP404"]).1;
    assert_eq!(stdout, common::PASSED);
    let outside = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        ("roots/fixtures/mod.py", "def _internal() -> None:\n    \"\"\"Internal doc.\"\"\"\n"),
    ]);
    assert!(check(outside.path(), &["--select", "SMP901"]).1.contains("SMP901"));
}

// A module that a test patches by attribute keeps its `module.Name` reads: binding the name at import time would miss
// the patch, so SMP304 offers no fix for any of the patch forms.
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_run() -> None:\n",
        "    with mock.patch(\"bench.operations.OPERATIONS\"):\n",
        "        pass\n"
    ) ; "dotted_string"
)]
#[test_case(
    concat!(
        "from bench import operations\n",
        "\n",
        "\n",
        "def test_run() -> None:\n",
        "    setattr(operations, \"OPERATIONS\", {})\n"
    ) ; "setattr"
)]
#[test_case(
    concat!(
        "from bench import operations\n",
        "\n",
        "\n",
        "def test_run(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(operations, \"OPERATIONS\", {})\n"
    ) ; "monkeypatch"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "from bench import operations\n",
        "\n",
        "\n",
        "def test_run() -> None:\n",
        "    with mock.patch.object(operations, \"OPERATIONS\"):\n",
        "        pass\n"
    ) ; "patch_object"
)]
fn patched_module_attribute_keeps_module_reads(test: &str) {
    let root = project(&[
        ("tools/bench/__init__.py", ""),
        ("tools/bench/operations.py", "OPERATIONS = {}\n\n\ndef run() -> dict:\n    return OPERATIONS\n"),
        (
            "tools/bench/report.py",
            "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n",
        ),
        ("tests/test_ops.py", test),
    ]);
    let (_, stdout, _) = check(root.path(), &["--select", "SMP304", "--unsafe-fixes", "--fix"]);
    assert_eq!(stdout, common::PASSED);
    assert_eq!(
        common::read(root.path(), "tools/bench/report.py"),
        "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n"
    );
    let unpatched = project(&[
        ("tools/bench/__init__.py", ""),
        ("tools/bench/operations.py", "OPERATIONS = {}\n\n\ndef run() -> dict:\n    return OPERATIONS\n"),
        (
            "tools/bench/report.py",
            "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n",
        ),
    ]);
    check(unpatched.path(), &["--select", "SMP304", "--unsafe-fixes", "--fix"]);
    assert_ne!(
        common::read(unpatched.path(), "tools/bench/report.py"),
        "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n"
    );
}

// A store to one attribute of a module from another file, or a dotted string patching one, touches only that attribute:
// the module's other attributes still move into the importer.
#[test_case(
    "pkg/setup.py",
    concat!(
        "from pkg import exc\n",
        "\n",
        "exc._version_token = 1\n"
    ),
    concat!(
        "pkg/use.py:5:12: SMP304 Import `Error` from `pkg.exc` instead of reaching it through the module\n",
        "pkg/use.py:5:23: SMP304 Import `Other` from `pkg.exc` instead of reaching it through the module\n",
        "Found 2 errors.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "store"
)]
#[test_case(
    "pkg/test_use.py",
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_it() -> None:\n",
        "    with mock.patch(\"pkg.exc.Error\"):\n",
        "        pass\n"
    ),
    concat!(
        "pkg/use.py:5:23: SMP304 Import `Other` from `pkg.exc` instead of reaching it through the module\n",
        "Found 1 error.\n"
    ) ; "patched"
)]
fn stored_attribute_leaves_other_attributes_reported(path: &str, body: &str, expected: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/exc.py", "class Error(Exception):\n    pass\n\n\nclass Other(Exception):\n    pass\n"),
        ("pkg/use.py", "from pkg import exc\n\n\ndef check() -> object:\n    return exc.Error, exc.Other\n"),
        (path, body),
    ]);
    let (_, stdout, _) = check(root.path(), &["--select", "SMP304"]);
    assert_eq!(stdout, common::native(expected));
}

// A doctest reads a module's name, which is no patch of it, so the module's own reads still get the import fix.
#[test]
fn doctest_read_does_not_patch_module() {
    let root = project(&[
        ("tools/bench/__init__.py", ""),
        (
            "tools/bench/operations.py",
            "OPERATIONS = {}\n\n\ndef run() -> dict:\n    \"\"\"Return the table.\n\n    >>> from bench import \
            operations\n    >>> operations.OPERATIONS\n    {}\n    \"\"\"\n    return OPERATIONS\n",
        ),
        (
            "tools/bench/report.py",
            "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304"]).1,
        common::native(
            "tools/bench/report.py:5:12: SMP304 Import `OPERATIONS` from `bench.operations` instead of reaching it \
             through the module\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A nested test project's pages name its own definitions, and do not count as the run's project's documentation, which
// leaves the run's unused definition reported.
#[test]
fn nested_project_pages_do_not_name_the_run_projects_definitions() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("support.py", "def helper() -> None:\n    pass\n"),
        ("tests/roots/test-a/conf.py", ""),
        ("tests/roots/test-a/index.rst", ".. autofunction:: support.helper\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(stdout.contains(&common::native("support.py")), "{stdout}");
}

#[test]
fn sarif_related_location_is_a_file_uri() {
    let root = project(&[
        ("pyproject.toml", DEMO_BUILD),
        ("models.py", &format!("__all__ = []\n\n{PARTITION}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    let report: serde_json::Value =
        common::json(root.path(), &["check", "--select", "SMP703", "--output-format", "sarif"]);
    let result = &report["runs"][0]["results"][0];
    let primary = result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"].as_str().unwrap_or_default();
    let related =
        result["relatedLocations"][0]["physicalLocation"]["artifactLocation"]["uri"].as_str().unwrap_or_default();
    assert_eq!((primary.starts_with("file://"), related.starts_with("file://")), (true, true), "{related}");
}

// An autodoc directive names a package's function by the package, so the `__init__.py` function it documents is public
// documentation rather than internal code.
#[test]
fn autodoc_names_a_package_function_by_its_package() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        (
            "app/__init__.py",
            "def helper() -> int:\n    \"\"\"Compute the helper value from the environment and return it to the \
            caller.\"\"\"\n    return 2\n",
        ),
        ("docs/conf.py", ""),
        ("docs/index.rst", "API\n===\n\n.. autofunction:: app.helper\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP901,SMP409"]).1;
    assert_eq!(stdout, common::PASSED);
    let undocumented = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        (
            "app/__init__.py",
            concat!(
                "def helper() -> int:\n",
                "    \"\"\"Compute the helper value from the environment and return it to the caller.\"\"\"\n",
                "    return 2\n"
            ),
        ),
        ("docs/conf.py", ""),
    ]);
    assert!(check(undocumented.path(), &["--select", "SMP901"]).1.contains("SMP901"));
}

// An autodoc directive names a method by its class, so the method's docstring is documented rather than internal.
#[test]
fn autodoc_names_a_method_by_its_class() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        ("app/__init__.py", ""),
        (
            "app/mod.py",
            "__all__ = [\"Foo\"]\n\n\nclass Foo:\n    def run(self) -> int:\n        \"\"\"Run the thing and give back \
            the exit status of the subprocess it starts.\"\"\"\n        return 1\n",
        ),
        ("docs/conf.py", ""),
        ("docs/index.rst", "API\n===\n\n.. autoclass:: app.mod.Foo\n   :members:\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP901"]).1;
    assert_eq!(stdout, common::PASSED);
    let undocumented = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        ("app/__init__.py", ""),
        (
            "app/mod.py",
            concat!(
                "__all__ = [\"Foo\"]\n",
                "\n",
                "\n",
                "class Foo:\n",
                "    def run(self) -> int:\n",
                "        \"\"\"Run the thing and give back the exit status of the subprocess it starts.\"\"\"\n",
                "        return 1\n"
            ),
        ),
        ("docs/conf.py", ""),
    ]);
    assert!(check(undocumented.path(), &["--select", "SMP901"]).1.contains("SMP901"));
}

// A nested project's files are test input, so the rules it leaves out do not run there. Its results are cached apart
// from the outer project's, which judges the same file.
#[test]
fn nested_project_results_are_not_reused_by_the_outer_project() {
    let docstring = format!("def run() -> None:\n    \"\"\"{}\"\"\"\n", "word ".repeat(150).trim_end());
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"outer\"\nversion = \"0\"\n"),
        ("tests/proj/pyproject.toml", "[project]\nname = \"proj\"\nversion = \"0\"\n"),
        ("tests/proj/mod.py", &docstring),
    ]);
    let cache = root.path().join("cache");
    let cache = cache.to_str().expect("a UTF-8 path");
    let nested = root.path().join("tests/proj");
    let (_, inner, _) =
        common::simplint(&nested, &["check", "--cache-dir", cache, "--select", "SMP904", "--output-format", "concise"]);
    assert!(inner.contains("SMP904"), "{inner}");
    let (_, outer, _) = common::simplint(
        root.path(),
        &["check", "--cache-dir", cache, "--select", "SMP904", "--output-format", "concise"],
    );
    assert_eq!(outer, common::PASSED);
}

// coverage loads a plugin module by its dotted name and calls its `coverage_init`, so no import reaches the function.
#[test_case("__all__ = [\"coverage_init\"]\n" ; "listed")]
#[test_case("__all__ = []\n" ; "unlisted")]
fn coverage_plugin_init_is_used(all: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}{all}")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP402,SMP403,SMP409"]).1, common::PASSED);
    let unconfigured = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}{all}")),
    ]);
    assert!(check(unconfigured.path(), &["--select", "SMP402,SMP403,SMP409"]).1.contains("`coverage_init`"));
}
