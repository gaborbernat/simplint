use super::*;
use crate::common::{check, project};
use test_case::test_case;

// A module no file of the run resolves, such as `distutils.log` for a tree that shims its own `log`, reads the run's
// file of the same last name when the call's value is used, and only then, so the function returning that value is not
// reported.
#[test_case(
    concat!(
        "from distutils import log\n",
        "\n",
        "\n",
        "def run() -> int:\n",
        "    saved = log.set_threshold(1)\n",
        "    return saved\n"
    ),
    false ; "value_used_through_namesake"
)]
#[test_case(
    concat!(
        "from distutils import log\n",
        "\n",
        "\n",
        "def run() -> None:\n",
        "    log.set_threshold(1)\n"
    ),
    true ; "value_dropped_through_namesake"
)]
#[test_case(
    concat!(
        "from elsewhere import tools\n",
        "\n",
        "\n",
        "def run() -> int:\n",
        "    return tools.set_threshold(1)\n"
    ),
    true ; "unrelated_module_reads_nothing"
)]
fn namesake_module_reads_count_only_with_their_value(caller: &str, reported: bool) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/log.py", "def set_threshold(level: int) -> int:\n    return level\n"),
        ("pkg/tidy.py", "from pkg import log\n\n\ndef tidy() -> None:\n    log.set_threshold(2)\n"),
        ("pkg/wheel.py", caller),
    ]);
    let stdout = check(root.path(), &["--select", "SMP411"]).1;
    assert_eq!(stdout.contains(&common::native("pkg/log.py:1:5: SMP411")), reported, "{stdout}");
}

// A doctest in a module runs in the module's globals, so an example that names a class of the module uses it without
// importing it, while a docstring that only describes the class leaves it unused.
#[test_case(
    concat!(
        "    \"\"\"A release.\n",
        "\n",
        "    >>> _Trimmed(\"1.0.0\").release\n",
        "    '1'\n",
        "    \"\"\"\n"
    ),
    false ; "doctest_names_it"
)]
#[test_case("    \"\"\"A release.\"\"\"\n", true ; "docstring_only_describes_it")]
fn class_named_by_its_own_module_doctest_is_used(docstring: &str, reported: bool) {
    let source = format!(
        "class _Trimmed:\n{docstring}\n    def __init__(self, release: str) -> None:\n        self.release = \
        release.split(\".\")[0]\n"
    );
    let root = project(&[("pkg/version.py", source.as_str())]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains(&common::native("pkg/version.py:1:7: SMP409 `_Trimmed`")), reported, "{stdout}");
}

// A doctest reads a name off a module, which the call-site rules must see as a use, yet it imports nothing for the
// other modules: a function only a doctest calls is not reported as unread, and a name a doctest reads is not imported
// by the others.
#[test_case(">>> q.calc.ratio(3)\n1.5", "" ; "doctest_reads_ratio")]
#[test_case(">>> q.calc.total(3)\n3", "`ratio`" ; "doctest_reads_total")]
fn doctest_reads_are_opaque_to_call_sites_but_not_imports(read: &str, unused: &str) {
    let root = project(&[
        ("q/__init__.py", ""),
        (
            "q/calc.py",
            "def total(value):\n    ratio(value)\n    return value\n\n\ndef ratio(value):\n    return value / 2\n",
        ),
        ("q/guide.py", &format!("\"\"\"Guide.\n\n>>> import q.calc\n{read}\n\"\"\"\n")),
        ("q/main.py", "from q.calc import total\n\nprint(total(1))\n"),
    ]);
    let unread = check(root.path(), &["--select", "SMP411"]).1;
    assert_eq!((unread == common::PASSED, unread.contains(unused)), (unused.is_empty(), true), "{unread}");
    let stdout = check(root.path(), &["--select", "SMP401"]).1;
    assert!(stdout.contains("`total` is imported by other modules"), "{stdout}");
    assert!(!stdout.contains("`ratio`"), "{stdout}");
}

// A name a doctest reads off the module of another file counts as used there, though the doctest imports no name of it.
#[test]
fn doctest_module_reads_leave_the_definition_public_and_used() {
    let root = project(&[
        ("p/__init__.py", ""),
        (
            "p/util.py",
            "__all__ = [\"other\"]\n\n\ndef other():\n    return helper()\n\n\ndef helper():\n    return 1\n",
        ),
        ("p/doc.py", "\"\"\"Docs.\n\n>>> import p.util\n>>> p.util.helper()\n1\n\"\"\"\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP409"]).1, common::PASSED);
    let without_doc = project(&[
        ("p/__init__.py", ""),
        (
            "p/util.py",
            "__all__ = [\"other\"]\n\n\ndef other():\n    return helper()\n\n\ndef helper():\n    return 1\n",
        ),
    ]);
    assert!(check(without_doc.path(), &["--select", "SMP402"]).1.contains("`other`"));
}

