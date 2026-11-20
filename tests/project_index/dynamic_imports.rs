use super::*;
use crate::common::{check, project};
use test_case::test_case;

#[test_case(
    concat!(
        "import importlib\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    assert importlib.import_module(\"docs.tutorial\").client\n"
    ) ; "import_module"
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
    concat!(
        "import pytest\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    mod = pytest.importorskip(\"docs.tutorial\")\n",
        "    assert mod.client\n"
    ) ; "importorskip_variable"
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
             fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

#[test]
fn variable_rebound_to_another_value_reads_names_of_any_module() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        (
            "tests/test_docs.py",
            "import pytest\n\n\ndef test_app(other) -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    \
             mod = other\n    assert mod.client\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe \
             fix available with `--fix --unsafe-fixes`.\n"
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
    common::assert_clean(root.path(), "SMP404");
    let unread = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\n"),
        ("docs/other.py", "engine = object()\n"),
        (
            "tests/test_docs.py",
            concat!(
                "import pytest\n",
                "\n",
                "\n",
                "def test_engine() -> None:\n",
                "    mod = pytest.importorskip(\"docs.other\")\n",
                "    assert mod.engine\n"
            ),
        ),
    ]);
    assert!(check(unread.path(), &["--select", "SMP404"]).1.contains("`client`"));
}

// Without a `fromlist`, `__import__("pkg.mod")` returns `pkg`.
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

#[test]
fn local_named_like_a_module_variable_reads_no_module() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("scripts/run.py", "import sys\n\nif __name__ == \"__main__\":\n    url = sys.argv[1]\n    print(url)\n"),
        (
            "tests/test_proxy.py",
            "import pytest\n\nproxy = pytest.importorskip(\"proxy\")\n\n\ndef test_one(make_proxy) -> None:\n    proxy \
             = make_proxy()\n    assert proxy.url\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP102"]).1.matches("scripts/run.py").count(), 0);
}

// A local can refer to several modules on different branches.
#[test_case(
    concat!(
        "    m = pytest.importorskip(\"pkg.cli\")\n",
        "    assert m.client\n",
        "    m = pytest.importorskip(\"pkg.other\")\n",
        "    assert m.engine\n"
    ),
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
    assert_eq!(
        (
            stdout.contains(&common::native("pkg/third.py:3:1: SMP404 `client`")),
            stdout.contains(&common::native("pkg/other.py:3:1: SMP404 `engine`"))
        ),
        (true, other_reported),
        "{stdout}"
    );
}

// A local bound to an imported module's attribute may hold an ordinary object.
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
             assert app\n\n\ndef test_two() -> None:\n    app = object()\n    assert app.engine\n",
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

// A nonempty `fromlist` makes `__import__("pkg.mod", ...)` return the submodule. An unknown list may be nonempty.
#[test_case(
    "__import__(\"pkg.sub\", globals(), locals(), [], 0)",
    "pkg/sub.py:3:1: SMP404 `helper`",
    "pkg/__init__.py:3:1" ; "positional_empty_list"
)]
#[test_case(
    "__import__(\"pkg.sub\", fromlist=())",
    "pkg/sub.py:3:1: SMP404 `helper`",
    "pkg/__init__.py:3:1" ; "empty_tuple"
)]
#[test_case(
    "__import__(\"pkg.sub\", fromlist=None)",
    "pkg/sub.py:3:1: SMP404 `helper`",
    "pkg/__init__.py:3:1" ; "none"
)]
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
    assert!(stdout.contains(&common::native(reported)) && !stdout.contains(&common::native(unreported)), "{stdout}");
}

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

