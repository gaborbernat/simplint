mod common;

use test_case::test_case;

use common::{check, check_with_stdin, project};

#[test]
fn import_through_directory_without_init_counts_as_imported() {
    let root = project(&[
        ("support/marks.py", "def needs_network() -> None:\n    pass\n"),
        ("client.py", "from support.marks import needs_network\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1,
        common::native(
            "support/marks.py:1:5: SMP401 `needs_network` is imported by other modules; list it in `__all__`\nFound 1 \
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
        check(root.path(), &["--select", "SMP401"]).1.lines().next(),
        Some(
            common::native(
                "lib/pkg/helpers.py:1:5: SMP401 `helper` is imported by other modules; list it in `__all__`"
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
        check(root.path(), &["--select", "SMP401"]).1.lines().next(),
        Some(
            common::native(
                "src/pkg/helpers.py:1:5: SMP401 `helper` is imported by other modules; list it in `__all__`"
            )
            .as_str()
        )
    );
}

// An entry point names a module in the directory its `pyproject.toml` is in, which a flat layout keeps its modules in,
// so the entry point uses the name whatever `src` lists.
#[test_case(
    "cli.py",
    "[project]\nscripts = { tool = \"cli:run\" }\n[tool.ruff]\nsrc = [\"src\"]\n"
    ; "flat_layout_script"
)]
#[test_case(
    "cli.py",
    "[build-system]\nrequires = [\"setuptools\"]\nbuild-backend = \"setuptools.build_meta\"\n\
     [project]\nscripts = { tool = \"cli:run\" }\n[tool.setuptools]\npy-modules = [\"cli\"]\n\
     [tool.ruff]\nsrc = [\"src\"]\n"
    ; "flat_layout_py_modules"
)]
#[test_case(
    "src/pkg/cli.py",
    "[project]\nscripts = { tool = \"pkg.cli:run\" }\n[tool.ruff]\nsrc = [\"src\"]\n"
    ; "src_layout_script"
)]
fn entry_point_module_resolves_from_its_project(name: &str, pyproject: &str) {
    let root = project(&[
        ("pyproject.toml", pyproject),
        ("src/pkg/__init__.py", ""),
        (name, "__all__ = []\n\n\ndef run() -> None:\n    pass\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401,SMP403,SMP409"]).1,
        common::native(&format!(
            "{name}:4:5: SMP401 [*] `run` is imported by other modules; list it in `__all__`\nFound 1 error.\n[*] 1 fixable \
             with `--fix`.\n"
        ))
    );
}

// A bare module entry point loads the whole module, so the names it defines are the entry point's, whatever `src` says.
#[test]
fn flat_layout_module_entry_point_uses_its_public_names() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nentry-points = { demo = { basic = \"plugin\" } }\n[tool.ruff]\nsrc = [\"src\"]\n",
        ),
        ("src/pkg/__init__.py", ""),
        ("plugin.py", "__all__ = []\n\n\ndef activate() -> None:\n    pass\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401,SMP403,SMP409"]).1,
        common::native(
            "plugin.py:4:5: SMP401 [*] `activate` is imported by other modules; list it in `__all__`\nFound 1 error.\n[*] 1 \
             fixable with `--fix`.\n"
        )
    );
}

// A local module named after a standard-library one is the module the run imports, not the standard-library one.
#[test]
fn local_module_shadowing_the_standard_library_resolves_to_the_run() {
    let root = project(&[
        ("colorsys.py", "def helper() -> None:\n    pass\n"),
        ("client.py", "from colorsys import helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1.lines().next(),
        Some(
            common::native("colorsys.py:1:5: SMP401 `helper` is imported by other modules; list it in `__all__`")
                .as_str()
        )
    );
}

// A module only a stub declares has no source to read its attributes from, so reaching them through it is left alone.
#[test]
fn stub_only_module_attribute_is_not_reported() {
    let root = project(&[
        ("writers/__init__.py", ""),
        ("writers/findings.pyi", "def save() -> None: ...\n"),
        ("client.py", "from writers import findings\n\n\nfindings.save()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP304"]).1, "All checks passed!\n");
}

// A namespace package under `src` has no `__init__.py` at any level, and its modules import by their dotted path.
#[test]
fn namespace_package_under_src_resolves_its_modules() {
    let root = project(&[
        ("src/outer/inner/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from outer.inner.helpers import helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1.lines().next(),
        Some(
            common::native(
                "src/outer/inner/helpers.py:1:5: SMP401 `helper` is imported by other modules; list it in `__all__`"
            )
            .as_str()
        )
    );
}

// A stub for another module declares nothing about `backend.py`.
#[test_case("backend.pyi", "All checks passed!\n" ; "stub_of_the_module")]
#[test_case(
    "other.pyi",
    "backend.py:10:7: SMP403 `Missing` is not in `__all__`; name it `_Missing`\nFound 1 error.\n1 unsafe fix available \
     with `--fix --unsafe-fixes`.\n" ;
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
    assert_eq!(check(root.path(), &["--select", "SMP102,SMP403"]).1, stdout);
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
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, "All checks passed!\n");
}

#[test_case("pkg.manager._load", "All checks passed!\n" ; "patched")]
#[test_case(
    "pkg.manager.run",
    "pkg/manager.py:1:5: SMP501 `_load` only forwards to `import_module`; call `import_module` at its call sites \
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
    assert_eq!(check(root.path(), &["--select", "SMP501"]).1, common::native(stdout));
}

// `import *` takes the names `__all__` lists, or without one every public name, so those may be in use unnamed.
#[test_case("", "All checks passed!\n" ; "every_public_name")]
#[test_case(
    "__all__ = [\"run\"]\n",
    "pkg/data.py:2:1: SMP404 `TAG` is not in `__all__`; name it `_TAG`\nFound 1 error.\n1 unsafe fix available \
     with `--fix --unsafe-fixes`.\n" ;
    "listed_names"
)]
fn star_import_reaches_public_names(all: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/data.py", &format!("{all}TAG = {{\"a\": 1}}\n\n\ndef run() -> None:\n    pass\n")),
        ("pkg/user.py", "from pkg.data import *\n\n\ndef get() -> dict:\n    return TAG\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP404"]).1, common::native(stdout));
}

