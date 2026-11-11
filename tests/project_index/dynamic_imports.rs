use super::*;
use crate::common::{check, project};
use test_case::test_case;

// A module loaded by a string-named import call is the module the names read off its result belong to, whether the
// call's result is read directly or through a variable it is assigned to.
#[test_case(
    "import importlib\n\n\ndef test_app() -> None:\n    assert importlib.import_module(\"docs.tutorial\").client\n" ;
    "import_module"
)]
#[test_case("def test_app() -> None:\n    assert __import__(\"docs.tutorial\").tutorial.client\n" ; "dunder_import")]
#[test_case(
    "def test_app() -> None:\n    assert __import__(\"docs.tutorial\", fromlist=[\"client\"]).client\n" ;
    "dunder_import_with_fromlist"
)]
#[test_case(
    "import pytest\n\n\ndef test_app() -> None:\n    assert pytest.importorskip(\"docs.tutorial\").client\n" ;
    "importorskip"
)]
#[test_case(
    "import pytest\n\n\ndef test_app() -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    assert \
    mod.client\n" ;
    "importorskip_variable"
)]
fn attributes_of_statically_named_dynamic_imports_keep_their_names(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe \
            fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A variable a string-named import call loads a module into, and some other value binds too, reads the names of any
// module, so a name read off it is not reported.
#[test]
fn variable_rebound_to_another_value_reads_names_of_any_module() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        (
            "tests/test_docs.py",
            "import pytest\n\n\ndef test_app(other) -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    \
            mod = \
             other\n    assert mod.client\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe \
            fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

#[test]
fn variable_reused_for_each_loaded_module_reads_each_module() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\n"),
        ("docs/other.py", "engine = object()\n"),
        (
            "tests/test_docs.py",
            "import pytest\n\n\ndef test_client() -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    \
            assert \
             mod.client\n\n\ndef test_engine() -> None:\n    mod = pytest.importorskip(\"docs.other\")\n    assert \
             mod.engine\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP404"]).1, common::PASSED);
}

// `__import__` returns the top package of a dotted name, so the names read off its result are the package's, and a
// submodule's are read off the submodule.
#[test]
fn dunder_import_reads_names_of_the_top_package() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        ("tests/test_docs.py", "def test_app() -> None:\n    assert __import__(\"docs.tutorial\").client\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:1:1: SMP404 `client` is not in `__all__`; name it `_client`\ndocs/tutorial.py:2:1: \
            SMP404 \
             `engine` is not in `__all__`; name it `_engine`\nFound 2 errors.\n2 unsafe fixes available with `--fix \
             --unsafe-fixes`.\n"
        )
    );
}

// A name a function binds to a module reads the names of that module, and a second function's binding of the same name
// to another module does not hide the second module's names from the reports.
#[test]
fn names_bound_to_a_module_in_one_function_do_not_clash_with_another() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/cli.py", "__all__ = [\"client\"]\n\nclient = object()\n"),
        ("pkg/other.py", "__all__: list[str] = []\n\nengine = object()\n"),
        (
            "tests/test_cli.py",
            "import pytest\n\n\ndef test_one() -> None:\n    m = pytest.importorskip(\"pkg.cli\")\n    assert \
            m.client\n\n\n\
             def test_two() -> None:\n    m = object()\n    assert m.engine\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "pkg/other.py:3:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A module-level name a function shadows with a local of the same name, as `proxy` is in a test, reads nothing: the
// local's attributes are not the module's, so no other file's variable of that name counts as read.
#[test]
fn local_named_like_a_module_variable_reads_no_module() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("scripts/run.py", "import sys\n\nif __name__ == \"__main__\":\n    url = sys.argv[1]\n    print(url)\n"),
        (
            "tests/test_proxy.py",
            "import pytest\n\nproxy = pytest.importorskip(\"proxy\")\n\n\ndef test_one(make_proxy) -> None:\n    proxy \
            = \
             make_proxy()\n    assert proxy.url\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP102"]).1.matches("scripts/run.py").count(), 0);
}

