use super::*;
use crate::common::{check, project};
use test_case::test_case;

// A production module reaches another as a value through a dotted name, or through an import in a function or class
// body, so a call through the value reaches the module's functions with no import of them. Only the import alone is not
// a value.
#[test_case("import pkg.fast\n\nCODEC = pkg.fast\n", common::PASSED ; "dotted_name_loaded")]
#[test_case(
    "def codec():\n    from pkg import fast\n\n    return fast\n",
    common::PASSED ; "import_in_function"
)]
#[test_case(
    "class Service:\n    from pkg import fast as codec\n\n\ndef run(text: str) -> str:\n    return Service.codec.decode(text)\n",
    common::PASSED ;
    "import_in_class_read_as_an_attribute"
)]
#[test_case(
    "import pkg.fast\n",
    "pkg/fast.py:1:5: SMP408 `decode` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ;
    "dotted_import_alone"
)]
#[test_case(
    "class Service:\n    import pkg.fast\n\n\ndef run(text: str) -> str:\n    return Service.pkg.fast.decode(text)\n",
    common::PASSED ;
    "dotted_import_in_class_read_through_the_class"
)]
#[test_case(
    "def run(text: str) -> str:\n    from pkg import fast\n\n    return str(fast.OTHER)\n\n\ndef other(fast: object) -> object:\n    return fast\n",
    "pkg/fast.py:1:5: SMP408 `decode` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ;
    "function_attribute_while_another_parameter_is_named_the_module"
)]
fn module_loaded_through_a_nested_or_dotted_name_has_no_test_only_names(service: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/fast.py", "def decode(text: str) -> str:\n    return text\n"),
        ("pkg/service.py", service),
        (
            "tests/test_fast.py",
            "from pkg import fast\n\n\ndef test_decode() -> None:\n    assert fast.decode(\"1\") == \"1\"\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP408"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A function that imports a module and only calls one of its functions loads no name as a value, so the module's other
// functions that only tests use are still reported.
#[test]
fn import_only_called_through_keeps_other_test_only_names() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/fast.py",
            "def decode(text: str) -> str:\n    return text\n\n\ndef encode(text: str) -> str:\n    return text\n",
        ),
        ("pkg/service.py", "def run(text: str) -> str:\n    from pkg import fast\n\n    return fast.decode(text)\n"),
        (
            "tests/test_fast.py",
            "from pkg import fast\n\n\ndef test_encode() -> None:\n    assert fast.encode(\"1\") == \"1\"\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP408"]).1,
        "pkg/fast.py:5:5: SMP408 `encode` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n"
            .replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

const MAIL: &str = "def send(address: str, urgent: bool = False) -> str:\n    return f\"{address}:{urgent}\"\n";

const PLAIN: &str = "from app.mail import send\n\n\ndef notify(address: str) -> str:\n    return send(address)\n";

const SERVICE: &str =
    "from app.mail import send\n\n\ndef escalate(address: str) -> str:\n    return send(address, urgent=True)\n";
const PROGRAM_SERVICE: &str = "from app.mail import send\n\n\ndef escalate(address: str) -> str:\n    return send(address, urgent=True)\n\n\nif __name__ == \"__main__\":\n    print(escalate(\"x\"))\n";