// A store to a module attribute from another file uses the name, as `setattr` over a literal does, so no fix localizes
// the module variable or renames it out from under the store.
#[test_case("report.TABLE_JSON_DIR = Path(\"out\")\n" ; "assignment")]
#[test_case("del report.TABLE_JSON_DIR\n" ; "deletion")]
#[test_case("setattr(report, \"TABLE_JSON_DIR\", Path(\"out\"))\n" ; "setattr")]
fn stored_module_variable_stays_module_level(store: &str) {
    let report = "from pathlib import Path\n\nTABLE_JSON_DIR: Path | None = None\n\n\ndef render() -> bool:\n    return \
                  TABLE_JSON_DIR is not None\n";
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/report.py", report),
        ("pkg/cli.py", &format!("from pathlib import Path\n\nfrom pkg import report\n\n{store}")),
    ]);
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP202,SMP404"]);
    assert_eq!(
        (stdout, common::read(root.path(), "pkg/report.py")),
        ("All checks passed!\n".to_owned(), report.to_owned())
    );
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
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102,SMP403,SMP401"]);
    assert_eq!(
        (stdout, common::read(root.path(), "lib/humio/_types.py")),
        (
            common::native(
                "lib/humio/_types.py:1:7: SMP401 `ResourceType` is imported by other modules; list it in `__all__`\n\
                 lib/humio/_types.py:5:5: SMP401 `parse` is imported by other modules; list it in `__all__`\nFound 2 \
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
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "support/other.py:1:1: SMP404 `region` is not in `__all__`; name it `_region`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// Another file's writes count by the class of their receiver: a mock's never, an instance's or an unknown object's
// always.
#[test_case("mock = MagicMock()\n    mock.log = log\n", "[*] ", "@dataclass(frozen=True)" ; "mock")]
#[test_case("result = Result(log=\"\")\n    result.log = log\n", "", "@dataclass" ; "instance")]
#[test_case("thing = make()\n    thing.log = log\n", "", "@dataclass" ; "unknown")]
#[test_case("results: dict[str, Result] = {}\n    results[\"a\"].log = log\n", "", "@dataclass" ; "subscript")]
#[test_case(
    "pass\n\n\nclass Result:\n    def __init__(self) -> None:\n        self.extra = 1\n",
    "",
    "@dataclass(frozen=True)"
    ; "namesake"
)]
#[test_case(
    "import pickle\n\n    result = Result(log=\"\")\n    result.log = log\n    pickle.dumps(result, 1)\n", "", "@dataclass" ; "low_protocol_pickle"
)]
#[test_case(
    "from pickle import dumps as write\n\n    result = Result(log=\"\")\n    result.log = log\n    write(result, 1)\n",
    "",
    "@dataclass"
    ; "low_protocol_aliased_pickle"
)]
#[test_case(
    "import pickle\n\n    result = Result(log=\"\")\n    result.log = log\n    pickle.dumps(result, protocol=pickle.HIGHEST_PROTOCOL)\n",
    "",
    "@dataclass"
    ; "highest_protocol_pickle"
)]
fn dataclass_frozen_reads_writes_in_other_files(body: &str, marker: &str, decorator: &str) {
    let root = project(&[
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Result:\n    log: str\n"),
        (
            "use.py",
            &format!(
                "from unittest.mock import MagicMock\n\nfrom models import Result\n\n\ndef run(log: str) -> None:\n    \
                 {body}"
            ),
        ),
    ]);
    let select = ["--no-cache", "--select", "SMP701", "--target-version", "py314"];
    let report = check(root.path(), &select).1;
    check(root.path(), &[&select[..], &["--fix", "--unsafe-fixes"]].concat());
    assert_eq!(
        (report.lines().next(), common::read(root.path(), "models.py").lines().nth(3)),
        (
            Some(
                common::native(&format!(
                    "models.py:4:2: SMP701 {marker}Dataclass is missing frozen; set `frozen=True`"
                ))
                .as_str()
            ),
            Some(decorator)
        )
    );
}

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

const PARTITION: &str =
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Partition:\n    topic: str\n    number: int\n";

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
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("models.py", &format!("__all__ = []\n\n{PARTITION}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    assert_eq!(
        common::simplint(root.path(), &["check", "--select", "SMP703", "--output-format", "full"]).1,
        common::native(
            "SMP703 Dataclass is missing kw_only; set `kw_only=True`; 1 positional call in another file needs \
             keyword arguments by hand\n --> models.py:6:2\n  |\n6 | @dataclass\n  |  ^^^^^^^^^\n7 | class Partition:\n8 |     topic: str\n  |\n ::: caller.py:3:1\n  |\n1 | from models import Partition\n2 |\n3 | Partition(\"a\", 1)\n  | ----------------- positional arguments pass the fields by position\n\nFound 1 error.\n"
        )
    );
}

// The related locations reach the machine formats as `related` in JSON and `relatedLocations` in SARIF, and a second
// run that reads the report from the cache shows the same output.
#[test_case("json" ; "json")]
#[test_case("sarif" ; "sarif")]
fn dataclass_kw_only_related_locations_in_machine_formats(format: &str) {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
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

// Positional calls in the class's own file are covered by the fix, so one report rewrites the class and them all.
#[test]
fn dataclass_kw_only_fixes_same_file_calls_with_the_class() {
    let root = project(&[("models.py", &format!("{PARTITION}\nPartition(\"a\", 1)\n"))]);
    let fix = ["--select", "SMP703", "--fix", "--unsafe-fixes", "--target-version", "py314"];
    assert_eq!(check(root.path(), &fix).1, "Found 1 error (1 fixed, 0 remaining).\n");
    assert_eq!(common::read(root.path(), "models.py").lines().last(), Some("Partition(topic=\"a\", number=1)"));
}

// A suppression on the class definition silences the report, and with it the positional calls it names elsewhere.
#[test]
fn dataclass_kw_only_suppressed_class_silences_cross_file_calls() {
    let models = PARTITION.replace("@dataclass\n", "@dataclass  # noqa: SMP703\n");
    let root = project(&[
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("models.py", &format!("__all__ = []\n\n{models}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP703"]).1, "All checks passed!\n");
}

// A library's public dataclass keeps its positional calls, which its users make: one defined in a public module's
// `__all__`, or re-exported from one. A dataclass that only a sibling module imports stays internal, so its class and
// its positional calls are reported and rewritten.
#[test_case(&[("models.py", "__all__ = [\"Partition\"]\n\n")], "from models import Partition\n\nPartition(\"a\", 1)\n", false ; "defined_public")]
#[test_case(
    &[("models.py", "__all__ = []\n\n"), ("relay.py", "from models import Partition\n\n__all__ = [\"Partition\"]\n")],
    "from relay import Partition\n\nPartition(\"a\", 1)\n",
    false
    ; "reexported_public"
)]
#[test_case(&[("models.py", "__all__ = []\n\n")], "from models import Partition\n\nPartition(\"a\", 1)\n", true ; "imported_by_sibling")]
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
    let mut all = vec![
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("caller.py", caller),
        ("models.py", models.as_str()),
    ];
    all.extend(others);
    let root = project(&all);
    // A library's public class is not reported at all; an internal one is, with no fix while a caller is in another file.
    let stdout = check(root.path(), &["--select", "SMP703"]).1;
    assert_eq!(stdout.contains("Found 1 error."), reported, "{stdout}");
    assert_eq!(common::read(root.path(), "caller.py"), caller);
}

// A use no rewrite can follow keeps the decorator as it is, however often the fixes run.
#[test_case(&[("caller.py", "from models import *\n\nPartition(\"a\", 1)\n")] ; "star_import")]
#[test_case(&[("caller.py", "import functools\n\nfrom models import Partition\n\nfunctools.partial(Partition, \"a\")\nPartition(\"b\", 2)\n")] ; "partial")]
#[test_case(&[("caller.py", "from models import Partition\n\nrow = (\"a\", 1)\nPartition(*row)\n")] ; "unpacked")]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1, 2)\n")] ; "too_many")]
#[test_case(
    &[("relay.py", "from models import Partition\n"), ("caller.py", "from relay import Partition\n\nPartition(topic=\"a\", number=1)\n")]
    ; "reexported"
)]
#[test_case(
    &[("caller.py", "from models import Partition\n\n\ndef run(make):\n    Partition = make\n    Partition(\"a\", 1)\n")]
    ; "shadowed"
)]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa: SMP703\n")] ; "suppressed")]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa\n")] ; "suppressed_blanket")]
#[test_case(
    &[("caller.py", "from models import Partition\n\nPartition(\n    \"a\",  # simplint: ignore[SMP703] paper\n    1,\n)\n")]
    ; "ignored_inside"
)]
#[test_case(
    &[("caller.py", "import importlib\n\nloaded = importlib.import_module(\"models\")\nloaded.Partition(\"a\", 1)\n")]
    ; "loaded_module"
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
    assert_eq!(check(root.path(), &["--select", "SMP901"]).1, "All checks passed!\n");
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
    "def test_app(mod: ModuleType) -> None:\n    assert getattr(mod, \"client\")\n    assert hasattr(mod, _NAME)\n" ;
    "getattr"
)]
#[test_case(
    "@pytest.fixture(name=\"loaded\")\ndef get_mod():\n    return importlib.import_module(f\"docs.{_NAME}\")\n\n\n\
     def test_app(loaded) -> None:\n    alias = cast(Any, loaded)\n    alias.client = None\n" ;
    "fixture_alias"
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
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A module loaded by a string-named import call is the module the names read off its result belong to, whether the
// call's result is read directly or through a variable it is assigned to.
#[test_case(
    "import importlib\n\n\ndef test_app() -> None:\n    assert importlib.import_module(\"docs.tutorial\").client\n" ;
    "import_module"
)]
#[test_case("def test_app() -> None:\n    assert __import__(\"docs.tutorial\").client\n" ; "dunder_import")]
#[test_case(
    "import pytest\n\n\ndef test_app() -> None:\n    assert pytest.importorskip(\"docs.tutorial\").client\n" ;
    "importorskip"
)]
#[test_case(
    "import pytest\n\n\ndef test_app() -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    assert mod.client\n" ;
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
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A variable a string-named import call loads a module into, and some other value binds too, reads no module.
#[test]
fn variable_rebound_to_another_value_reads_no_loaded_module() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\nengine = object()\n"),
        (
            "tests/test_docs.py",
            "import pytest\n\n\ndef test_app(other) -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    mod = \
             other\n    assert mod.client\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP404"]).1,
        common::native(
            "docs/tutorial.py:1:1: SMP404 `client` is not in `__all__`; name it `_client`\ndocs/tutorial.py:2:1: SMP404 \
             `engine` is not in `__all__`; name it `_engine`\nFound 2 errors.\n2 unsafe fixes available with `--fix \
             --unsafe-fixes`.\n"
        )
    );
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
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
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
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, common::native("All checks passed!\n"));
}