// A name a function binds to several modules reads the names of each of them, and of no other module. Binding it to a
// value that is no module too keeps those reads.
#[test_case(
    "    m = pytest.importorskip(\"pkg.cli\")\n    assert m.client\n    m = pytest.importorskip(\"pkg.other\")\n    \
    assert m.engine\n",
    false ; "several_modules"
)]
#[test_case(
    "    m = pytest.importorskip(\"pkg.cli\")\n    assert m.client\n    m = object()\n    assert m.engine\n",
    true ; "module_and_value"
)]
fn name_bound_to_several_modules_reads_each_of_them(body: &str, other_reported: bool) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/cli.py", "__all__ = [\"client\"]\n\nclient = object()\n"),
        ("pkg/other.py", "__all__: list[str] = []\n\nengine = object()\n"),
        ("pkg/third.py", "__all__: list[str] = []\n\nclient = object()\n"),
        ("tests/test_cli.py", &format!("import pytest\n\n\ndef test_app() -> None:\n{body}")),
    ]);
    let stdout = check(root.path(), &["--select", "SMP404"]).1;
    assert!(stdout.contains("pkg/third.py:3:1: SMP404 `client`"), "{stdout}");
    assert_eq!(stdout.contains("pkg/other.py:3:1: SMP404 `engine`"), other_reported, "{stdout}");
}

// The attribute of an import call's result is a name of the module the call loads, not a module itself, so a name a
// function binds to it reads nothing, and the names another function reads off a local of that name are reported.
#[test]
fn attribute_of_an_import_call_result_is_no_module() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/cli.py", "__all__ = [\"client\"]\n\nclient = object()\n"),
        ("pkg/other.py", "__all__: list[str] = []\n\nengine = object()\n"),
        (
            "tests/test_cli.py",
            "import importlib\n\n\ndef test_one() -> None:\n    app = importlib.import_module(\"pkg.cli\").client\n    \
            assert \
             app\n\n\ndef test_two() -> None:\n    app = object()\n    assert app.engine\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "pkg/other.py:3:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// `__import__` returns the package a dotted name starts with, unless its `fromlist` names something: an empty or absent
// one, given by keyword or by position, names nothing, while a non-empty literal, or one that may hold names, returns
// the module itself.
#[test_case(
    "__import__(\"pkg.sub\", globals(), locals(), [], 0)", "pkg/sub.py:3:1: SMP404 `helper`", "pkg/__init__.py:3:1" ;
    "positional_empty_list"
)]
#[test_case("__import__(\"pkg.sub\", fromlist=())", "pkg/sub.py:3:1: SMP404 \
    `helper`", "pkg/__init__.py:3:1" ; "empty_tuple")]
#[test_case("__import__(\"pkg.sub\", fromlist=None)", "pkg/sub.py:3:1: SMP404 \
    `helper`", "pkg/__init__.py:3:1" ; "none")]
#[test_case(
    "__import__(\"pkg.sub\", fromlist=[\"x\"])", "pkg/__init__.py:3:1: SMP404 `helper`", "pkg/sub.py:3:1" ;
    "literal_names"
)]
#[test_case(
    "__import__(\"pkg.sub\", fromlist=names)", "pkg/__init__.py:3:1: SMP404 `helper`", "pkg/sub.py:3:1" ;
    "unknown_fromlist"
)]
fn dunder_import_fromlist_selects_the_module(call: &str, reported: &str, unreported: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", "__all__: list[str] = []\n\nhelper = object()\n"),
        ("pkg/sub.py", "__all__: list[str] = []\n\nhelper = object()\n"),
        ("tests/test_a.py", &format!("def test_app(names) -> None:\n    assert {call}.helper\n")),
    ]);
    let stdout = check(root.path(), &["--select", "SMP404"]).1;
    assert!(stdout.contains(reported) && !stdout.contains(unreported), "{stdout}");
}

// A module a spec loads from a path the project names reads the names read off it, whichever way the path is spelled.
#[test_case("Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"" ; "parents_index")]
#[test_case("Path(__file__).parent.parent / \"docs/tutorial.py\"" ; "parent_chain")]
#[test_case("\"docs/tutorial.py\"" ; "string_literal")]
#[test_case("pathlib.Path(\"docs\", \"tutorial.py\")" ; "pathlib_path")]
#[test_case("os.path.join(\"docs\", \"tutorial.py\")" ; "join_literals")]
#[test_case("os.path.join(os.path.dirname(__file__), \"..\", \"docs\", \"tutorial.py\")" ; "dirname_join")]
fn attributes_of_spec_loaded_modules_keep_their_names(path: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/other.py", "client = object()\n"),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        ("tests/test_docs.py", &spec_loading_test(path)),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/other.py:1:1: SMP404 `client` is not in `__all__`; name it `_client`\ndocs/tutorial.py:2:1: SMP404 \
             `engine` is not in `__all__`; name it `_engine`\nFound 2 errors.\n2 unsafe fixes available with `--fix \
             --unsafe-fixes`.\n"
        )
    );
}