// A module a star import or a `__main__` guard runs is production code, so its calls to `send` are production calls, not
// the calls of a test that imports it.
#[test_case(
    "from app.plain import notify\nfrom app.service import *\n\nprint(notify(\"x\"), escalate(\"x\"))\n",
    SERVICE,
    common::PASSED ;
    "star_imported_by_program"
)]
#[test_case("from app.plain import notify\n\nprint(notify(\"x\"))\n", PROGRAM_SERVICE, common::PASSED ; "guard_runs_as_program")]
#[test_case(
    "from app.plain import notify\n\nprint(notify(\"x\"))\n",
    SERVICE,
    "app/mail.py:1:24: SMP410 Only tests pass `urgent` to `send`; production calls leave it at its default\nFound 1 error.\n" ;
    "imported_only_by_tests"
)]
fn module_run_as_a_program_makes_production_calls(main: &str, service: &str, stdout: &str) {
    let root = project(&[
        ("app/__init__.py", ""),
        ("app/__main__.py", main),
        ("app/mail.py", MAIL),
        ("app/plain.py", PLAIN),
        ("app/service.py", service),
        (
            "tests/test_service.py",
            "from app.service import escalate\n\n\ndef test_escalate() -> None:\n    assert escalate(\"a\") == \"a:True\"\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP410"]).1,
        stdout.replace("app/", &format!("app{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// The value load only hides the module's test-only names from SMP408. SMP401 still lists a name that production imports.
#[test_case("from pkg import fast\n\nCODEC = fast\n", common::PASSED ; "module_loaded_as_value")]
#[test_case(
    "from pkg.fast import LIMIT\n\nCODEC = LIMIT\n",
    "pkg/fast.py:1:1: SMP401 `LIMIT` is imported by other modules; list it in `__all__`\nFound 1 error.\n1 unsafe fix \
     available with `--fix --unsafe-fixes`.\n" ;
    "imported_by_production"
)]
fn module_loaded_as_value_has_no_test_only_names_for_all(service: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/fast.py", "LIMIT = 1\nMODE = 2\n"),
        ("pkg/service.py", service),
        ("tests/test_fast.py", "from pkg.fast import LIMIT\n\n\ndef test_limit() -> None:\n    assert LIMIT == 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A version check that holds on Python 3 leaves its else branch unreachable, a `TYPE_CHECKING` bound to a true constant
// guards nothing, and a star import rebinds a flag bound false.
#[test_case("import sys\n\nif sys.version_info[0] == 3:\n    pass\nelse:\n    from a import VALUE\n\nSCALE = 2\n", false ; "else_of_three_never_runs")]
#[test_case("import sys\n\nif sys.version_info[0] == 3:\n    from a import VALUE\n\nSCALE = 2\n", true ; "body_of_three_runs")]
#[test_case("import sys\n\nif sys.version_info[0] == 2:\n    from a import VALUE\n\nSCALE = 2\n", false ; "body_of_two_never_runs")]
#[test_case("import sys\n\nif sys.version_info[0] == 2:\n    pass\nelse:\n    from a import VALUE\n\nSCALE = 2\n", true ; "else_of_two_runs")]
#[test_case("from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from a import VALUE\n\nSCALE = 2\n", false ; "typing_guard")]
#[test_case("TYPE_CHECKING = True\n\nif TYPE_CHECKING:\n    from a import VALUE\n\nSCALE = 2\n", true ; "true_constant_is_no_guard")]
#[test_case("FLAG = False\n\nif FLAG:\n    from a import VALUE\n\nSCALE = 2\n", false ; "false_constant_guard")]
#[test_case("FLAG = False\nfrom flags import *\n\nif FLAG:\n    from a import VALUE\n\nSCALE = 2\n", true ; "star_rebinds_flag")]
#[test_case("_FLAG = False\nfrom flags import *\n\nif _FLAG:\n    from a import VALUE\n\nSCALE = 2\n", false ; "star_keeps_private_flag")]
fn runtime_guards_follow_versions_and_constants(b: &str, reported: bool) {
    let root = project(&[("a.py", "import b\n\nVALUE = b.SCALE + 1\n"), ("b.py", b), ("flags.py", "FLAG = True\n")]);
    let report = check(root.path(), &["--select", "SMP308"]).1;
    assert_eq!(report.contains("SMP308"), reported, "{report}");
}

// A flag bound false at module level keeps the imports under it out of the cycle check, unless some binding of it in the
// module's blocks, or a `global` declaration of it, may make it true. A binding inside a function is local to it.
#[test_case("", false ; "unrebound")]
#[test_case("def enable() -> None:\n    HAS_JSON = True\n\n", false ; "local_in_function")]
#[test_case("try:\n    import json\n    HAS_JSON = True\nexcept ImportError:\n    pass\n", true ; "try_body")]
#[test_case("try:\n    import json\nexcept ImportError:\n    HAS_JSON = True\n", true ; "except_handler")]
#[test_case("try:\n    pass\nfinally:\n    HAS_JSON = True\n", true ; "finally_block")]
#[test_case("if SCALE:\n    HAS_JSON = True\n", true ; "if_block")]
#[test_case("if SCALE > 5:\n    pass\nelse:\n    HAS_JSON = True\n", true ; "else_block")]
#[test_case("while SCALE:\n    break\nelse:\n    HAS_JSON = True\n", true ; "while_else")]
#[test_case("for HAS_JSON in [True]:\n    pass\n", true ; "for_target")]
#[test_case("with open(__file__) as HAS_JSON:\n    pass\n", true ; "with_target")]
#[test_case("try:\n    pass\nexcept ValueError as HAS_JSON:\n    pass\n", true ; "except_name")]
#[test_case("match SCALE:\n    case HAS_JSON:\n        pass\n", true ; "match_capture")]
#[test_case("match SCALE:\n    case [*HAS_JSON]:\n        pass\n", true ; "match_star")]
#[test_case("match SCALE:\n    case {**HAS_JSON}:\n        pass\n", true ; "match_mapping_rest")]
#[test_case("if (HAS_JSON := True):\n    pass\n", true ; "walrus")]
#[test_case("if SCALE:\n    HAS_JSON: bool = True\n", true ; "annotated_assignment")]
#[test_case("if SCALE:\n    HAS_JSON += 1\n", true ; "augmented_assignment")]
#[test_case("HAS_JSON, OTHER = True, 1\n", true ; "tuple_target")]
#[test_case("import json as HAS_JSON\n", true ; "import_alias")]
#[test_case("from json import dumps as HAS_JSON\n", true ; "from_import_alias")]
#[test_case("def HAS_JSON() -> None:\n    pass\n", true ; "function_name")]
#[test_case("class HAS_JSON:\n    pass\n", true ; "class_name")]
#[test_case("def enable() -> None:\n    global HAS_JSON\n    HAS_JSON = True\n", true ; "global_declaration")]
fn module_flag_rebound_in_a_block_keeps_its_imports(rebinding: &str, reported: bool) {
    let root = project(&[
        ("a.py", "import b\n\nVALUE = b.SCALE + 1\n"),
        ("b.py", &format!("HAS_JSON = False\n{rebinding}\nif HAS_JSON:\n    from a import VALUE\n\nSCALE = 2\n")),
    ]);
    let report = check(root.path(), &["--select", "SMP308"]).1;
    assert_eq!(report.contains("SMP308"), reported, "{report}");
}

// The names a `.pth` file reads are used like an entry point's, so no rule that asks whether a name is used, listed or
// unlisted reports them.
#[test_case("__all__ = [\"_run\"]\n\n\n", "_run", "SMP402" ; "unused_export")]
#[test_case("", "_run", "SMP409" ; "unused_definition")]
#[test_case("__all__ = []\n\n\n", "run", "SMP403" ; "unexported_public_name")]
fn startup_pth_reads_are_not_unused(prefix: &str, name: &str, rule: &str) {
    let root = project(&[
        ("pkg.pth", &format!("import pkg.hook; pkg.hook.{name}()\n")),
        ("pkg/__init__.py", ""),
        ("pkg/hook.py", &format!("{prefix}def {name}() -> None:\n    print(\"hook\")\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", rule]).1, common::PASSED);
}
