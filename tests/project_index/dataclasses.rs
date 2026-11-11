use super::*;
use crate::common::{check, project};
use test_case::test_case;

// A package's `__init__.py` defines the package, so a write on an instance of a class it imports from there counts.
#[test]
fn dataclass_frozen_reads_writes_on_package_classes() {
    let root = project(&[
        ("pkg/__init__.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Context:\n    count: int\n"),
        ("pkg/run.py", "from pkg import Context\n\n\ndef run(context: Context) -> None:\n    context.count = 1\n"),
    ]);
    check(root.path(), &["--no-cache", "--select", "SMP701", "--target-version", "py314", "--fix", "--unsafe-fixes"]);
    assert_eq!(common::read(root.path(), "pkg/__init__.py").lines().nth(3), Some("@dataclass"));
}

// A positional call in another file is not rewritten: the class report names it, and a fix needs every call in the
// class's own file. Each shape of caller keeps the class reported, with no fix, and leaves the caller as it was.
#[test_case("from models import Partition, helper\n\nhelper(1)\nPartition(\"a\", 1)\n" ; "imported")]
#[test_case("import models\n\nmodels.Partition(\"a\", 1)\n" ; "module")]
#[test_case("from models import Partition\n\nPartition(\"a\", 1)  # noqa: E501\n" ; "other_code_suppressed")]
#[test_case("from models import Partition\n\nPartition(\"a\", 1)\nPartition(\"b\", 2)\n" ; "several_calls")]
#[test_case("from models import Partition\n\nPartition(\"noqa\", 1)\n" ; "noqa_in_string")]
#[test_case("from models import Partition as P\n\nP(\"a\", 1)\n" ; "alias")]
#[test_case("from models import Partition\n\nrow = {\"number\": 1}\nPartition(\"a\", **row)\n" ; "keyword_unpacking")]
fn dataclass_kw_only_leaves_cross_file_calls_to_the_class_report(caller: &str) {
    let models = format!("{PARTITION}\n\ndef helper(value):\n    return value\n");
    let root = project(&[("models.py", &models), ("caller.py", caller)]);
    let fix = ["--select", "SMP703", "--fix", "--unsafe-fixes", "--target-version", "py314"];
    let first = check(root.path(), &fix).1;
    assert_eq!(
        (
            first.contains("Found 1 error.\n"),
            common::read(root.path(), "models.py") == models,
            common::read(root.path(), "caller.py") == caller,
        ),
        (true, true, true)
    );
}