// A test module that loads `docs/tutorial.py` from `path` and reads its names off the module, once directly and once
// through a variable.
fn spec_loading_test(path: &str) -> String {
    format!(
        "import importlib.util\nimport os\nimport pathlib\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    \
         spec = importlib.util.spec_from_file_location(\"tutorial\", {path})\n    assert spec is not None\n    module = \
         importlib.util.module_from_spec(spec)\n    assert spec.loader is not None\n    spec.loader.exec_module(module)\n    \
         alias = module\n    assert alias.client\n"
    )
}

// A module `runpy` runs keeps the names a string key reads off its globals, however the call spells the module.
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    assert runpy.run_module(\"docs.tutorial\")[\"client\"]\n" ;
    "run_module_subscript"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\", run_name=\"x\")\n    assert ns.get(\"client\")\n" ;
    "run_module_get"
)]
#[test_case(
    "import runpy\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    ns = runpy.run_path(str(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\"))\n    assert ns[\"client\"]\n" ;
    "run_path_computed"
)]
#[test_case(
    "import runpy\nfrom pathlib import Path\n\n\ndef test_app() -> None:\n    assert runpy.run_path(Path(__file__).parents[1] / \"docs\" / \"tutorial.py\")[\"client\"]\n" ;
    "run_path_static"
)]
#[test_case(
    "import runpy\n\n\ndef test_app(name: str) -> None:\n    assert runpy.run_module(name)[\"client\"]\n" ;
    "run_module_computed"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\")\n    ns = {}\n    assert ns[\"client\"]\n" ;
    "rebound_to_dict"
)]
#[test_case(
    "import runpy\n\n\ndef test_app() -> None:\n    ns = runpy.run_module(\"docs.tutorial\")\n    ns = runpy.run_module(\"docs.other\")\n    assert ns[\"client\"]\n" ;
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
            "docs/tutorial.py:7:5: SMP403 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A computed read a run cannot resolve may read any name of a module, so the names `__all__` lists keep their place only
// when nothing reads them.
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
            "docs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports it; list only what \
             other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A test that patches a module attribute by its dotted path, or through the module object, uses the name, so `__all__`
// keeps it.
#[test_case("import unittest.mock\n\n\ndef test_app() -> None:\n    with unittest.mock.patch(\"docs.tutorial.client\"):\n        pass\n" ; "mock_patch_string")]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(\"docs.tutorial.client\", None)\n" ; "monkeypatch_string")]
#[test_case("from docs import tutorial\n\n\ndef test_app(mocker) -> None:\n    mocker.patch.object(tutorial, \"client\")\n" ; "patch_object_module")]
#[test_case("from docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    monkeypatch.setattr(tutorial, \"client\", None)\n" ; "setattr_module")]
fn patched_names_are_used_by_all(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports it; list only what \
             other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A test that names the attribute it sets, deletes or patches with a loop over string literals uses each of them, as a
// literal name does.
#[test_case("for name in (\"client\", \"engine\"):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "setattr_loop_over_tuple")]
#[test_case("for name, value in ((\"client\", \"a\"), (\"engine\", \"b\")):\n        monkeypatch.setattr(tutorial, name, value)\n" ; "setattr_loop_over_pairs")]
#[test_case("NAMES = [\"client\", \"engine\"]\n\n    for name in NAMES:\n        monkeypatch.delattr(tutorial, name)\n" ; "delattr_loop_over_bound_list")]
#[test_case("for name in {\"client\", \"engine\"}:\n        mock.patch.object(tutorial, name).start()\n" ; "patch_object_loop_over_set")]
fn names_read_in_loops_over_string_literals_are_used(body: &str) {
    let test = format!(
        "from unittest import mock\n\nfrom docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    {body}"
    );
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, common::native("All checks passed!\n"));
}

// A loop over anything but string rows of the same width, whose items the run cannot know, names no attribute, and
// neither does a name rebound to something else after it held string literals.
#[test_case("for name in names():\n        monkeypatch.setattr(tutorial, name, None)\n" ; "call")]
#[test_case("for name in dir(tutorial):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "dir")]
#[test_case("for name in (\"client\", 1):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "mixed_items")]
#[test_case("for name, value in ((\"client\",), (\"engine\", \"b\")):\n        monkeypatch.setattr(tutorial, name, value)\n" ; "ragged_pairs")]
#[test_case("NAMES = [\"client\", \"engine\"]\n    NAMES = names()\n    for name in NAMES:\n        monkeypatch.setattr(tutorial, name, None)\n" ; "rebound_name")]
fn loops_over_unknown_iterables_name_no_attribute(body: &str) {
    let test = format!(
        "from docs import tutorial\n\n\ndef names() -> list[str]:\n    return []\n\n\ndef test_app(monkeypatch) -> None:\n    {body}"
    );
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "__all__ = [\"client\", \"engine\"]\n\nclient = object()\nengine = object()\n"),
        ("tests/test_docs.py", &test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:12: SMP402 [*] `client` is in `__all__`, yet no other module imports it; list only what \
             other modules use\ndocs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports \
             it; list only what other modules use\nFound 2 errors.\n[*] 2 fixable with `--fix`.\n"
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
            "from docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    for name in (\"client\",):\n        for name in (\"engine\",):\n            assert name\n        monkeypatch.setattr(tutorial, name, None)\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "docs/tutorial.py:1:22: SMP402 [*] `engine` is in `__all__`, yet no other module imports it; list only what \
             other modules use\nFound 1 error.\n[*] 1 fixable with `--fix`.\n"
        )
    );
}

