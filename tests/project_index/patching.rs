use super::*;
use crate::common::{check, project};
use test_case::test_case;

// A test that names the attribute it sets, deletes or patches with a loop over string literals uses each of them, as a
// literal name does.
#[test_case(
    concat!(
        "for name in (\"client\", \"engine\"):\n",
        "        monkeypatch.setattr(tutorial, name, None)\n"
    ) ; "setattr_loop_over_tuple"
)]
#[test_case(
    concat!(
        "for name, value in ((\"client\", \"a\"), (\"engine\", \"b\")):\n",
        "        monkeypatch.setattr(tutorial, name, value)\n"
    ) ; "setattr_loop_over_pairs"
)]
#[test_case(
    concat!(
        "NAMES = [\"client\", \"engine\"]\n",
        "\n",
        "    for name in NAMES:\n",
        "        monkeypatch.delattr(tutorial, name)\n"
    ) ; "delattr_loop_over_bound_list"
)]
#[test_case(
    concat!(
        "for name in {\"client\", \"engine\"}:\n",
        "        mock.patch.object(tutorial, name).start()\n"
    ) ; "patch_object_loop_over_set"
)]
fn names_read_in_loops_over_string_literals_are_used(body: &str) {
    let test = format!(
        "from unittest import mock\n\nfrom docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    {body}"
    );
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, common::native(common::PASSED));
}

// A loop over anything but string rows of the same width, whose items the run cannot know, names no attribute, and
// neither does a name rebound to something else after it held string literals, or a computed name.
#[test_case("for name in names():\n        monkeypatch.setattr(tutorial, name, None)\n" ; "call")]
#[test_case("for name in dir(tutorial):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "dir")]
#[test_case("for name in (\"client\", 1):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "mixed_items")]
#[test_case(
    concat!(
        "for name, value in ((\"client\",), (\"engine\", \"b\")):\n",
        "        monkeypatch.setattr(tutorial, name, value)\n"
    ) ; "ragged_pairs"
)]
#[test_case(
    concat!(
        "NAMES = [\"client\", \"engine\"]\n",
        "    NAMES = names()\n",
        "    for name in NAMES:\n",
        "        monkeypatch.setattr(tutorial, name, None)\n"
    ) ; "rebound_name"
)]
#[test_case("monkeypatch.setattr(tutorial, names()[0], None)\n" ; "computed_name")]
fn loops_over_unknown_iterables_name_no_attribute(body: &str) {
    let test = format!(
        "from docs import tutorial\n\n\ndef names() -> list[str]:\n    return []\n\n\ndef test_app(monkeypatch) -> \
        None:\n    {body}"
    );
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:12: SMP402 [*] `client` is in `__all__`, yet no other module imports it; list only \
            what \
             other modules use\ndocs/tutorial.py:1:22: SMP402 `engine` is in `__all__`, yet no other module imports \
             it; list only what other modules use\nFound 2 errors.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A `setattr` in a loop over a literal of modules writes the name on each module the items import or name, as a literal
// module does, while an item that names no module is ignored and a module outside the loop still keeps its attribute.
#[test_case("(tutorial, importlib.import_module(\"docs.other\"))" ; "imported_and_import_module")]
#[test_case("(tutorial, importlib.import_module(\"docs.other\"), 1)" ; "item_naming_no_module_ignored")]
fn setattr_loop_over_modules_writes_each_module(items: &str) {
    let test = format!(
        "import importlib\n\nfrom docs import engine, other, tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    \
         for module in {items}:\n        monkeypatch.setattr(module, \"SHARED\", {{}})\n"
    );
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "SHARED = {}\n"),
        ("docs/other.py", "SHARED = {}\n"),
        ("docs/engine.py", "SHARED = {}\n"),
        (
            "docs/app.py",
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, \
            other.SHARED, engine.SHARED\n",
        ),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304,SMP601"]).1,
        common::native(
            "docs/app.py:5:43: SMP304 Import `SHARED` from `docs.engine` instead of reaching it through the module\n\
             docs/engine.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\nFound 2 errors.\n"
        )
    );
}

