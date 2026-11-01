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

// Patching an attribute of the module's own import reaches `os` through `files` without importing it from there.
#[test_case("mocker.patch(\"pkg.files.os.getcwd\")", "All checks passed!\n" ; "patch_of_an_attribute")]
#[test_case(
    "from pkg.files import os",
    "pkg/files.py:1:8: SMP113 `os` is imported by other modules; list it in `__all__`\nFound 1 error.\n1 \
     unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "import_of_the_name"
)]
fn patch_target_through_module_import_is_not_an_export(reference: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/files.py", "import os\n\n\ndef cwd() -> str:\n    return os.getcwd()\n"),
        ("test_files.py", &format!("def test_cwd(mocker):\n    {reference}\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP113"]).1, common::native(stdout));
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