// A constant a string in the run spells, as a dotted path a test patches or a parametrized name, keeps its module-level
// definition, which inlining it into the one function that reads it would bypass.
#[test_case("import unittest.mock\n\n\ndef test_app() -> None:\n    with unittest.mock.patch(\"docs.tutorial.LIMIT\"):\n        pass\n" ; "mock_patch_string")]
#[test_case("from docs import tutorial\n\n\ndef test_app(monkeypatch) -> None:\n    monkeypatch.setattr(tutorial, \"LIMIT\", 5)\n" ; "setattr_module")]
#[test_case("import pytest\n\n\n@pytest.mark.parametrize(\"name\", [\"LIMIT\"])\ndef test_app(name: str) -> None:\n    assert name\n" ; "parametrized_name")]
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
    assert_eq!(check(root.path(), &fix).1, common::native("All checks passed!\n"));
    assert_eq!(
        common::read(root.path(), "docs/tutorial.py"),
        "__all__ = []\n\nLIMIT: int | None = None\n\n\ndef limit() -> int | None:\n    return LIMIT or 3\n"
    );
}

// A computed write reaches the module its target names, so the module's variables keep their names. An f-string names a
// module only through the literal dotted text before its first interpolation.
#[test_case("import unittest.mock\n\n\ndef test_app(name: str) -> None:\n    with unittest.mock.patch(f\"docs.tutorial.{name}\"):\n        pass\n", false ; "dotted_prefix_names_the_module")]
#[test_case("import unittest.mock\n\n\ndef test_app(name: str) -> None:\n    with unittest.mock.patch(f\"{name}\"):\n        pass\n", true ; "leading_interpolation_names_no_module")]
#[test_case("import unittest.mock\n\n\ndef test_app() -> None:\n    with unittest.mock.patch(f\"docs.tutorial\"):\n        pass\n", true ; "no_interpolation_writes_nothing")]
#[test_case("import unittest.mock\n\n\ndef test_app(name: str) -> None:\n    with unittest.mock.patch(f\"docs{name}\"):\n        pass\n", true ; "no_dot_before_interpolation")]
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

// `pkg.views` is no module, yet reaches `pkg/sub/views.py` through the submodule the package re-exports; `pkg.missing`
// names no module of the package, and nothing reaches `pkg/sub/other.py`.
#[test]
fn attribute_path_through_a_reexported_submodule_counts_as_referenced() {
    let module = "__all__ = [\"View\"]\n\n\nclass View:\n    pass\n\n\nclass ViewBase:\n    pass\n";
    let root = project(&[
        ("pkg/__init__.py", "from pkg.sub import *\n"),
        ("pkg/sub/__init__.py", "from pkg.sub import views\n"),
        ("pkg/sub/views.py", module),
        ("pkg/sub/other.py", module),
        ("user.py", "import pkg\n\n_BASES = (pkg.views.ViewBase, pkg.missing.ViewBase)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403"]).1,
        common::native(
            "pkg/sub/other.py:8:7: SMP403 `ViewBase` is not in `__all__`; name it `_ViewBase`\nFound 1 error.\n1 unsafe \
             fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

const LIBRARY: &str = "[project]\nname = \"pkg\"\n[build-system]\nrequires = []\n";

// An application has no Python API, so a function other modules import is internal, its docstring included.
#[test]
fn application_has_no_public_api() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"tool\"\n"),
        ("tool/__init__.py", ""),
        (
            "tool/cli.py",
            "def run() -> None:\n    \"\"\"Run the tool from the command line.\"\"\"\n\n\n__all__ = [\"run\"]\n",
        ),
        ("tool/__main__.py", "from tool.cli import run\n\nrun()\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP901,SMP903"]).1,
        "tool/cli.py:2:5: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
         Found 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
            .replace('/', std::path::MAIN_SEPARATOR_STR)
    );
}

// Without an API package to go by, every module counts but those of the documentation.
#[test]
fn documentation_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nrequires = []\n"),
        ("docs/conf.py", "def setup() -> None: ...\n"),
        ("lib.py", "from _lib import run as run\n"),
        ("_lib.py", "def run() -> None: ...\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        "_lib.py:1:5: SMP903 Public function `run` has no docstring; document it for its users; `lib.py` re-exports it\n\
         Found 1 error.\n"
    );
}

// A package picks its implementation in top-level `if` and `try` blocks, and re-exports whichever it picked.
#[test]
fn reexports_in_top_level_blocks_count() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/__init__.py",
            "import sys\n\ntry:\n    from ._fast import run\nexcept ImportError:\n    from ._slow import run\nelse:\n    \
             pass\nfinally:\n    pass\nif sys.platform == \"win32\":\n    from ._windows import path\nelif sys.platform == \
             \"linux\":\n    from ._linux import path\nelse:\n    from ._posix import path\n\n__all__ = [\"path\", \"run\"]\n",
        ),
        ("pkg/_fast.py", "def run() -> None: ...\n"),
        ("pkg/_slow.py", "def run() -> None: ...\n"),
        ("pkg/_windows.py", "path = 1\n"),
        ("pkg/_linux.py", "path = 1\n"),
        ("pkg/_posix.py", "path = 1\n"),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.lines().count(), 6, "{reports}");
}

// A package `__init__.py` Ruff's `lint.exclude` leaves out still presents the API of the modules behind it.
#[test]
fn excluded_init_module_still_reexports() {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}[tool.ruff.lint]\nexclude = [\"pkg/__init__.py\"]\n")),
        ("pkg/__init__.py", "from ._core import run\n\n__all__ = [\"run\"]\n"),
        ("pkg/_core.py", "def run() -> None: ...\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        "pkg/_core.py:1:5: SMP903 Public function `run` has no docstring; document it for its users; \
         `pkg/__init__.py` re-exports it\nFound 1 error.\n"
            .replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A module read from standard input has no index, so its own `__all__` tells what it offers. Ruff's D103 reports the
// undocumented `run` of such a module, so SMP903 leaves it to Ruff.
#[test]
fn standard_input_reads_its_own_exports() {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}[tool.ruff.lint]\nselect = [\"D103\"]\n")),
        ("pkg/__init__.py", ""),
    ]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP903", "--stdin-filename", "pkg/mod.py", "-"],
        "def run() -> None: ...\n\n\ndef helper() -> None: ...\n\n\n__all__ = [\"run\"]\n",
    )
    .1;
    assert_eq!(stdout, "All checks passed!\n");
}

// A module-level `__getattr__` supplies names the index cannot list, so its re-exports do not count, while a plain
// module's do. The names a module defines itself are Ruff's to report.
#[test]
fn lazy_module_offers_what_it_defines() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/__init__.py",
            "from pkg._core import lazy as lazy\n\n\ndef __getattr__(name: str) -> int:\n    return len(name)\n",
        ),
        ("pkg/_core.py", "def lazy() -> None: ...\n\n\ndef helper() -> None: ...\n"),
        ("pkg/plain.py", "from pkg._core import helper as helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        format!(
            "pkg{sep}_core.py:4:5: SMP903 Public function `helper` has no docstring; document it for its users; \
             `pkg{sep}plain.py` re-exports it\nFound 1 error.\n",
            sep = std::path::MAIN_SEPARATOR_STR
        )
    );
}

const DOCSTRINGED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    \"\"\"Register the flag.\"\"\"\n    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \"\"\"Format messages.\"\"\"\n";
const UNDOCUMENTED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \"\"\"Format messages.\"\"\"\n";