#[test_case("Path(os.environ[\"DOCS\"]) / \"tutorial.py\"" ; "computed_path")]
#[test_case("DIRS[0] / \"tutorial.py\"" ; "subscript_of_name")]
#[test_case("Path(__file__).name / \"tutorial.py\"" ; "attribute_other_than_parent")]
#[test_case("\"/elsewhere/docs/tutorial.py\"" ; "outside_project")]
#[test_case("\"docs/tutorial.txt\"" ; "not_python_file")]
#[test_case("os.getcwd()" ; "unknown_call")]
#[test_case("location=\"docs/tutorial.py\"" ; "location_keyword")]
#[test_case("Path(__file__).parent / os.environ[\"DOCS\"]" ; "computed_right_operand")]
#[test_case("os.getcwd().parent / \"tutorial.py\"" ; "parent_of_unknown_call")]
#[test_case("Path(__file__).parents[DEPTH] / \"tutorial.py\"" ; "parents_by_name")]
#[test_case("Path(__file__).parents[1.5] / \"tutorial.py\"" ; "parents_by_float")]
#[test_case("Path(__file__).parents[99999999999999999999999] / \"tutorial.py\"" ; "parents_past_any_index")]
#[test_case("Path(__file__).parents[18446744073709551615] / \"tutorial.py\"" ; "parents_at_the_largest_index")]
#[test_case("os.getcwd().parents[0] / \"tutorial.py\"" ; "parents_of_unknown_call")]
#[test_case("os.path.join(os.path.dirname(os.getcwd()), \"tutorial.py\")" ; "dirname_of_unknown_call")]
#[test_case("Path(__file__).parents[1]" ; "project_root")]
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
             fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

#[test]
fn spec_loaded_module_read_keeps_name_listed() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\"]\n\n\nclient = object()\n"),
        ("tests/test_docs.py", &spec_loading_test("Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"")),
    ]);
    common::assert_clean(root.path(), "SMP402");
    let unread = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\"]\n\n\nclient = object()\n"),
        ("tests/test_docs.py", "def test_app() -> None:\n    pass\n"),
    ]);
    assert!(check(unread.path(), &["--select", "SMP402"]).1.contains("`client` is in `__all__`"));
}

fn spec_loading_test(path: &str) -> String {
    format!(
        "import importlib.util\nimport os\nimport pathlib\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    \
         spec = importlib.util.spec_from_file_location(\"tutorial\", {path})\n    assert spec is not None\n    module \
         = importlib.util.module_from_spec(spec)\n    assert spec.loader is not None\n    \
         spec.loader.exec_module(module)\n    \
         alias = module\n    assert alias.client\n"
    )
}

// `runpy` returns a globals dictionary. A lookup by key can be the only use of a module-level name.
#[test_case(
    concat!(
        "import runpy\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    assert runpy.run_module(\"docs.tutorial\")[\"client\"]\n"
    ) ; "run_module_subscript"
)]
#[test_case(
    concat!(
        "import runpy\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    ns = runpy.run_module(\"docs.tutorial\", run_name=\"x\")\n",
        "    assert ns.get(\"client\")\n"
    ) ; "run_module_get"
)]
#[test_case(
    concat!(
        "import runpy\n",
        "from pathlib import Path\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    ns = runpy.run_path(str(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"))\n",
        "    assert ns[\"client\"]\n"
    ) ; "run_path_computed"
)]
#[test_case(
    concat!(
        "import runpy\n",
        "from pathlib import Path\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    assert runpy.run_path(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\")[\"client\"]\n"
    ) ; "run_path_static"
)]
#[test_case(
    "import runpy\n\n\ndef test_app(name: str) -> None:\n    assert runpy.run_module(name)[\"client\"]\n" ;
    "run_module_computed"
)]
#[test_case(
    concat!(
        "import runpy\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    ns = runpy.run_module(\"docs.tutorial\")\n",
        "    ns = {}\n",
        "    assert ns[\"client\"]\n"
    ) ; "rebound_to_dict"
)]
#[test_case(
    concat!(
        "import runpy\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    ns = runpy.run_module(\"docs.tutorial\")\n",
        "    ns = runpy.run_module(\"docs.other\")\n",
        "    assert ns[\"client\"]\n"
    ) ; "rebound_to_other_globals"
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
             fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// The path is unknown but the key is literal. Only `client` might be read.
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
             what other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with unittest.mock.patch(\"docs.tutorial.client\"):\n",
        "        pass\n"
    ) ; "mock_patch_string"
)]
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(\"docs.tutorial.client\", None)\n"
    ) ; "monkeypatch_string"
)]
#[test_case(
    concat!(
        "from docs import tutorial\n",
        "\n",
        "\n",
        "def test_app(mocker) -> None:\n",
        "    mocker.patch.object(tutorial, \"client\")\n"
    ) ; "patch_object_module"
)]
#[test_case(
    concat!(
        "from docs import tutorial\n",
        "\n",
        "\n",
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(tutorial, \"client\", None)\n"
    ) ; "setattr_module"
)]
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
             what other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}