// A constant a doctest of its module reads is kept, as inlining it would leave the example reading a name that is gone.
#[test]
fn constant_read_by_its_own_module_doctest_is_kept() {
    let root = project(&[(
        "pkg/consts.py",
        "_TIMEOUT = 30\n\n\ndef wait():\n    \"\"\"Wait.\n\n    >>> _TIMEOUT\n    30\n    \"\"\"\n    return \
        _TIMEOUT\n",
    )]);
    assert_eq!(check(root.path(), &["--select", "SMP202"]).1, common::PASSED);
    let unread = project(&[("pkg/consts.py", "_TIMEOUT = 30\n\n\ndef wait():\n    return _TIMEOUT\n")]);
    assert!(check(unread.path(), &["--select", "SMP202"]).1.contains("`_TIMEOUT` is used by one function only"));
}

// The stdlib's `json` is a module of its own, so a `json.loads` value a file of the run reads is not a read of the
// run's `codec/json.py`, and a `json.dumps` call with its own `indent` is not a call of the run's `dumps`.
#[test]
fn stdlib_module_reads_are_not_namesake_reads() {
    let root = project(&[
        ("app/__init__.py", ""),
        ("app/codec/__init__.py", ""),
        (
            "app/codec/json.py",
            "def loads(text):\n    return text.strip()\n\n\ndef dumps(obj, indent=None):\n    return \
            f\"{obj}{indent}\"\n",
        ),
        ("app/web.py", "import json\n\n\ndef handle(body):\n    return json.loads(body), json.dumps(body, indent=2)\n"),
        ("app/main.py", "from app.codec import json\n\n\ndef run():\n    json.loads(\" x \")\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP411,SMP506"]).1;
    assert!(stdout.contains(&common::native("app/codec/json.py:1:5: SMP411")), "{stdout}");
    assert!(!stdout.contains("SMP506"), "{stdout}");
    let defaults = project(&[
        ("app/__init__.py", ""),
        ("app/tool.py", "def helper(host, timeout=30):\n    return host, timeout\n\n\nhelper(\"a\", 30)\n"),
    ]);
    assert!(check(defaults.path(), &["--select", "SMP506"]).1.contains("Every call to"));
}

// Another module's write to a module variable withholds the rename, so SMP102 leaves a mixedCase store to Ruff's N816,
// which reports it with no fix.
#[test_case("cli.py", "from pkg import report\n\nreport.tableJson = 5\n" ; "assignment")]
#[test_case("cli.py", "from pkg import report\n\nsetattr(report, \"tableJson\", 5)\n" ; "setattr")]
#[test_case(
    "test_report.py",
    concat!(
        "from unittest import mock\n",
        "\n",
        "\n",
        "def test_it() -> None:\n",
        "    with mock.patch(\"pkg.report.tableJson\"):\n",
        "        pass\n"
    ) ; "dotted_patch"
)]
fn stored_module_variable_left_to_n816_without_fix(path: &str, body: &str) {
    let report = "tableJson = 0\n\n\ndef read() -> int:\n    return tableJson\n";
    let root = project(&[("pkg/__init__.py", ""), ("pkg/report.py", report), (path, body)]);
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    assert_eq!((stdout, common::read(root.path(), "pkg/report.py")), (common::PASSED.to_owned(), report.to_owned()));
    let unwritten = project(&[("pkg/__init__.py", ""), ("pkg/report.py", report)]);
    check(unwritten.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    assert_ne!(common::read(unwritten.path(), "pkg/report.py"), report);
}

// A library directory whose name only starts with `test`, such as `testkit`, is not a nested test project, though its
// `sub` holds a `conf.py`, so the docstring rule still checks its public function.
#[test]
fn test_prefixed_library_directory_is_not_a_nested_project() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("testkit/__init__.py", ""),
        ("testkit/sub/conf.py", "\"\"\"Conf.\"\"\"\n\nVALUE = 1\n"),
        ("testkit/sub/mod.py", "def public_thing(x: int) -> int:\n    return x\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP903"]).1;
    assert!(stdout.contains(&common::native("testkit/sub/mod.py")), "{stdout}");
}

// A doctest reads a name through a package whose `__init__.py` does not bind the submodule it names, so the name counts
// as read by that submodule's file, which no import reaches.
#[test_case("\"\"\"Guide.\n\n>>> import pkg\n>>> pkg.reportviews._build()\n1\n\"\"\"\n", false ; "doctest_reads_it")]
#[test_case("\"\"\"Guide.\"\"\"\n", true ; "docstring_reads_nothing")]
fn doctest_reads_through_a_package_reach_the_submodule(guide: &str, reported: bool) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/classes/__init__.py", ""),
        ("pkg/classes/reportviews.py", "def _build() -> int:\n    return 1\n"),
        ("pkg/guide.py", guide),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains("SMP409 `_build`"), reported, "{stdout}");
}