// A bare module entry point is loaded whole by its host, which finds the hooks and fixtures it defines by name, so its
// public names are public API whatever `__all__` lists. An entry point naming an attribute makes only that one public.
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    "All checks passed!\n" ; "bare_module_hook_is_not_internal"
)]
#[test_case(
    "entry-points.pytest11.demo = \"missing.plugin\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
     Found 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ; "unresolved_module_names_nothing"
)]
#[test_case(
    "",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
     Found 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ; "unlisted_hook_is_internal"
)]
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin:Formatter\"\n",
    DOCSTRINGED_PLUGIN,
    "SMP901",
    "pkg/plugin.py:5:5: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
     Found 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ; "attribute_entry_point_names_one_name"
)]
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    UNDOCUMENTED_PLUGIN,
    "SMP903",
    "pkg/plugin.py:4:5: SMP903 Public function `pytest_addoption` has no docstring; document it for its users\n\
     Found 1 error.\n" ; "bare_module_undocumented_hook_reported"
)]
#[test_case(
    "",
    UNDOCUMENTED_PLUGIN,
    "SMP903",
    "All checks passed!\n" ; "unlisted_undocumented_hook_not_reported"
)]
fn bare_module_entry_point_names_are_public_api(entry_points: &str, source: &str, rule: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", ""),
        ("pkg/plugin.py", source),
    ]);
    assert_eq!(
        check(root.path(), &["--select", rule]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

const UNUSED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    print(parser)\n\n\ndef helper() -> None:\n    print(\"helper\")\n\n\n__all__ = [\"pytest_addoption\"]\n";

// The host of a bare module entry point uses every public name of it, so none of them is unused, and an `__all__` entry
// it does not import is not a stale export.
#[test_case(
    "entry-points.pytest11.demo = \"pkg.plugin\"\n",
    "All checks passed!\n" ; "bare_module_names_are_used"
)]
#[test_case(
    "",
    "pkg/plugin.py:8:5: SMP409 `helper` is never used in the run; remove it or give it a caller\nFound 1 error.\n" ;
    "unlisted_names_are_unused"
)]
fn bare_module_entry_point_names_are_used(entry_points: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", ""),
        ("pkg/plugin.py", UNUSED_PLUGIN),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402,SMP409"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A bare package entry point is its `__init__.py`, whose imports from a private module are public API the way a
// re-export is. Without the entry point, a plain import in a module without `__all__` keeps the name internal.
#[test_case(
    "entry-points.pytest11.demo = \"pkg\"\n",
    "pkg/_core.py:1:5: SMP903 Public function `plugin_hook` has no docstring; document it for its users; `pkg/__init__.py` \
     re-exports it\nFound 1 error.\n" ; "package_imports_are_public"
)]
#[test_case("", "All checks passed!\n" ; "unlisted_package_imports_stay_internal")]
fn bare_package_entry_point_imports_are_public_api(entry_points: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\n{entry_points}[build-system]\nrequires = []\n")),
        ("pkg/__init__.py", "from pkg._core import plugin_hook\n"),
        ("pkg/_core.py", "def plugin_hook() -> None:\n    print()\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// The `__all__` of a public module of a library is the interface its users import, so no module of the run needs to
// import it. A private module of the same library still reports the names nothing imports.
#[test_case("pkg/errors.py", "All checks passed!\n" ; "public_module_kept")]
#[test_case(
    "pkg/_errors.py",
    "pkg/_errors.py:4:12: SMP402 `Oops` is in `__all__`, yet no other module imports it; list only what other modules \
     use\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "private_module_reported"
)]
fn public_module_all_is_the_declared_api(module: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", "\"\"\"Package.\"\"\"\n"),
        (module, "class Oops(Exception): ...\n\n\n__all__ = [\"Oops\"]\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// Nothing outside an application imports its modules, so dropping an unused export is safe there.
#[test]
fn application_drops_unused_exports_safely() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"tool\"\n"),
        ("tool/__init__.py", ""),
        ("tool/cli.py", "def run() -> None: ...\n\n\n__all__ = [\"run\"]\n"),
    ]);
    check(root.path(), &["--select", "SMP402", "--fix"]);
    assert_eq!(common::read(root.path(), "tool/cli.py"), "def run() -> None: ...\n\n\n__all__ = []\n");
}

// A namespace package's API package is its dotted name, so its modules are public below the namespace.
#[test_case("[tool.uv.build-backend]\nmodule-name = \"acme.widgets\"\n" ; "build_backend")]
#[test_case("[tool.simplint]\napi-packages = [\"acme.widgets\"]\n" ; "setting")]
fn namespace_package_is_public_api(settings: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"acme-widgets\"\n[build-system]\nrequires = []\n{settings}")),
        ("src/acme/widgets/__init__.py", "from ._shapes import circle as circle\n"),
        ("src/acme/widgets/_shapes.py", "def circle() -> int: ...\n"),
        ("src/acme/other.py", "def square() -> int: ...\n"),
    ]);
    let sep = std::path::MAIN_SEPARATOR_STR;
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        format!(
            "{}:1:5: SMP903 Public function `circle` has no docstring; document it for its users; `{}` re-exports it\n\
             Found 1 error.\n",
            ["src", "acme", "widgets", "_shapes.py"].join(sep),
            ["src", "acme", "widgets", "__init__.py"].join(sep)
        )
    );
}

// `__main__` runs as a script, so it offers no API, while another dunder module, as `__about__`, does.
#[test]
fn main_module_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_core.py", "def main() -> None: ...\n\n\ndef helper() -> None: ...\n"),
        ("pkg/__main__.py", "from pkg._core import main as main\n"),
        ("pkg/__about__.py", "from pkg._core import helper as helper\n"),
    ]);
    let sep = std::path::MAIN_SEPARATOR_STR;
    assert_eq!(
        check(root.path(), &["--select", "SMP903"]).1,
        format!(
            "pkg{sep}_core.py:4:5: SMP903 Public function `helper` has no docstring; document it for its users; \
             `pkg{sep}__about__.py` re-exports it\nFound 1 error.\n"
        )
    );
}

const BACKEND: &str = "[project]\nname = \"demo\"\n[build-system]\nbuild-backend = \"build_backend\"\n\
                       requires = []\nbackend-path = [\".\"]\n";

// A build frontend calls the backend's hooks by name, so no import shows they are used: they are public names of the
// backend, as entry points are.
#[test]
fn listed_build_hooks_are_not_unused_exports() {
    let root = project(&[
        ("pyproject.toml", BACKEND),
        ("demo/__init__.py", ""),
        (
            "build_backend.py",
            "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n\
             __all__ = [\"build_wheel\"]\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, "All checks passed!\n");
}

#[test]
fn unlisted_build_hooks_are_listed_like_entry_points() {
    let root = project(&[
        ("pyproject.toml", BACKEND),
        ("demo/__init__.py", ""),
        (
            "build_backend.py",
            "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n\
             def get_requires_for_build_wheel() -> list[str]:\n    return []\n\n\n__all__ = []\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401,SMP403,SMP409"]).1,
        "build_backend.py:1:5: SMP401 [*] `build_wheel` is imported by other modules; list it in `__all__`\n\
         build_backend.py:5:5: SMP401 [*] `get_requires_for_build_wheel` is imported by other modules; list it in \
         `__all__`\nFound 2 errors.\n[*] 2 fixable with `--fix`.\n"
    );
}

// The object `build-backend` names after the colon is the backend, so the run keeps its name.
#[test]
fn build_backend_object_is_used() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nbuild-backend = \"backend:Backend\"\nrequires = []\n"),
        (
            "backend.py",
            concat!(
                "class Backend:\n    def build_wheel(self, wheel_directory: str) -> str:\n",
                "        return wheel_directory\n\n\n__all__ = []\n"
            ),
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP403,SMP409"]).1, "All checks passed!\n");
}

// Only the backend's hooks count as used, so a helper the backend does not call is still reported.
#[test]
fn build_backend_in_backend_path_keeps_its_other_names_reported() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nbuild-backend = \"backend\"\nrequires = []\nbackend-path = [\"build\"]\n"),
        (
            "build/backend.py",
            "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n\
             def helper() -> None:\n    pass\n\n\n__all__ = []\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403,SMP409"]).1,
        common::native(
            "build/backend.py:5:5: SMP403 `helper` is not in `__all__`; list it there, or rename it to `_helper` by hand\n\
             build/backend.py:5:5: SMP409 `helper` is never used in the run; remove it or give it a caller\n\
             Found 2 errors.\n"
        )
    );
}