// A loop that rebinds its module's name, or a function or lambda whose parameter shadows it, patches neither module it
// loops over.
#[test_case(
    concat!(
        "    for module in (tutorial, other):\n",
        "        module = engine\n",
        "        monkeypatch.setattr(module, \"SHARED\", {})\n"
    ) ; "rebound"
)]
#[test_case(
    concat!(
        "    for module in (tutorial, other):\n",
        "        (lambda module: monkeypatch.setattr(module, \"SHARED\", {}))(engine)\n"
    ) ; "shadowed_by_lambda"
)]
#[test_case(
    concat!(
        "    for module in (tutorial, other):\n",
        "        def helper(module: object) -> None:\n",
        "            monkeypatch.setattr(module, \"SHARED\", {})\n",
        "\n",
        "        helper(engine)\n"
    ) ; "shadowed_by_parameter"
)]
fn loops_that_rebind_or_shadow_their_module_patch_neither(body: &str) {
    let test = format!("from docs import engine, other, tutorial\n\n\ndef test_app(monkeypatch) -> None:\n{body}");
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "SHARED = {}\n"),
        ("docs/other.py", "SHARED = {}\n"),
        ("docs/engine.py", "SHARED = {}\n"),
        (
            "docs/app.py",
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, \
            other.SHARED, engine.SHARED\n",
        ),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304,SMP601"]).1,
        common::native(
            "docs/app.py:5:12: SMP304 Import `SHARED` from `docs.tutorial` instead of reaching it through the module\n\
             docs/app.py:5:29: SMP304 Import `SHARED` from `docs.other` instead of reaching it through the module\n\
             docs/app.py:5:43: SMP304 Import `SHARED` from `docs.engine` instead of reaching it through the module\n\
             docs/engine.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\n\
             docs/other.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\n\
             docs/tutorial.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\nFound 6 errors.\n"
        )
    );
}

// An `importlib.import_module` item without a literal module names no module, so the loop writes on the others only.
#[test_case("importlib.import_module()" ; "without_argument")]
#[test_case("importlib.import_module(MODULE)" ; "non_literal_argument")]
fn import_module_without_a_literal_name_writes_no_module(item: &str) {
    let test = format!(
        "import importlib\n\nfrom docs import engine, other, tutorial\n\nMODULE = \"docs.other\"\n\n\ndef \
        test_app(monkeypatch) -> None:\n    for module in (tutorial, {item}):\n        monkeypatch.setattr(module, \
        \"SHARED\", {{}})\n"
    );
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "SHARED = {}\n"),
        ("docs/other.py", "SHARED = {}\n"),
        ("docs/engine.py", "SHARED = {}\n"),
        (
            "docs/app.py",
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, \
            other.SHARED, engine.SHARED\n",
        ),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304,SMP601"]).1,
        common::native(
            "docs/app.py:5:29: SMP304 Import `SHARED` from `docs.other` instead of reaching it through the module\n\
             docs/app.py:5:43: SMP304 Import `SHARED` from `docs.engine` instead of reaching it through the module\n\
             docs/engine.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\n\
             docs/other.py:1:1: SMP601 `SHARED` is never reassigned; annotate it `Final`\n\
             tests/test_docs.py:5:1: SMP601 `MODULE` is never reassigned; annotate it `Final`\nFound 5 errors.\n\
             1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A loop nested in another that binds the same name hands the outer loop's strings back once it ends.
#[test]
fn nested_loops_restore_the_outer_name() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        (
            "tests/test_docs.py",
            "from docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    for name in \
            (\"client\",):\n        for name in (\"engine\",):\n            assert name\n        \
            monkeypatch.setattr(tutorial, name, None)\n",
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

// A constant a string in the run spells, as a dotted path a test patches or a parametrized name, keeps its module-level
// definition, which inlining it into the one function that reads it would bypass.
#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with unittest.mock.patch(\"docs.tutorial.LIMIT\"):\n",
        "        pass\n"
    ) ; "mock_patch_string"
)]
#[test_case(
    concat!(
        "from docs import tutorial\n",
        "\n",
        "\n",
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(tutorial, \"LIMIT\", 5)\n"
    ) ; "setattr_module"
)]
#[test_case(
    concat!(
        "import pytest\n",
        "\n",
        "\n",
        "@pytest.mark.parametrize(\"name\", [\"LIMIT\"])\n",
        "def test_app(name: str) -> None:\n",
        "    assert name\n"
    ) ; "parametrized_name"
)]
fn constants_a_string_spells_are_not_inlined(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        (
            "docs/tutorial.py",
            "__all__ = []\n\nLIMIT: int | None = None\n\n\ndef limit() -> int | None:\n    return LIMIT or 3\n",
        ),
        ("tests/test_docs.py", test),
    ]);
    let fix = ["--select", "SMP202", "--fix", "--unsafe-fixes"];
    assert_eq!(check(root.path(), &fix).1, common::native(common::PASSED));
    assert_eq!(
        common::read(root.path(), "docs/tutorial.py"),
        "__all__ = []\n\nLIMIT: int | None = None\n\n\ndef limit() -> int | None:\n    return LIMIT or 3\n"
    );
}