// The report names the positional calls of another file as related locations, in full output, and says they need
// converting by hand, since no fix reaches them.
#[test]
fn dataclass_kw_only_points_at_cross_file_calls() {
    let root = project(&[
        ("pyproject.toml", DEMO_BUILD),
        ("models.py", &format!("__all__ = []\n\n{PARTITION}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    assert_eq!(
        common::simplint(root.path(), &["check", "--select", "SMP703", "--output-format", "full"]).1,
        common::native(
            "SMP703 Dataclass is missing kw_only; set `kw_only=True`; 1 positional call in another file needs \
             keyword arguments by hand\n --> models.py:6:2\n  |\n6 | @dataclass\n  |  ^^^^^^^^^\n7 | class \
             Partition:\n8 |     topic: str\n  |\n ::: caller.py:3:1\n  |\n1 | from models import Partition\n2 |\n3 | \
             Partition(\"a\", 1)\n  | ----------------- positional arguments pass the fields by position\n\nFound 1 \
             error.\n"
        )
    );
}

// The related locations reach the machine formats as `related` in JSON and `relatedLocations` in SARIF, and a second
// run that reads the report from the cache shows the same output.
#[test_case("json" ; "json")]
#[test_case("sarif" ; "sarif")]
fn dataclass_kw_only_related_locations_in_machine_formats(format: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO_BUILD),
        ("models.py", &format!("__all__ = []\n\n{PARTITION}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    let args = ["check", "--select", "SMP703", "--output-format", format];
    let first = common::simplint(root.path(), &args).1;
    let second = common::simplint(root.path(), &args).1;
    let report: serde_json::Value = serde_json::from_str(&first).expect("json report");
    let related = match format {
        "json" => &report[0]["related"],
        _ => &report["runs"][0]["results"][0]["relatedLocations"],
    };
    let location = match format {
        "json" => related[0]["filename"].as_str().map(str::to_owned),
        _ => related[0]["physicalLocation"]["artifactLocation"]["uri"].as_str().map(str::to_owned),
    };
    assert_eq!((first == second, location.is_some_and(|uri| uri.ends_with("caller.py"))), (true, true));
}

#[test]
fn dataclass_kw_only_fixes_same_file_calls_with_the_class() {
    let root = project(&[("models.py", &format!("{PARTITION}\nPartition(\"a\", 1)\n"))]);
    let fix = ["--select", "SMP703", "--fix", "--unsafe-fixes", "--target-version", "py314"];
    assert_eq!(check(root.path(), &fix).1, common::ONE_FIXED);
    assert_eq!(common::read(root.path(), "models.py").lines().last(), Some("Partition(topic=\"a\", number=1)"));
}

// A suppression on the class definition silences the report, and with it the positional calls it names elsewhere.
#[test]
fn dataclass_kw_only_suppressed_class_silences_cross_file_calls() {
    let models = PARTITION.replace("@dataclass\n", "@dataclass  # noqa: SMP703\n");
    let root = project(&[
        ("pyproject.toml", DEMO_BUILD),
        ("models.py", &format!("__all__ = []\n\n{models}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP703"]).1, common::PASSED);
}

// A library's public dataclass keeps its positional calls, which its users make: one defined in a public module's
// `__all__`, or re-exported from one. A dataclass that only a sibling module imports stays internal, so its class and
// its positional calls are reported and rewritten.
#[test_case(
    &[("models.py", "__all__ = [\"Partition\"]\n\n")],
    concat!(
        "from models import Partition\n",
        "\n",
        "Partition(\"a\", 1)\n"
    ),
    false ; "defined_public"
)]
#[test_case(
    &[("models.py", "__all__ = []\n\n"), ("relay.py", "from models import Partition\n\n__all__ = [\"Partition\"]\n")],
    concat!(
        "from relay import Partition\n",
        "\n",
        "Partition(\"a\", 1)\n"
    ),
    false ; "reexported_public"
)]
#[test_case(
    &[("models.py", "__all__ = []\n\n")],
    concat!(
        "from models import Partition\n",
        "\n",
        "Partition(\"a\", 1)\n"
    ),
    true ; "imported_by_sibling"
)]
fn dataclass_kw_only_leaves_public_api_alone(files: &[(&str, &str)], caller: &str, reported: bool) {
    let mut models = String::new();
    let mut others = Vec::new();
    for (name, text) in files {
        if *name == "models.py" {
            models = format!("{text}{PARTITION}");
        } else {
            others.push((*name, *text));
        }
    }
    let mut all = vec![("pyproject.toml", DEMO_BUILD), ("caller.py", caller), ("models.py", models.as_str())];
    all.extend(others);
    let root = project(&all);
    // A library's public class is not reported at all; an internal one is, with no fix while a caller is in another
    // file.
    let stdout = check(root.path(), &["--select", "SMP703"]).1;
    assert_eq!(stdout.contains("Found 1 error."), reported, "{stdout}");
    assert_eq!(common::read(root.path(), "caller.py"), caller);
}