// A backend is found by its dotted name, as a module or as a package.
#[test_case("pkg.backend", "pkg/backend/__init__.py" ; "package")]
#[test_case("pkg.backend", "pkg/backend.py" ; "module")]
fn build_backend_by_dotted_name(backend: &str, file: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[build-system]\nbuild-backend = \"{backend}\"\nrequires = []\n")),
        (file, "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n__all__ = []\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, "All checks passed!\n");
}

#[test]
fn missing_build_backend_names_no_hooks() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nbuild-backend = \"absent\"\nrequires = []\n"),
        ("other.py", "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n__all__ = []\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP409"]).1,
        "other.py:1:5: SMP409 `build_wheel` is never used in the run; remove it or give it a caller\n\
         Found 1 error.\n"
    );
}

const HOOK_PROJECT: &str = "def _run() -> None:\n    print(\"hook\")\n";

// The interpreter runs the `import` lines of a `.pth` file at startup, so a name they read is used outside the tests, as
// an entry point's is. A bare import reads no name, and a path line or a line with no `import` is not run.
#[test_case("import pkg.hook; pkg.hook._run()\n", "All checks passed!\n" ; "startup_read_keeps_the_name")]
#[test_case("import\tpkg.hook; pkg.hook._run()\n", "All checks passed!\n" ; "tab_after_import_keeps_the_name")]
#[test_case("import pkg.hook as hook; hook._run()\n", "All checks passed!\n" ; "alias_keeps_the_name")]
#[test_case("import pkg.hook\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "bare_import_reads_no_name")]
#[test_case("pkg.hook._run()\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "line_without_import_is_not_run")]
#[test_case("pkg/hook.py\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "path_line_names_nothing")]
#[test_case("import pkg.hook; pkg.hook._run(\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "unparsable_line_names_nothing")]
#[test_case("import pkg.hook; os.sep\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "unbound_attribute_names_nothing")]
#[test_case("import missing.hook; missing.hook._run()\n", "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper module\nFound 1 error.\n" ; "unresolved_module_names_nothing")]
fn startup_pth_reads_are_production_use(pth: &str, stdout: &str) {
    let root = project(&[
        ("pkg.pth", pth),
        ("pkg/__init__.py", ""),
        ("pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP408"]).1,
        stdout.replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A source root under `src` holds the `.pth` file the interpreter reads, as a project laid out with `src/` does.
#[test]
fn startup_pth_under_src_keeps_the_name() {
    let root = project(&[
        ("src/pkg.pth", "import pkg.hook; pkg.hook._run()\n"),
        ("src/pkg/__init__.py", ""),
        ("src/pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, "All checks passed!\n");
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
    assert_eq!(check(root.path(), &["--select", rule]).1, "All checks passed!\n");
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

// Statements that `duplicate-code` reports when another file of their project repeats them.
const REPEATED: &str = "def build(items):\n    total = 0\n    for item in items:\n        total += item\n    count = \
                        len(items)\n    log(total)\n    log(count)\n    return total\n";

// The files `duplicate-code` reports, sorted, by their paths in the concise output.
fn duplicate_code_reports(files: &[(&str, &str)]) -> Vec<String> {
    let root = project(files);
    let stdout = check(root.path(), &["--select", "SMP530"]).1;
    let mut reported: Vec<String> = stdout
        .lines()
        .filter(|line| line.contains(" SMP530 "))
        .filter_map(|line| line.split(':').next())
        .map(|path| path.replace(std::path::MAIN_SEPARATOR, "/"))
        .collect();
    reported.sort_unstable();
    reported
}

// Test directories of a project are projects of their own, each repeating the code of the others by design, so their
// copies are not compared with one another.
#[test]
fn nested_test_projects_are_not_compared_with_each_other() {
    assert_eq!(
        duplicate_code_reports(&[
            ("roots/test-a/conf.py", ""),
            ("roots/test-a/parser.py", REPEATED),
            ("roots/test-b/conf.py", ""),
            ("roots/test-b/parser.py", REPEATED),
        ]),
        Vec::<String>::new()
    );
}

#[test]
fn repeats_within_one_nested_test_project_are_reported() {
    assert_eq!(
        duplicate_code_reports(&[
            ("roots/test-a/conf.py", ""),
            ("roots/test-a/parser.py", REPEATED),
            ("roots/test-a/other.py", REPEATED),
            ("roots/test-b/conf.py", ""),
            ("roots/test-b/parser.py", REPEATED),
        ]),
        ["roots/test-a/other.py", "roots/test-a/parser.py"]
    );
}

#[test]
fn nested_test_project_is_not_compared_with_the_main_project() {
    assert_eq!(
        duplicate_code_reports(&[
            ("pkg/parser.py", REPEATED),
            ("roots/test-a/conf.py", ""),
            ("roots/test-a/parser.py", REPEATED),
        ]),
        Vec::<String>::new()
    );
}

// A directory is a nested project by any of the markers of a project, and each is recognized on its own.
#[test_case("conf.py", "" ; "sphinx_conf")]
#[test_case("pyproject.toml", "[project]\nname = \"nested\"\n" ; "pyproject")]
#[test_case("setup.py", "" ; "setup_py")]
#[test_case("setup.cfg", "[metadata]\nname = nested\n" ; "setup_cfg")]
fn each_project_marker_makes_a_nested_test_project(marker: &str, content: &str) {
    assert_eq!(
        duplicate_code_reports(&[
            ("pkg/parser.py", REPEATED),
            ("roots/test-a/parser.py", REPEATED),
            ("roots/test-a/other.py", REPEATED),
            (&format!("roots/test-a/{marker}"), content),
        ]),
        ["roots/test-a/other.py", "roots/test-a/parser.py"]
    );
}

// A directory without a test name is part of the main project, even with a marker of its own.
#[test]
fn marked_directory_without_a_test_name_is_the_main_project() {
    assert_eq!(
        duplicate_code_reports(&[
            ("pkg/parser.py", REPEATED),
            ("roots/fixtures/conf.py", ""),
            ("roots/fixtures/parser.py", REPEATED),
        ]),
        ["pkg/parser.py", "roots/fixtures/parser.py"]
    );
}

// A test-named directory without a marker is not a project of its own.
#[test]
fn unmarked_test_directory_is_the_main_project() {
    assert_eq!(
        duplicate_code_reports(&[("pkg/parser.py", REPEATED), ("roots/test-a/parser.py", REPEATED)]),
        ["pkg/parser.py", "roots/test-a/parser.py"]
    );
}

// A nested project is indexed on its own: a name `widget.helper()` in one nested project uses is not a use of another
// nested project's `helper`, while the same layout outside a test-named directory shares one index.
#[test_case("roots/test-a", "roots/test-b", &["roots/test-a/support.py:1:5: SMP409 `helper` is never used in the run; remove it or give it a caller", "roots/test-b/use.py:4:5: SMP409 `run` is never used in the run; remove it or give it a caller"] ; "nested_projects_are_indexed_apart")]
#[test_case("roots/fixtures", "roots/other", &["roots/other/use.py:4:5: SMP409 `run` is never used in the run; remove it or give it a caller"] ; "directories_without_test_names_share_one_index")]
fn name_reached_only_from_another_project_is_unused(first: &str, second: &str, expected: &[&str]) {
    let root = project(&[
        (&format!("{first}/conf.py"), ""),
        (&format!("{first}/support.py"), "def helper() -> None:\n    pass\n"),
        (&format!("{second}/conf.py"), ""),
        (&format!("{second}/use.py"), "import support\n\n\ndef run(widget):\n    widget.helper()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    let reported: Vec<&str> = stdout.lines().filter(|line| line.contains(" SMP409 ")).collect();
    let expected: Vec<String> = expected.iter().map(|line| common::native(line)).collect();
    assert_eq!(reported, expected.iter().map(String::as_str).collect::<Vec<_>>());
}

// A nested project under a `tests` directory is not a test module of the outer project, as its files are its own, while
// a directory of the tests without a marker of its own is.
#[test_case("tests/roots/test-c", true, false ; "nested_project_is_not_a_test_module")]
#[test_case("tests/roots/fixtures", false, true ; "test_directory_without_a_marker_is_a_test_module")]
fn test_module_docstring_follows_the_project_of_the_file(directory: &str, marked: bool, reported: bool) {
    let mut files = vec![(format!("{directory}/parser.py"), "\"\"\"Parser.\"\"\"\n".to_owned())];
    if marked {
        files.push((format!("{directory}/conf.py"), String::new()));
    }
    let files: Vec<(&str, &str)> = files.iter().map(|(name, content)| (name.as_str(), content.as_str())).collect();
    let stdout = check(project(&files).path(), &["--select", "SMP902"]).1;
    assert_eq!(stdout.contains(" SMP902 "), reported, "{stdout}");
}

// A nested project takes the kind of the project around it, as a library's nested roots are library modules.
#[test_case("library", false ; "library")]
#[test_case("application", true ; "application")]
fn nested_project_takes_the_project_kind_around_it(kind: &str, reported: bool) {
    let root = project(&[
        (
            "pyproject.toml",
            &format!("[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"{kind}\"\n"),
        ),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/mod.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains(&common::native("roots/test-a/mod.py")), reported, "{stdout}");
}

// A `[tool.simplint]` of a nested project's own replaces the project kind around it.
#[test]
fn nested_project_with_its_own_settings_takes_them() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        ("roots/test-a/pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/mod.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP903"]).1;
    assert!(!stdout.contains(" SMP903 "), "{stdout}");
}

// A nested project's documentation names its own definitions as used, and the documentation of the run's project does
// not reach them, nor the other way round.
#[test]
fn nested_project_names_its_own_documentation() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("index.rst", ".. autofunction:: support.helper\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/index.rst", ".. autofunction:: support.helper\n"),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
        ("roots/test-b/conf.py", ""),
        ("roots/test-b/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    let reported: Vec<&str> = stdout.lines().filter(|line| line.contains(" SMP409 ")).collect();
    assert_eq!(
        reported,
        [format!(
            "{}:1:5: SMP409 `helper` is never used in the run; remove it or give it a caller",
            common::native("roots/test-b/support.py")
        )]
    );
}

// A nested project's `pyproject.toml` that does not parse is an error, as the one of the project around it is.
#[test_case("[project\n" ; "invalid_toml")]
#[test_case("[project]\nname = 1\n" ; "invalid_metadata")]
fn invalid_pyproject_of_a_nested_project_is_an_error(content: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/pyproject.toml", content),
        ("roots/test-a/mod.py", "x = 1\n"),
    ]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101"]);
    assert_eq!((code, stderr.contains("Failed to parse")), (2, true), "{stderr}");
}

// A run from a directory whose name is not UTF-8 cannot resolve its imports, whether it checks a file or standard input.
#[cfg(unix)]
#[test_case(false ; "linted_file")]
#[test_case(true ; "standard_input")]
fn working_directory_that_is_not_utf8_is_an_error(stdin: bool) {
    use std::os::unix::ffi::OsStrExt;
    let root = project(&[]);
    let directory = root.path().join(std::ffi::OsStr::from_bytes(b"bad\xff"));
    std::fs::create_dir(&directory).expect("create directory");
    let (code, _, stderr) = if stdin {
        common::check_with_stdin(&directory, &["--select", "SMP101", "--stdin-filename", "mod.py", "-"], "x = 1\n")
    } else {
        std::fs::write(directory.join("mod.py"), "x = 1\n").expect("write file");
        check(&directory, &["--select", "SMP101"])
    };
    assert_eq!((code, stderr.contains("is not a UTF-8 path")), (2, true), "{stderr}");
}

// A name the run's project reads counts as used in a nested project, so the test module of the run that imports a
// function of a nested project does not leave it unused.
#[test]
fn name_read_by_the_outer_project_is_used_in_a_nested_project() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("use.py", "def run(widget):\n    return widget.helper()\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(!stdout.contains(&common::native("roots/test-a/support.py")), "{stdout}");
}

// A name a nested project reads does not count as used in the run's project, which does not see the nested project.
#[test]
fn name_read_by_a_nested_project_is_unused_in_the_outer_project() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/support.py", "def helper() -> None:\n    pass\n"),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/use.py", "def run(widget):\n    return widget.helper()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(stdout.contains(&common::native("pkg/support.py:1:5: SMP409 `helper`")), "{stdout}");
}

// The names `unused-definition` reports in the run, sorted.
fn unused_names(stdout: &str) -> Vec<&str> {
    let mut names: Vec<&str> =
        stdout.lines().filter(|line| line.contains(" SMP409 ")).filter_map(|line| line.split('`').nth(1)).collect();
    names.sort_unstable();
    names
}

const FUNCTIONS: &str =
    "def public() -> None:\n    pass\n\n\ndef other() -> None:\n    pass\n\n\ndef _hidden() -> None:\n    pass\n";

// What `automodule` with `:members:` documents: the module's public names when it has no `__all__`, the names its
// `__all__` lists when it has one, and with the directive's options the names it excludes or takes in. A directive
// without `:members:` documents the module alone.
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members:\n", &["_hidden"] ; "members_are_the_public_names")]
#[test_case(
    &format!("__all__ = [\"public\"]\n\n\n{FUNCTIONS}"),
    ".. automodule:: pkg.api\n   :members:\n", &["_hidden", "other"] ; "members_are_the_names_of_all"
)]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members:\n   :private-members:\n", &[] ; "private_members_are_taken_in")]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members:\n   :exclude-members: other\n", &["_hidden", "other"] ; "excluded_members_are_left_out")]
#[test_case(&format!("{FUNCTIONS}\n\ndef __special__() -> None:\n    pass\n"), ".. automodule:: pkg.api\n   :members:\n", &["_hidden"] ; "special_names_are_not_reported_without_the_flag")]
#[test_case(&format!("{FUNCTIONS}\n\ndef __special__() -> None:\n    pass\n"), ".. automodule:: pkg.api\n   :members:\n   :special-members:\n", &["_hidden"] ; "special_names_are_not_reported_with_the_flag")]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n   :members: public\n", &["_hidden", "other"] ; "listed_members_only")]
#[test_case(FUNCTIONS, ".. automodule:: pkg.api\n", &["_hidden", "other", "public"] ; "module_without_members")]
fn automodule_members_are_used_as_sphinx_renders_them(source: &str, directive: &str, expected: &[&str]) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/api.py", source),
        ("index.rst", directive),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), expected, "{stdout}");
}