// A test that sets a module variable through a `MonkeyPatch` writes it, whatever the object is named and however it is
// reached, so the variable is not `Final`.
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    with monkeypatch.context() as scoped:\n",
        "        scoped.setattr(tutorial, \"LIMIT\", 5)\n"
    ) ; "context_target"
)]
#[test_case("def test_app(patches) -> None:\n    patches.setattr(tutorial, \"LIMIT\", 5)\n" ; "fixture_returned")]
#[test_case(
    concat!(
        "def test_app(request) -> None:\n",
        "    request.config.patches.setattr(tutorial, \"LIMIT\", 5)\n"
    ) ; "attribute_receiver"
)]
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(\"docs.tutorial.LIMIT\", 5)\n"
    ) ; "dotted_string"
)]
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    for name, value in ((\"LIMIT\", \"5\"),):\n",
        "        monkeypatch.setattr(tutorial, name, value)\n"
    ) ; "loop_over_string_literals"
)]
fn module_variables_patched_through_a_monkeypatch_are_not_final(test: &str) {
    let test = format!("from docs import tutorial\n\n\n{test}");
    let root = project(&[("docs/__init__.py", ""), ("docs/tutorial.py", "LIMIT = 3\n"), ("tests/test_docs.py", &test)]);
    assert_eq!(check(root.path(), &["--select", "SMP601"]).1, common::native(common::PASSED));
}

