mod common;

use test_case::test_case;

use common::{check, project};

#[test]
fn import_through_directory_without_init_counts_as_imported() {
    let root = project(&[
        ("support/marks.py", "def needs_network() -> None:\n    pass\n"),
        ("client.py", "from support.marks import needs_network\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP113"]).1,
        common::native(
            "support/marks.py:1:5: SMP113 `needs_network` is imported by other modules; list it in `__all__`\nFound 1 \
             error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

#[test]
fn package_outside_the_source_roots_resolves_from_its_parent() {
    let root = project(&[
        ("lib/pkg/__init__.py", ""),
        ("lib/pkg/helpers.py", "def helper() -> None:\n    pass\n"),
        ("lib/pkg/use.py", "from pkg.helpers import helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP113"]).1.lines().next(),
        Some(
            common::native(
                "lib/pkg/helpers.py:1:5: SMP113 `helper` is imported by other modules; list it in `__all__`"
            )
            .as_str()
        )
    );
}

#[test]
fn src_layout_imports_resolve() {
    let root = project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from pkg.helpers import helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP113"]).1.lines().next(),
        Some(
            common::native(
                "src/pkg/helpers.py:1:5: SMP113 `helper` is imported by other modules; list it in `__all__`"
            )
            .as_str()
        )
    );
}

// A stub for another module declares nothing about `backend.py`.
#[test_case("backend.pyi", "All checks passed!\n" ; "stub_of_the_module")]
#[test_case(
    "other.pyi",
    "backend.py:2:1: SMP102 Module-scope variable `LIMIT` should be named `_LIMIT`\nbackend.py:3:1: SMP102 \
     Module-scope variable `NAME` should be named `_NAME`\nbackend.py:10:7: SMP103 `Missing` is not in `__all__`; \
     name it `_Missing`\nFound 3 errors.\n3 unsafe fixes available with `--fix --unsafe-fixes`.\n" ;
    "stub_of_another_module"
)]
fn stub_declarations_are_public_api(stub: &str, stdout: &str) {
    let root = project(&[
        (
            "backend.py",
            "__all__ = [\"run\"]\nLIMIT = 1\nNAME: str = \"x\"\n\n\ndef run() -> None:\n    pass\n\n\nclass Missing(TypeError):\n    pass\n",
        ),
        (stub, "import os\nLIMIT: int\nNAME = ...\ndef run() -> None: ...\nclass Missing(TypeError): ...\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP102,SMP103"]).1, stdout);
}

// Patching an attribute of the module's own import reaches `os` through `files` without importing it from there, and
// importing it from there borrows a name `files` uses rather than one it exports.
#[test_case("mocker.patch(\"pkg.files.os.getcwd\")" ; "patch_of_an_attribute")]
#[test_case("from pkg.files import os" ; "import_of_the_name")]
fn module_import_is_not_an_export(reference: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/files.py", "import os\n\n\ndef cwd() -> str:\n    return os.getcwd()\n"),
        ("test_files.py", &format!("def test_cwd(mocker):\n    {reference}\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP113"]).1, "All checks passed!\n");
}

#[test_case("pkg.manager._load", "All checks passed!\n" ; "patched")]
#[test_case(
    "pkg.manager.run",
    "pkg/manager.py:1:5: SMP030 `_load` only forwards to `import_module`; call `import_module` at its call sites \
     instead\nFound 1 error.\n" ;
    "not_patched"
)]
fn patched_forwarding_function_is_a_seam(target: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/manager.py",
            "def _load(name: str) -> str:\n    return import_module(name)\n\n\ndef run() -> str:\n    return _load(\"x\")\n",
        ),
        ("test_manager.py", &format!("def test_run(mocker):\n    mocker.patch(\"{target}\")\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP030"]).1, common::native(stdout));
}

// `import *` takes the names `__all__` lists, or without one every public name, so those may be in use unnamed.
#[test_case("", "All checks passed!\n" ; "every_public_name")]
#[test_case(
    "__all__ = [\"run\"]\n",
    "pkg/data.py:2:1: SMP102 Module-scope variable `TAG` should be named `_TAG`\nFound 1 error.\n1 unsafe fix available \
     with `--fix --unsafe-fixes`.\n" ;
    "listed_names"
)]
fn star_import_reaches_public_names(all: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/data.py", &format!("{all}TAG = {{\"a\": 1}}\n\n\ndef run() -> None:\n    pass\n")),
        ("pkg/user.py", "from pkg.data import *\n\n\ndef get() -> dict:\n    return TAG\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP102"]).1, common::native(stdout));
}

// A name imported from a package counts in the module its `__init__.py` star-imports it from, where an `__all__` would
// narrow what the package re-exports.
#[test]
fn star_reexport_keeps_names_of_the_package() {
    let types = "class ResourceType:\n    pass\n\n\ndef parse() -> None:\n    pass\n";
    let root = project(&[
        ("lib/__init__.py", ""),
        ("lib/humio/__init__.py", "from ._types import *\n"),
        ("lib/humio/_types.py", types),
        ("app.py", "from lib import humio\nfrom lib.humio import parse\n\n_TYPES = (humio.ResourceType, parse)\n"),
    ]);
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102,SMP103,SMP113"]);
    assert_eq!(
        (stdout, common::read(root.path(), "lib/humio/_types.py")),
        (
            common::native(
                "lib/humio/_types.py:1:7: SMP113 `ResourceType` is imported by other modules; list it in `__all__`\n\
                 lib/humio/_types.py:5:5: SMP113 `parse` is imported by other modules; list it in `__all__`\nFound 2 \
                 errors.\n"
            ),
            types.to_owned()
        )
    );
}

// `oci_test_init` resolves only through a `sys.path` entry a `conftest.py` adds, so any module of the run by that name
// may be the one star-imported and keeps its names, while modules by other names cannot be.
#[test]
fn unresolved_star_import_keeps_names_of_modules_by_its_name() {
    let root = project(&[
        ("support/oci_test_init.py", "region = \"x\"\n"),
        ("support/other.py", "region = \"y\"\n"),
        ("tests/test_oci.py", "from oci_test_init import *\n\n\ndef test_region() -> None:\n    assert region\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP102"]).1,
        common::native(
            "support/oci_test_init.py:1:1: SMP102 Module-scope variable `region` should be named `REGION`\n\
             support/other.py:1:1: SMP102 Module-scope variable `region` should be named `_REGION`\nFound 2 errors.\n1 \
             unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}