// The methods of an `autoclass` with `:members:` are documented, as the class's own name is.
#[test]
fn autoclass_members_are_used() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("pkg/__init__.py", ""),
        (
            "pkg/api.py",
            "class Widget:\n    def render(self) -> None:\n        pass\n\n    def unused(self) -> None:\n        pass\n",
        ),
        ("index.rst", ".. autoclass:: pkg.api.Widget\n   :members:\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), Vec::<&str>::new(), "{stdout}");
}

// A function an `automodule` with `:members:` documents is public API, so its docstring stays; without the directive it
// is internal code, whose docstring is reported.
#[test_case(Some(".. automodule:: pkg.api\n   :members:\n"), false ; "documented")]
#[test_case(None, true ; "undocumented")]
fn documented_public_function_keeps_its_docstring(page: Option<&str>, reported: bool) {
    let mut files = vec![
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/api.py", "def public() -> None:\n    \"\"\"Public doc.\"\"\"\n"),
    ];
    if let Some(page) = page {
        files.push(("index.rst", page));
    }
    let stdout = check(project(&files).path(), &["--select", "SMP901"]).1;
    assert_eq!(stdout.contains(&common::native("pkg/api.py:2:5: SMP901")), reported, "{stdout}");
}

// A name the run's project imports from a nested project does not make `missing-from-all` report it there, since the
// export rules leave a nested project alone.
#[test_case("from support import helper\n" ; "imported")]
#[test_case("def run(widget):\n    return widget.helper()\n" ; "read")]
fn nested_project_is_not_reported_for_exports_read_by_the_outer_project(outer: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("use.py", outer),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/support.py", "def helper() -> None:\n    pass\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP401"]).1;
    assert!(!stdout.contains(&common::native("roots/test-a/support.py")), "{stdout}");
}