// A `setattr` on an object that is not a module of the run writes no module variable, so the variable stays `Final`.
#[test_case(
    concat!(
        "from docs.tutorial import Settings\n",
        "\n",
        "\n",
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(Settings, \"LIMIT\", 5)\n"
    ) ; "class_of_module"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "from docs.tutorial import Settings\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with mock.patch.object(Settings, \"LIMIT\", 5):\n",
        "        pass\n"
    ) ; "class_of_module_patch_object"
)]
#[test_case(
    concat!(
        "def test_app(monkeypatch, settings) -> None:\n",
        "    monkeypatch.setattr(settings, \"LIMIT\", 5)\n"
    ) ; "parameter"
)]
fn setattr_on_a_non_module_writes_no_module_variable(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "LIMIT = 3\n\n\nclass Settings:\n    pass\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP601"]).1,
        common::native(
            "docs/tutorial.py:1:1: SMP601 `LIMIT` is never reassigned; annotate it `Final`\nFound 1 error.\n1 unsafe \
            fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A write sets the name of the module it is made on, so another module's variable of the same name stays `Final`.
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    with monkeypatch.context() as scoped:\n",
        "        scoped.setattr(tutorial, \"LIMIT\", 5)\n"
    ) ; "context_target"
)]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(tutorial, \"LIMIT\", 5)\n" ; "setattr_module")]
#[test_case(
    concat!(
        "def test_app(monkeypatch) -> None:\n",
        "    monkeypatch.setattr(\"docs.tutorial.LIMIT\", 5)\n"
    ) ; "dotted_string"
)]
#[test_case("def test_app(mocker) -> None:\n    mocker.patch.object(tutorial, \"LIMIT\", 5)\n" ; "mocker_patch_object")]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with mock.patch.object(tutorial, \"LIMIT\", 5):\n",
        "        pass\n"
    ) ; "mock_patch_object_context"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "@mock.patch.object(tutorial, \"LIMIT\", 5)\n",
        "def test_app() -> None:\n",
        "    pass\n"
    ) ; "mock_patch_object_decorator"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with mock.patch(\"docs.tutorial.LIMIT\", 5):\n",
        "        pass\n"
    ) ; "mock_patch_dotted_string"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    for name in (\"LIMIT\",):\n",
        "        mock.patch.object(tutorial, name, 5).start()\n"
    ) ; "patch_object_loop"
)]
fn module_write_quiets_only_the_module_it_writes(test: &str) {
    let test = format!("from docs import tutorial\n\n\n{test}");
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "LIMIT = 3\n"),
        ("docs/other.py", "LIMIT = 4\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP601"]).1,
        common::native(
            "docs/other.py:1:1: SMP601 `LIMIT` is never reassigned; annotate it `Final`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A computed write may set any name of the module it reaches, so that module's variables are not `Final`, while another
// module's variable of the same name still is.
#[test_case(
    concat!(
        "def test_app(monkeypatch, name: str) -> None:\n",
        "    monkeypatch.setattr(tutorial, name, 5)\n"
    ) ; "parameter_name"
)]
#[test_case(
    concat!(
        "def names() -> list[str]:\n",
        "    return []\n",
        "\n",
        "\n",
        "def test_app(monkeypatch) -> None:\n",
        "    for name in names():\n",
        "        monkeypatch.setattr(tutorial, name, None)\n"
    ) ; "loop_over_call"
)]
#[test_case(
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_app(name: str) -> None:\n",
        "    with mock.patch.object(tutorial, name, 5):\n",
        "        pass\n"
    ) ; "patch_object_parameter"
)]
fn computed_module_write_quiets_only_the_module_it_reaches(test: &str) {
    let test = format!("from docs import tutorial\n\n\n{test}");
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "LIMIT = 3\n"),
        ("docs/other.py", "LIMIT = 4\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP601"]).1,
        common::native(
            "docs/other.py:1:1: SMP601 `LIMIT` is never reassigned; annotate it `Final`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A docstring that mentions a constant is prose, which neither patches nor looks it up, so the constant still moves
// into the one function that reads it.
#[test]
fn docstring_mention_does_not_keep_a_constant() {
    let root = project(&[(
        "pkg/helpers.py",
        "_LIMIT = 3\n\n\ndef check(value):\n    \"\"\"Compare with the limit.\n\n    The ``_LIMIT`` constant sets the \
        bound.\n    \"\"\"\n    return value < _LIMIT\n",
    )]);
    assert_eq!(
        check(root.path(), &["--select", "SMP202"]).1,
        common::native(
            "pkg/helpers.py:1:1: SMP202 `_LIMIT` is used by one function only; define it there\nFound 1 error.\n1 \
            unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A computed write reaches the module its target names, so the module's variables keep their names. An f-string names a
// module only through the literal dotted text before its first interpolation.
#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app(name: str) -> None:\n",
        "    with unittest.mock.patch(f\"docs.tutorial.{name}\"):\n",
        "        pass\n"
    ),
    false ; "dotted_prefix_names_the_module"
)]
#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app(name: str) -> None:\n",
        "    with unittest.mock.patch(f\"{name}\"):\n",
        "        pass\n"
    ),
    true ; "leading_interpolation_names_no_module"
)]
#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app() -> None:\n",
        "    with unittest.mock.patch(f\"docs.tutorial\"):\n",
        "        pass\n"
    ),
    true ; "no_interpolation_writes_nothing"
)]
#[test_case(
    concat!(
        "import unittest.mock\n",
        "\n",
        "\n",
        "def test_app(name: str) -> None:\n",
        "    with unittest.mock.patch(f\"docs{name}\"):\n",
        "        pass\n"
    ),
    true ; "no_dot_before_interpolation"
)]
fn computed_patch_targets_name_the_module_they_write(test: &str, renamable: bool) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "limit = 3\n\n\ndef show() -> int:\n    return limit\n"),
        ("tests/test_docs.py", test),
    ]);
    let stdout = check(root.path(), &["--unsafe-fixes", "--select", "SMP102"]).1;
    assert_eq!(stdout.contains("SMP102 [*]"), renamable);
}

