use super::*;
use crate::common::{check, native, project};
use test_case::test_case;

const UNDOCUMENTED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    \
    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \"\"\"Format messages.\"\"\"\n";

const DOCSTRINGED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    \
    \"\"\"Register the flag.\"\"\"\n    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \
    \"\"\"Format messages.\"\"\"\n";

// A bare module entry point is loaded whole by its host, which finds the hooks and fixtures it defines by name, so its
// public names are public API whatever `__all__` lists. An entry point naming an attribute makes only that one public.
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    common::PASSED ; "bare_module_hook_is_not_internal"
)]
#[test_case(
    "entry-points.pytest11.demo = \"missing.plugin\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    concat!(
        "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and ",
        "types document it\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "unresolved_module_names_nothing"
)]
#[test_case(
    "",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    concat!(
        "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and ",
        "types document it\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "unlisted_hook_is_internal"
)]
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin:Formatter\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    concat!(
        "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and ",
        "types document it\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "attribute_entry_point_names_one_name"
)]
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP401",
    common::PASSED ; "bare_module_names_are_not_imports"
)]
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    UNDOCUMENTED_PLUGIN,
    "SMP903",
    concat!(
        "pkg/plugin.py:4:5: SMP903 Public function `pytest_addoption` has no docstring; document it for ",
        "its users\n",
        "Found 1 error.\n"
    ) ; "bare_module_undocumented_hook_reported"
)]
#[test_case(
    "",
    UNDOCUMENTED_PLUGIN,
    "SMP903",
    common::PASSED ; "unlisted_undocumented_hook_not_reported"
)]
fn bare_module_entry_point_names_are_public_api(entry_points: &str, source: &str, rule: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", ""),
        ("pkg/plugin.py", source),
    ]);
    assert_eq!(check(root.path(), &["--select", rule]).1, stdout.replace("pkg/", &native("pkg/")));
}

// A target that is no module name, a standard library module or a namespace package names no file of the run, so the
// plugin's hook stays internal.
#[test_case("pkg..plugin" ; "invalid_module_name")]
#[test_case("json" ; "standard_library")]
#[test_case("space" ; "namespace_package")]
fn entry_point_without_a_run_file_names_nothing(target: &str) {
    let root = project(&[
        (
            "pyproject.toml",
            &format!(
                "[project]\nname = \"pkg\"\nentry-points.pytest11.demo = \"{target}\"\n[build-system]\nrequires = []\n"
            ),
        ),
        ("pkg/__init__.py", ""),
        ("pkg/plugin.py", DOCSTRINGED_PLUGIN),
        ("space/module.py", ""),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP901"]).1,
        native(
            "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and types document \
             it\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A project directory whose name is not UTF-8 is no search path, so its entry points name nothing.
#[cfg(unix)]
#[test]
fn entry_point_of_a_project_whose_directory_is_not_utf8_names_nothing() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    let directory = Path::new(OsStr::from_bytes(b"pr\xffoject"));
    let pyproject =
        "[project]\nname = \"pkg\"\nentry-points.pytest11.demo = \"pkg.plugin\"\n[build-system]\nrequires = []\n";
    let root = project(&[
        (directory.join("pyproject.toml"), pyproject),
        (directory.join("pkg/__init__.py"), ""),
        (directory.join("pkg/plugin.py"), DOCSTRINGED_PLUGIN),
    ]);
    let (code, stdout, stderr) = check(root.path(), &["--select", "SMP901"]);
    assert_eq!(common::reports(&stdout, "SMP901").len(), 1, "{code} {stdout} {stderr}");
}

// A `from __future__` import binds a compiler flag that the host of a bare module entry point never reads, so it is no
// public name for `missing-from-all` to list.
#[test]
fn bare_module_future_import_is_not_public_api() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"pkg\"\nentry-points.pytest11.demo = \"pkg.plugin\"\n[build-system]\nrequires = []\n",
        ),
        ("pkg/__init__.py", ""),
        (
            "pkg/plugin.py",
            "\"\"\"Plugin.\"\"\"\n\nfrom __future__ import annotations\n\n__all__ = [\"pytest_addoption\"]\n\n\ndef \
            pytest_addoption(parser: object) -> None:\n    \"\"\"Register the flag.\"\"\"\n    print(parser)\n",
        ),
    ]);
    common::assert_clean(root.path(), "SMP401");
    let imported = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"pkg\"\nentry-points.pytest11.demo = \"pkg.plugin\"\n[build-system]\nrequires = []\n",
        ),
        ("pkg/__init__.py", ""),
        (
            "pkg/plugin.py",
            concat!(
                "__all__ = [\"pytest_addoption\"]\n",
                "\n",
                "\n",
                "def pytest_addoption(parser: object) -> None:\n",
                "    print(parser)\n",
                "\n",
                "\n",
                "def helper() -> None:\n",
                "    pass\n"
            ),
        ),
        ("pkg/other.py", "from pkg.plugin import helper\n\nhelper()\n"),
    ]);
    assert!(check(imported.path(), &["--select", "SMP401"]).1.contains("`helper` is imported by other modules"));
}