// Files of a nested test project are test input for the rules that judge their docstrings and the visibility of their
// names, so an internal docstring, an undocumented public function and a name `__all__` omits are left alone there,
// while another rule still checks them. The same code outside a nested project is reported.
#[test_case("roots/test-a", false ; "nested_project")]
#[test_case("roots/fixtures", true ; "directory_outside_a_nested_project")]
fn nested_project_is_not_judged_for_docstrings_or_exports(directory: &str, reported: bool) {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        (&format!("{directory}/conf.py"), ""),
        (
            &format!("{directory}/mod.py"),
            "__all__ = [\"keep\"]\n\nbadName = 1\n\n\ndef keep() -> None:\n    pass\n\n\ndef unlisted() -> None:\n    pass\n\n\ndef _internal() -> None:\n    \"\"\"Internal doc.\"\"\"\n\n\ndef f(x: int) -> int:\n    return x\n",
        ),
    ]);
    let stdout = check(root.path(), &["--select", "SMP101,SMP102,SMP401,SMP403,SMP901,SMP903"]).1;
    assert!(stdout.contains(" SMP101 "), "{stdout}");
    let judged = [" SMP901 ", " SMP403 ", " SMP903 ", " SMP102 "].map(|code| stdout.contains(code));
    assert_eq!(judged, [reported; 4], "{stdout}");
}

// Every rule that judges docstrings or the visibility of names leaves a nested test project alone, whatever the file
// holds.
#[test]
fn docstring_and_visibility_rules_leave_a_nested_project_alone() {
    let codes = [
        "SMP102", "SMP401", "SMP402", "SMP403", "SMP404", "SMP901", "SMP902", "SMP903", "SMP904", "SMP910", "SMP911",
        "SMP912", "SMP913", "SMP914", "SMP915", "SMP916", "SMP917", "SMP918", "SMP919", "SMP920", "SMP921", "SMP922",
        "SMP923", "SMP924",
    ];
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"0\"\n\n[tool.simplint]\nproject-kind = \"library\"\n",
        ),
        ("roots/test-a/conf.py", ""),
        (
            "roots/test-a/mod.py",
            "\"\"\"Module.\"\"\"\n\n__all__ = [\"keep\"]\n\nbadName = 1\n\n\ndef keep(x: int) -> int:\n    \"\"\"Keep `x`.\n\n    :param y: missing.\n    :returns: nothing.\n    \"\"\"\n    return x\n\n\ndef unlisted() -> None:\n    pass\n\n\ndef _internal() -> None:\n    \"\"\"Internal doc.\"\"\"\n\n\nclass Holder:\n    \"\"\"Holds.\n\n    :ivar count: a count.\n    \"\"\"\n\n    count: int = 0\n",
        ),
    ]);
    let select = codes.join(",");
    let stdout = check(root.path(), &["--select", &select]).1;
    for code in codes {
        assert!(!stdout.contains(code), "{stdout}");
    }
}

// A module that a test patches by attribute keeps its `module.Name` reads: binding the name at import time would miss
// the patch, so SMP304 offers no fix for any of the patch forms.
#[test_case("from unittest import mock\n\n\ndef test_run() -> None:\n    with mock.patch(\"bench.operations.OPERATIONS\"):\n        pass\n" ; "dotted_string")]
#[test_case("from bench import operations\n\n\ndef test_run() -> None:\n    setattr(operations, \"OPERATIONS\", {})\n" ; "setattr")]
#[test_case("from bench import operations\n\n\ndef test_run(monkeypatch) -> None:\n    monkeypatch.setattr(operations, \"OPERATIONS\", {})\n" ; "monkeypatch")]
#[test_case("from unittest import mock\n\nfrom bench import operations\n\n\ndef test_run() -> None:\n    with mock.patch.object(operations, \"OPERATIONS\"):\n        pass\n" ; "patch_object")]
fn patched_module_attribute_keeps_module_reads(test: &str) {
    let root = project(&[
        ("tools/bench/__init__.py", ""),
        ("tools/bench/operations.py", "OPERATIONS = {}\n\n\ndef run() -> dict:\n    return OPERATIONS\n"),
        (
            "tools/bench/report.py",
            "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n",
        ),
        ("tests/test_ops.py", test),
    ]);
    let (_, stdout, _) = check(root.path(), &["--select", "SMP304", "--unsafe-fixes", "--fix"]);
    assert_eq!(stdout, "All checks passed!\n");
    assert_eq!(
        common::read(root.path(), "tools/bench/report.py"),
        "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n"
    );
}

// Another module's write to a module variable keeps its naming report with no fix: the rename would leave the write
// pointing at the old name.
#[test_case("cli.py", "from pkg import report\n\nreport.tableJson = 5\n" ; "assignment")]
#[test_case("cli.py", "from pkg import report\n\nsetattr(report, \"tableJson\", 5)\n" ; "setattr")]
#[test_case("test_report.py", "from unittest import mock\n\n\ndef test_it() -> None:\n    with mock.patch(\"pkg.report.tableJson\"):\n        pass\n" ; "dotted_patch")]
fn stored_module_variable_keeps_report_without_fix(path: &str, body: &str) {
    let report = "tableJson = 0\n\n\ndef read() -> int:\n    return tableJson\n";
    let root = project(&[("pkg/__init__.py", ""), ("pkg/report.py", report), (path, body)]);
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP102"]);
    assert_eq!(
        (stdout, common::read(root.path(), "pkg/report.py")),
        (
            common::native(
                "pkg/report.py:1:1: SMP102 Module-scope variable `tableJson` should be named `TABLEJSON`\nFound 1 error.\n"
            ),
            report.to_owned()
        )
    );
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

// A nested test project's pages name its own definitions, and do not count as the run's project's documentation, which
// leaves the run's unused definition reported.
#[test]
fn nested_project_pages_do_not_name_the_run_projects_definitions() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"0\"\n"),
        ("support.py", "def helper() -> None:\n    pass\n"),
        ("tests/roots/test-a/conf.py", ""),
        ("tests/roots/test-a/index.rst", ".. autofunction:: support.helper\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert!(stdout.contains(&common::native("support.py")), "{stdout}");
}

// SARIF gives the file of a related location as a file URI, as it gives the primary location of the same result.
#[test]
fn sarif_related_location_is_a_file_uri() {
    let root = project(&[
        ("pyproject.toml", "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n"),
        ("models.py", &format!("__all__ = []\n\n{PARTITION}")),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]);
    let stdout = common::simplint(root.path(), &["check", "--select", "SMP703", "--output-format", "sarif"]).1;
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("sarif report");
    let result = &report["runs"][0]["results"][0];
    let primary = result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"].as_str().unwrap_or_default();
    let related =
        result["relatedLocations"][0]["physicalLocation"]["artifactLocation"]["uri"].as_str().unwrap_or_default();
    assert_eq!((primary.starts_with("file://"), related.starts_with("file://")), (true, true), "{related}");
}