// A spec whose path no static path names, or names outside the project, loads a module whose names are unknown, as a
// computed import does.
#[test_case("Path(os.environ[\"DOCS\"]) / \"tutorial.py\"" ; "computed_path")]
#[test_case("DIRS[0] / \"tutorial.py\"" ; "subscript_of_name")]
#[test_case("Path(__file__).name / \"tutorial.py\"" ; "attribute_other_than_parent")]
#[test_case("\"/elsewhere/docs/tutorial.py\"" ; "outside_project")]
#[test_case("\"docs/tutorial.txt\"" ; "not_python_file")]
#[test_case("os.getcwd()" ; "unknown_call")]
fn attributes_of_unresolved_spec_loaded_modules_keep_every_name(path: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/other.py", "client = object()\n"),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        ("tests/test_docs.py", &spec_loading_test(path)),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe \
            fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// The module a spec loads from a file the project names keeps the names it lists in `__all__` a read uses.
#[test]
fn spec_loaded_module_read_keeps_name_listed() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\"]\n\n\nclient = object()\n"),
        ("tests/test_docs.py", &spec_loading_test("Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, common::native(common::PASSED));
}

// A test module that loads `docs/tutorial.py` from `path` and reads its names off the module, once directly and once
// through a variable.
fn spec_loading_test(path: &str) -> String {
    format!(
        "import importlib.util\nimport os\nimport pathlib\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    \
         spec = importlib.util.spec_from_file_location(\"tutorial\", {path})\n    assert spec is not None\n    module \
         = \
         importlib.util.module_from_spec(spec)\n    assert spec.loader is not None\n    \
         spec.loader.exec_module(module)\n    \
         alias = module\n    assert alias.client\n"
    )
}

// A module `runpy` runs keeps the names a string key reads off its globals, however the call spells the module.
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    assert runpy.run_module(\"docs.tutorial\")[\"client\"]\n" ;
    "run_module_subscript"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\", run_name=\"x\")\n    \
    assert ns.get(\"client\")\n" ;
    "run_module_get"
)]
#[test_case(
    "import runpy\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    ns = \
    runpy.run_path(str(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"))\n    assert ns[\"client\"]\n" ;
    "run_path_computed"
)]
#[test_case(
    "import runpy\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    assert \
    runpy.run_path(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\")[\"client\"]\n" ;
    "run_path_static"
)]
#[test_case(
    "import runpy\n\n\ndef test_app(name: str) -> None:\n    assert runpy.run_module(name)[\"client\"]\n" ;
    "run_module_computed"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\")\n    ns = {}\n    assert \
    ns[\"client\"]\n" ;
    "rebound_to_dict"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\")\n    ns = \
    runpy.run_module(\"docs.other\")\n    assert ns[\"client\"]\n" ;
    "rebound_to_other_globals"
)]
fn runpy_globals_keep_names_read_by_key(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = []\n\n\ndef client() -> None: ...\n\n\ndef engine() -> None: ...\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403"]).1,
        common::native(
            "docs/tutorial.py:7:5: SMP403 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe \
            fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A computed read a run cannot resolve may read any name of a module, so the names `__all__` lists keep their place
// only when nothing reads them.
#[test]
fn computed_run_globals_read_keeps_listed_names() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        (
            "tests/test_docs.py",
            "import runpy\n\n\ndef test_app(name: str) -> None:\n    assert runpy.run_path(name)[\"client\"]\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports it; list only \
            what \
             other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A test that patches a module attribute by its dotted path, or through the module object, uses the name, so `__all__`
// keeps it.
#[test_case("import unittest.mock\n\n\ndef test_app() -> None:\n    with \
    unittest.mock.patch(\"docs.tutorial.client\"):\n        pass\n" ; "mock_patch_string")]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(\"docs.tutorial.client\", \
    None)\n" ; "monkeypatch_string")]
#[test_case("from docs import tutorial\n\n\ndef test_app(mocker) -> None:\n    mocker.patch.object(tutorial, \
    \"client\")\n" ; "patch_object_module")]
#[test_case("from docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    monkeypatch.setattr(tutorial, \
    \"client\", None)\n" ; "setattr_module")]
fn patched_names_are_used_by_all(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports it; list only \
            what \
             other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}
