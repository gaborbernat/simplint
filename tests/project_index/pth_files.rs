use super::*;
use crate::common::{check, native, project};
use test_case::test_case;

// A source root under `src` holds the `.pth` file the interpreter reads, as a project laid out with `src/` does.
#[test]
fn startup_pth_under_src_keeps_the_name() {
    let root = project(&[
        ("src/pkg.pth", "import pkg.hook; pkg.hook._run()\n"),
        ("src/pkg/__init__.py", ""),
        ("src/pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, common::PASSED);
    let without_pth = project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    assert!(check(without_pth.path(), &["--select", "SMP408"]).1.contains("`_run` is used only by tests"));
}

// A production module that passes another module on as a value calls that module's functions through the value, which
// no import of a name shows, so they are not used only by tests. Only such a load counts: a module that production only
// imports still reports the functions the tests alone call.
#[test_case(
    "from pkg import fast\n\nCODEC = fast if CHOSEN else None\n",
    common::PASSED ; "conditional_alias"
)]
#[test_case(
    "from pkg import fast\n\n\ndef parse(text, backend=fast):\n    return backend.decode(text)\n",
    common::PASSED ; "default_argument"
)]
#[test_case(
    "from pkg import fast\n\n\nclass Service:\n    def __init__(self):\n        self.backend = fast\n",
    common::PASSED ; "instance_attribute"
)]
#[test_case("from pkg import fast\n\nBACKENDS = {\"fast\": fast}\n", common::PASSED ; "dict_value")]
#[test_case(
    "from pkg import fast\n",
    concat!(
        "pkg/fast.py:1:5: SMP408 `decode` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "import_alone_is_not_a_value"
)]
fn module_loaded_as_value_has_no_test_only_names(service: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/fast.py", "def decode(text: str) -> str:\n    return text\n"),
        ("pkg/service.py", service),
        (
            "tests/test_fast.py",
            "from pkg import fast\n\n\ndef test_decode() -> None:\n    assert fast.decode(\"1\") == \"1\"\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, stdout.replace("pkg/", &native("pkg/")));
}

// A `.pth` file that is not UTF-8 is skipped with a warning, as an unreadable Python file is, so the names it would
// read are reported and the run goes on.
#[test]
fn non_utf8_startup_pth_is_skipped_with_a_warning() {
    let root = project(&[
        ("pkg.pth", ""),
        ("pkg/__init__.py", ""),
        ("pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    std::fs::write(root.path().join("pkg.pth"), b"import pkg.hook; pkg.hook._run()\xff\n").expect("write file");
    let (code, stdout, stderr) = check(root.path(), &["--select", "SMP408"]);
    assert_eq!(
        (code, stdout.contains("SMP408"), stderr.contains("Failed to read") && stderr.contains("pkg.pth")),
        (1, true, true)
    );
}