#[test_case(&[("caller.py", "from models import *\n\nPartition(\"a\", 1)\n")] ; "star_import")]
#[test_case(
    &[("caller.py", "import functools\n\nfrom models import Partition\n\nfunctools.partial(Partition, \
    \"a\")\nPartition(\"b\", 2)\n")] ; "partial"
)]
#[test_case(&[("caller.py", "from models import Partition\n\nrow = (\"a\", 1)\nPartition(*row)\n")] ; "unpacked")]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1, 2)\n")] ; "too_many")]
#[test_case(
    &[("relay.py", "from models import Partition\n"), ("caller.py", "from relay import \
    Partition\n\nPartition(topic=\"a\", number=1)\n")] ; "reexported"
)]
#[test_case(
    &[("caller.py", "from models import Partition\n\n\ndef run(make):\n    Partition = make\n    Partition(\"a\", \
    1)\n")] ; "shadowed"
)]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa: SMP703\n")] ; "suppressed")]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa\n")] ; "suppressed_blanket")]
#[test_case(
    &[("caller.py", "from models import Partition\n\nPartition(\n    \"a\",  # simplint: ignore[SMP703] paper\n    \
    1,\n)\n")] ; "ignored_inside"
)]
#[test_case(
    &[(
        "caller.py",
        "import importlib\n\nloaded = importlib.import_module(\"models\")\nloaded.Partition(\"a\", 1)\n",
    )] ; "loaded_module"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"caller.py\" = [\"SMP703\"] }\n"),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]
    ; "ignored_in_caller"
)]
fn dataclass_kw_only_withholds_fix_for_unfollowed_uses(callers: &[(&str, &str)]) {
    let root = project(&[&[("models.py", PARTITION)], callers].concat());
    let fix = ["--no-cache", "--select", "SMP703", "--fix", "--unsafe-fixes", "--target-version", "py314"];
    check(root.path(), &fix);
    check(root.path(), &fix);
    assert_eq!(common::read(root.path(), "models.py"), PARTITION);
}

// Code reading docstrings off objects may read any docstring of the run, so none is reported.
#[test_case("print(_helper.__doc__)\n" ; "attribute")]
#[test_case("import inspect\n\nprint(inspect.getdoc(_helper))\n" ; "getdoc")]
#[test_case("print(getattr(_helper, \"__doc__\"))\n" ; "getattr")]
fn docstring_reader_keeps_docstrings(reader: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/helpers.py", "def _helper() -> None:\n    \"\"\"Help.\"\"\"\n"),
        ("pkg/reader.py", &format!("from pkg.helpers import _helper\n\n{reader}")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP901"]).1, common::PASSED);
}

#[test]
fn module_docstring_read_is_no_docstring_reader() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/helpers.py", "def _helper() -> None:\n    \"\"\"Help.\"\"\"\n"),
        ("pkg/reader.py", "print(__doc__)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP901"]).1.lines().next(),
        Some(
            common::native(
                "pkg/helpers.py:2:5: SMP901 Function docstring on internal code; let the name, signature and types \
                 document it"
            )
            .as_str()
        )
    );
}

// Python names a module of a namespace package, one without an `__init__.py`, from its search root, so its relative
// imports reach the package above it.
#[test]
fn relative_import_from_namespace_package_counts_as_imported() {
    let root = project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/config.py", "__all__ = [\"Config\"]\n\n\nclass Config:\n    pass\n\n\nclass Attribute:\n    pass\n"),
        ("src/pkg/sansio/app.py", "from ..config import Attribute, Config\n\n_TYPES = (Attribute, Config)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403,SMP401"]).1,
        common::native(
            "src/pkg/config.py:8:7: SMP401 [*] `Attribute` is imported by other modules; list it in `__all__`\nFound \
             1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A module loaded by a computed name may be any module, so what is read off it keeps its name everywhere; a name read
// off a module simplint resolves keeps it only there.
#[test_case(
    "def test_app() -> None:\n    mod = importlib.import_module(f\"docs.{_NAME}\")\n    assert mod.client\n" ;
    "import_module"
)]
#[test_case("def test_app(mod: ModuleType) -> None:\n    assert mod.client\n" ; "module_type_parameter")]
#[test_case(
    concat!(
        "def test_app(mod: ModuleType) -> None:\n",
        "    assert getattr(mod, \"client\")\n",
        "    assert hasattr(mod, _NAME)\n"
    ) ; "getattr"
)]
#[test_case(
    concat!(
        "@pytest.fixture(name=\"loaded\")\n",
        "def get_mod():\n",
        "    return importlib.import_module(f\"docs.{_NAME}\")\n",
        "\n",
        "\n",
        "def test_app(loaded) -> None:\n",
        "    alias = cast(Any, loaded)\n",
        "    alias.client = None\n"
    ) ; "fixture_alias"
)]
fn attributes_of_dynamically_loaded_modules_keep_their_names(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        ("tests/test_docs.py", &format!("import importlib\n\n_NAME = \"tutorial\"\n\n\n{test}")),
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