const UNUSED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    \
    print(parser)\n\n\ndef helper() -> None:\n    print(\"helper\")\n\n\n__all__ = [\"pytest_addoption\"]\n";

// The host of a bare module entry point uses every public name of it, so none of them is unused, and an `__all__` entry
// it does not import is not a stale export.
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    common::PASSED ; "bare_module_names_are_used"
)]
#[test_case(
    "",
    concat!(
        "pkg/plugin.py:8:5: SMP409 `helper` is never used in the run; remove it or give it a caller\n",
        "Found 1 error.\n"
    ) ; "unlisted_names_are_unused"
)]
fn bare_module_entry_point_names_are_used(entry_points: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", ""),
        ("pkg/plugin.py", UNUSED_PLUGIN),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402,SMP409"]).1, stdout.replace("pkg/", &native("pkg/")));
}

// A bare package entry point is its `__init__.py`, whose imports from a private module are public API the way a
// re-export is. Without the entry point, a plain import in a module without `__all__` keeps the name internal.
#[test_case(
    "entry-points.pytest11.demo = \"pkg\"\n",
    concat!(
        "pkg/_core.py:1:5: SMP903 Public function `plugin_hook` has no docstring; document it for its ",
        "users; `pkg/__init__.py` re-exports it\n",
        "Found 1 error.\n"
    ) ; "package_imports_are_public"
)]
#[test_case("", common::PASSED ; "unlisted_package_imports_stay_internal")]
fn bare_package_entry_point_imports_are_public_api(entry_points: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", "from pkg._core import plugin_hook\n"),
        ("pkg/_core.py", "def plugin_hook() -> None:\n    print()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP903"]).1, stdout.replace("pkg/", &native("pkg/")));
}

// The `__all__` of a public module of a library is the interface its users import, so no module of the run needs to
// import it. A private module of the same library still reports the names nothing imports.
#[test_case("pkg/errors.py", common::PASSED ; "public_module_kept")]
#[test_case(
    "pkg/_errors.py",
    concat!(
        "pkg/_errors.py:4:12: SMP402 `Oops` is in `__all__`, yet no other module imports it; list only ",
        "what other modules use\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "private_module_reported"
)]
fn public_module_all_is_the_declared_api(module: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", "\"\"\"Package.\"\"\"\n"),
        (module, "class Oops(Exception): ...\n\n\n__all__ = [\"Oops\"]\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, stdout.replace("pkg/", &native("pkg/")));
}

// A module that a file loads as a value, or that a star import binds from its package's `__all__`, has every name read
// through it, so none of its names is reported. Reading only one name through a module leaves the others reported.
#[test_case("from app import config as c\n\nsettings = c\nprint(settings.LIMIT)\n", common::PASSED ; "loaded_as_value")]
#[test_case("from app import *\n\nprint(config.LIMIT)\n", common::PASSED ; "star_bound_by_package_all")]
#[test_case(
    concat!(
        "from app import config\n",
        "\n",
        "print(config.LIMIT)\n"
    ),
    concat!(
        "app/config.py:8:21: SMP402 [*] `local` is in `__all__`, yet no other module imports it; list ",
        "only what other modules use\n",
        "Found 1 error.\n",
        "[*] 1 fixable with `--fix`.\n"
    ) ; "module_attribute"
)]
fn names_read_through_a_whole_module_are_used(consumer: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("app/__init__.py", "__all__ = [\"config\"]\n"),
        ("app/config.py", "LIMIT = 3\n\n\ndef local() -> int:\n    return 1\n\n\n__all__ = [\"LIMIT\", \"local\"]\n"),
        ("cli.py", consumer),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, stdout.replace("app/", &native("app/")));
}

// Only a production file's star import binds the submodules a package's `__all__` lists, so a test's star import of the
// package exempts none of them.
#[test]
fn star_import_in_a_test_exempts_no_submodule_name() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", "__all__ = [\"sub\"]\n"),
        ("pkg/sub.py", "LIMIT = 3\n\n\ndef func() -> int:\n    return LIMIT\n\n\n__all__ = [\"LIMIT\", \"func\"]\n"),
        ("cli.py", "from pkg.sub import LIMIT\n\nprint(LIMIT)\n"),
        ("tests/test_pkg.py", "from pkg import *\n\n\ndef test_it() -> None:\n    assert sub\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        "pkg/sub.py:8:21: SMP402 [*] `func` is in `__all__`, yet no other module imports it; list only what other \
         modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
            .replace("pkg/", &native("pkg/"))
    );
}

// A `__all__` read off a module the index cannot name may be any module's, so it does not exempt the `__all__` of a
// module the run defines.
#[test]
fn all_read_off_an_unnamed_module_keeps_other_modules_checked() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("a.py", "LIMIT = 3\n\n\ndef local() -> int:\n    return 1\n\n\n__all__ = [\"LIMIT\", \"local\"]\n"),
        ("b.py", "from a import LIMIT\n\nprint(LIMIT)\n"),
        (
            "tests/test_json.py",
            "import sys\n\n\ndef test_json_lists_names() -> None:\n    module = sys.modules[\"json\"]\n    assert \
            \"dumps\" in module.__all__\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        "a.py:8:21: SMP402 [*] `local` is in `__all__`, yet no other module imports it; list only what other modules \
        use\n\
         Found 1 error.\n[*] 1 fixable with `--fix`.\n"
    );
}