// The package's `import *` of `backends` would bind the dict named `backends` over the attribute that holds the module.
#[test]
fn name_of_a_module_its_package_star_imports_stays_unlisted() {
    let root = project(&[
        ("pkg/__init__.py", "from pkg.backends import *\n"),
        (
            "pkg/backends.py",
            "__all__ = [\"dispatch\"]\nbackends = {}\ninfo = {}\n\n\ndef dispatch() -> None:\n    pass\n",
        ),
        ("pkg/user.py", "from pkg.backends import backends, info\n\n_ALL = (backends, info)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1,
        common::native(
            "pkg/backends.py:3:1: SMP401 [*] `info` is imported by other modules; list it in `__all__`\nFound 1 \
             error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A package `__init__.py` that Ruff's `lint.exclude` leaves unlinted still star-imports the module.
#[test]
fn package_init_excluded_from_linting_still_star_imports() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nexclude = [\"pkg/__init__.py\"]\n"),
        ("pkg/__init__.py", "from pkg.backends import *\n"),
        ("pkg/backends.py", "__all__ = [\"dispatch\"]\nbackends = {}\n\n\ndef dispatch() -> None:\n    pass\n"),
        ("pkg/user.py", "from pkg.backends import backends\n\n_ALL = (backends,)\n"),
        ("other/__init__.py", ""),
        ("other/backends.py", "__all__ = [\"dispatch\"]\nbackends = {}\n\n\ndef dispatch() -> None:\n    pass\n"),
        ("other/user.py", "from other.backends import backends\n\n_ALL = (backends,)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1,
        common::native(
            "other/backends.py:2:1: SMP401 [*] `backends` is imported by other modules; list it in `__all__`\nFound 1 \
             error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// `pkg.views` is no module, yet reaches `pkg/sub/views.py` through the submodule the package re-exports, so its
// `ViewBase` is used. `pkg.missing` names no module of the package, so an unresolved read may name any module's
// `ViewBase`: `pkg/sub/other.py` still reports its `ViewBase` but offers no rename, and `Unread` is reported with one.
#[test]
fn attribute_path_through_a_reexported_submodule_counts_as_referenced() {
    let module = "__all__ = [\"View\"]\n\n\nclass View:\n    pass\n\n\nclass ViewBase:\n    pass\n";
    let root = project(&[
        ("pkg/__init__.py", "from pkg.sub import *\n"),
        ("pkg/sub/__init__.py", "from pkg.sub import views\n"),
        ("pkg/sub/views.py", module),
        ("pkg/sub/other.py", &format!("{module}\n\nclass Unread:\n    pass\n")),
        ("user.py", "import pkg\n\n_BASES = (pkg.views.ViewBase, pkg.missing.ViewBase)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403"]).1,
        common::native(
            "pkg/sub/other.py:8:7: SMP403 `ViewBase` is not in `__all__`; name it `_ViewBase`\npkg/sub/other.py:12:7: \
             SMP403 `Unread` is not in `__all__`; name it `_Unread`\nFound 2 errors.\n1 unsafe fix available with \
             `--fix --unsafe-fixes`.\n"
        )
    );
}