#[test_case("from a import __all__ as base\n\n__all__ = [*base, \"Z\"]\n" ; "imported")]
#[test_case("import a\n\n__all__ = [*a.__all__, \"Z\"]\n" ; "read_through_module")]
#[test_case("import a\n\nprint([name for name in a.__all__])\n" ; "iterated_through_module")]
fn names_another_module_reads_from_all_are_used(consumer: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("a.py", "X = 1\nY = 2\n\n__all__ = [\"X\", \"Y\"]\n"),
        ("b.py", consumer),
    ]);
    common::assert_clean(root.path(), "SMP402");
    let unread = project(&[
        ("pyproject.toml", DEMO),
        ("a.py", "X = 1\nY = 2\n\n__all__ = [\"X\", \"Y\"]\n"),
        ("b.py", "print(1)\n"),
    ]);
    assert!(check(unread.path(), &["--select", "SMP402"]).1.contains("`X`"));
}

#[test_case(
    concat!(
        "import pkgutil\n",
        "\n",
        "HANDLER = pkgutil.resolve_name(\"pkg._cli._validate:run\")\n"
    ),
    true ; "direct_resolve_name"
)]
#[test_case(
    concat!(
        "from pkgutil import resolve_name\n",
        "\n",
        "HANDLER = resolve_name(\"pkg._cli._validate:run\")\n"
    ),
    true ; "imported_resolve_name"
)]
#[test_case(
    concat!(
        "def build_parser(parser):\n",
        "    parser.set_defaults(handler=\"pkg._cli._validate:run\")\n"
    ),
    true ; "argparse_handler"
)]
#[test_case("HANDLER = \"pkg._cli._validate:run\"\n", false ; "unresolved_string")]
#[test_case(
    concat!(
        "def build_parser(parser):\n",
        "    parser.set_defaults(description=\"pkg._cli._validate:run\")\n"
    ),
    false ; "other_default"
)]
#[test_case(
    concat!(
        "def build_parser(parser):\n",
        "    parser.set_defaults(handler=\"pkg._cli._validate:run!\")\n"
    ),
    false ; "invalid_attribute"
)]
#[test_case(
    concat!(
        "import pkgutil\n",
        "\n",
        "HANDLER = pkgutil.resolve_name(\"pkg._cli._validate.run\")\n"
    ),
    true ; "dotted_resolve_name"
)]
fn string_named_handlers_count_as_imported(consumer: &str, imported: bool) {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_cli/__init__.py", ""),
        ("pkg/_cli/_validate.py", "def run() -> None: ...\n\n\n__all__ = [\"run\"]\n"),
        ("pkg/_cli/_parser.py", consumer),
    ]);
    let report = check(root.path(), &["--select", "SMP402"]).1;
    assert_eq!(report == common::PASSED, imported, "{report}");
}

#[test]
fn string_named_handler_requires_an_export() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_cli/__init__.py", ""),
        ("pkg/_cli/_validate.py", "def run() -> None: ...\n"),
        (
            "pkg/_cli/_parser.py",
            "def build_parser(parser):\n    parser.set_defaults(handler=\"pkg._cli._validate:run\")\n",
        ),
    ]);
    assert!(check(root.path(), &["--select", "SMP401"]).1.contains("`run` is imported by other modules"));
}

#[test_case(
    concat!(
        "[tool.setuptools.dynamic]\n",
        "version = { attr = \"pkg._plugin.version\" }\n"
    ),
    "version" ; "setuptools_dynamic_attribute"
)]
#[test_case("[tool.setuptools.cmdclass]\nbuild_py = \"pkg._plugin.Build\"\n", "Build" ; "setuptools_command_class")]
#[test_case("[tool.mypy]\nplugins = [\"pkg._plugin\"]\n", "plugin" ; "mypy_plugin")]
#[test_case("[tool.mypy]\nplugins = [\"pkg._plugin:custom\"]\n", "custom" ; "mypy_custom_plugin")]
#[test_case("[tool.pylint.main]\nload-plugins = [\"pkg._plugin\"]\n", "register" ; "pylint_plugin")]
#[test_case("[tool.poetry.plugins.example]\nplugin = \"pkg._plugin:activate\"\n", "activate" ; "poetry_plugin")]
fn pyproject_named_attribute_counts_as_used(tool: &str, name: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}{tool}")),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", &format!("__all__ = [\"{name}\"]\n\n\ndef {name}() -> None: ...\n")),
    ]);
    common::assert_clean(root.path(), "SMP402");
    let unused = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", &format!("__all__ = [\"{name}\"]\n\n\ndef {name}() -> None: ...\n")),
    ]);
    assert!(check(unused.path(), &["--select", "SMP402"]).1.contains(&format!("`{name}`")));
}
