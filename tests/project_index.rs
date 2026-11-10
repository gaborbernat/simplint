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
fn long_import_cycle_names_its_first_files_and_counts_the_rest() {
    let sources: Vec<(String, String)> =
        (0..12).map(|index| (format!("ring{index}.py"), format!("import ring{}\n", (index + 1) % 12))).collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(name, source)| (name.as_str(), source.as_str())).collect();
    let root = project(&files);
    assert!(check(root.path(), &["--select", "SMP308"]).1.contains(
        "ring0.py:1:1: SMP308 This import closes a cycle of runtime imports, `ring0.py -> ring1.py -> ring2.py -> \
             ring3.py -> ring4.py -> ring5.py -> ring6.py -> ring7.py -> ... (4 more) -> ring0.py`; import inside the \
             function that needs it"
    ));
}

#[test]
fn duplicate_block_shared_by_five_files_counts_the_ones_it_does_not_name() {
    let body = "def run(log):\n    log(1)\n    log(2)\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n";
    let names: Vec<String> = (0..5).map(|index| format!("copy{index}.py")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), body)).collect();
    let root = project(&files);
    assert!(check(root.path(), &["--select", "SMP530"]).1.contains(
        "copy0.py:2:5: SMP530 These statements repeat in `copy1.py:2`, `copy2.py:2`, `copy3.py:2`, and 1 more; move \
             them into one definition the modules share"
    ));
}

#[test]
fn duplicate_block_repeated_in_pieces_names_each_repeating_file_once() {
    let whole = "def run(log):\n    log(1)\n    log(2)\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n    log(7)\n    log(8)\n";
    let head = "def run(log):\n    log(1)\n    log(2)\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n";
    let tail = "def run(log):\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n    log(7)\n    log(8)\n";
    let root = project(&[("a.py", whole), ("b.py", whole), ("c.py", head), ("d.py", tail)]);
    assert!(check(root.path(), &["--select", "SMP530"]).1.contains(
        "a.py:2:5: SMP530 These statements repeat in `b.py:2`, `c.py:2`, `d.py:2`; move them into one definition the \
             modules share"
    ));
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
fn src_layout_test_imports_do_not_require_exports() {
    let root = project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from pkg.helpers import helper\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, "All checks passed!\n");
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
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP403,SMP409"]).1, "All checks passed!\n");
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

// A module named after a standard-library one imports that module by its own name, which is no import of its names, so
// the names its `__all__` lists stay reported when no other module of the run imports them.
#[test]
fn stdlib_import_of_own_stem_does_not_import_own_names() {
    let tomllib = "import sys\n\nif sys.version_info >= (3, 11):\n    from tomllib import TOMLDecodeError, load, loads\n\
                   else:\n    from tomli import TOMLDecodeError, load, loads\n\n__all__ = [\"TOMLDecodeError\", \"load\", \
                   \"loads\"]\n";
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/_compat/__init__.py", ""),
        ("pkg/_compat/tomllib.py", tomllib),
        ("pkg/reader.py", "from pkg._compat.tomllib import load\n\n\ndef read(fh):\n    return load(fh)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP402"]).1,
        common::native(
            "pkg/_compat/tomllib.py:8:12: SMP402 `TOMLDecodeError` is in `__all__`, yet no other module imports it; list \
             only what other modules use\npkg/_compat/tomllib.py:8:39: SMP402 `loads` is in `__all__`, yet no other module \
             imports it; list only what other modules use\nFound 2 errors.\n"
        )
    );
}

// A doctest in a module that shares its package's name reads `tz.gettz`, which names the module it is in. That name is no
// patch of the package's `__init__.py`, so `tz.tzstr` in another module of the run is still an import to offer.
#[test]
fn doctest_naming_its_own_module_does_not_patch_the_package() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/tz/__init__.py", "from .tz import tzstr\n\n__all__ = [\"tzstr\"]\n"),
        ("pkg/tz/tz.py", "class tzstr:\n    \"\"\"A zone.\n\n    >>> tz.gettz(\"America/Chicago\")\n    \"\"\"\n"),
        ("pkg/parser/__init__.py", ""),
        (
            "pkg/parser/_parser.py",
            "from .. import tz\n\n\ndef parse(name: str) -> object:\n    return tz.tzstr(name)\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304"]).1,
        common::native(
            "pkg/parser/_parser.py:5:12: SMP304 Import `tzstr` from `..tz` instead of reaching it through the module\nFound \
             1 error.\n"
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
fn namespace_package_test_imports_do_not_require_exports() {
    let root = project(&[
        ("src/outer/inner/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from outer.inner.helpers import helper\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, "All checks passed!\n");
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
        ("pyproject.toml", DEMO_BUILD),
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
        ("pyproject.toml", DEMO_BUILD),
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
    let mut all = vec![("pyproject.toml", DEMO_BUILD), ("caller.py", caller), ("models.py", models.as_str())];
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

// A variable a string-named import call loads a module into, and some other value binds too, reads the names of any
// module, so a name read off it is not reported.
#[test]
fn variable_rebound_to_another_value_reads_names_of_any_module() {
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
            "docs/tutorial.py:2:1: SMP404 `engine` is not in `__all__`; name it `_engine`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// Two tests each load a module into the same variable, and each reads a name of its module, so both names are read.
#[test]
fn variable_reused_for_each_loaded_module_reads_each_module() {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "client = object()\n"),
        ("docs/other.py", "engine = object()\n"),
        (
            "tests/test_docs.py",
            "import pytest\n\n\ndef test_client() -> None:\n    mod = pytest.importorskip(\"docs.tutorial\")\n    assert \
             mod.client\n\n\ndef test_engine() -> None:\n    mod = pytest.importorskip(\"docs.other\")\n    assert \
             mod.engine\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP404"]).1, "All checks passed!\n");
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
            "docs/tutorial.py:1:1: SMP404 `client` is not in `__all__`; name it `_client`\ndocs/tutorial.py:2:1: SMP404 \
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
            "import pytest\n\n\ndef test_one() -> None:\n    m = pytest.importorskip(\"pkg.cli\")\n    assert m.client\n\n\n\
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
            "import pytest\n\nproxy = pytest.importorskip(\"proxy\")\n\n\ndef test_one(make_proxy) -> None:\n    proxy = \
             make_proxy()\n    assert proxy.url\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP102"]).1.matches("scripts/run.py").count(), 0);
}

// A name a function binds to several modules reads the names of each of them, and of no other module. Binding it to a
// value that is no module too keeps those reads.
#[test_case(
    "    m = pytest.importorskip(\"pkg.cli\")\n    assert m.client\n    m = pytest.importorskip(\"pkg.other\")\n    assert m.engine\n",
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
            "import importlib\n\n\ndef test_one() -> None:\n    app = importlib.import_module(\"pkg.cli\").client\n    assert \
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
// one, given by keyword or by position, names nothing, while a non-empty literal, or one that may hold names, returns the
// module itself.
#[test_case(
    "__import__(\"pkg.sub\", globals(), locals(), [], 0)", "pkg/sub.py:3:1: SMP404 `helper`", "pkg/__init__.py:3:1" ;
    "positional_empty_list"
)]
#[test_case("__import__(\"pkg.sub\", fromlist=())", "pkg/sub.py:3:1: SMP404 `helper`", "pkg/__init__.py:3:1" ; "empty_tuple")]
#[test_case("__import__(\"pkg.sub\", fromlist=None)", "pkg/sub.py:3:1: SMP404 `helper`", "pkg/__init__.py:3:1" ; "none")]
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
// neither does a name rebound to something else after it held string literals, or a computed name.
#[test_case("for name in names():\n        monkeypatch.setattr(tutorial, name, None)\n" ; "call")]
#[test_case("for name in dir(tutorial):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "dir")]
#[test_case("for name in (\"client\", 1):\n        monkeypatch.setattr(tutorial, name, None)\n" ; "mixed_items")]
#[test_case("for name, value in ((\"client\",), (\"engine\", \"b\")):\n        monkeypatch.setattr(tutorial, name, value)\n" ; "ragged_pairs")]
#[test_case("NAMES = [\"client\", \"engine\"]\n    NAMES = names()\n    for name in NAMES:\n        monkeypatch.setattr(tutorial, name, None)\n" ; "rebound_name")]
#[test_case("monkeypatch.setattr(tutorial, names()[0], None)\n" ; "computed_name")]
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
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, other.SHARED, engine.SHARED\n",
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
#[test_case("    for module in (tutorial, other):\n        module = engine\n        monkeypatch.setattr(module, \"SHARED\", {})\n" ; "rebound")]
#[test_case("    for module in (tutorial, other):\n        (lambda module: monkeypatch.setattr(module, \"SHARED\", {}))(engine)\n" ; "shadowed_by_lambda")]
#[test_case("    for module in (tutorial, other):\n        def helper(module: object) -> None:\n            monkeypatch.setattr(module, \"SHARED\", {})\n\n        helper(engine)\n" ; "shadowed_by_parameter")]
fn loops_that_rebind_or_shadow_their_module_patch_neither(body: &str) {
    let test = format!("from docs import engine, other, tutorial\n\n\ndef test_app(monkeypatch) -> None:\n{body}");
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "SHARED = {}\n"),
        ("docs/other.py", "SHARED = {}\n"),
        ("docs/engine.py", "SHARED = {}\n"),
        (
            "docs/app.py",
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, other.SHARED, engine.SHARED\n",
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
        "import importlib\n\nfrom docs import engine, other, tutorial\n\nMODULE = \"docs.other\"\n\n\ndef test_app(monkeypatch) -> None:\n    for module in (tutorial, {item}):\n        monkeypatch.setattr(module, \"SHARED\", {{}})\n"
    );
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "SHARED = {}\n"),
        ("docs/other.py", "SHARED = {}\n"),
        ("docs/engine.py", "SHARED = {}\n"),
        (
            "docs/app.py",
            "from docs import engine, other, tutorial\n\n\ndef run() -> object:\n    return tutorial.SHARED, other.SHARED, engine.SHARED\n",
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

// A test that sets a module variable through a `MonkeyPatch` writes it, whatever the object is named and however it is
// reached, so the variable is not `Final`.
#[test_case("def test_app(monkeypatch) -> None:\n    with monkeypatch.context() as scoped:\n        scoped.setattr(tutorial, \"LIMIT\", 5)\n" ; "context_target")]
#[test_case("def test_app(patches) -> None:\n    patches.setattr(tutorial, \"LIMIT\", 5)\n" ; "fixture_returned")]
#[test_case("def test_app(request) -> None:\n    request.config.patches.setattr(tutorial, \"LIMIT\", 5)\n" ; "attribute_receiver")]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(\"docs.tutorial.LIMIT\", 5)\n" ; "dotted_string")]
#[test_case("def test_app(monkeypatch) -> None:\n    for name, value in ((\"LIMIT\", \"5\"),):\n        monkeypatch.setattr(tutorial, name, value)\n" ; "loop_over_string_literals")]
fn module_variables_patched_through_a_monkeypatch_are_not_final(test: &str) {
    let test = format!("from docs import tutorial\n\n\n{test}");
    let root = project(&[("docs/__init__.py", ""), ("docs/tutorial.py", "LIMIT = 3\n"), ("tests/test_docs.py", &test)]);
    assert_eq!(check(root.path(), &["--select", "SMP601"]).1, common::native("All checks passed!\n"));
}

// A `setattr` on an object that is not a module of the run writes no module variable, so the variable stays `Final`.
#[test_case("from docs.tutorial import Settings\n\n\ndef test_app(monkeypatch) -> None:\n    monkeypatch.setattr(Settings, \"LIMIT\", 5)\n" ; "class_of_module")]
#[test_case("from unittest import mock\n\nfrom docs.tutorial import Settings\n\n\ndef test_app() -> None:\n    with mock.patch.object(Settings, \"LIMIT\", 5):\n        pass\n" ; "class_of_module_patch_object")]
#[test_case("def test_app(monkeypatch, settings) -> None:\n    monkeypatch.setattr(settings, \"LIMIT\", 5)\n" ; "parameter")]
fn setattr_on_a_non_module_writes_no_module_variable(test: &str) {
    let root = project(&[
        ("docs/__init__.py", ""),
        ("docs/tutorial.py", "LIMIT = 3\n\n\nclass Settings:\n    pass\n"),
        ("tests/test_docs.py", test),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP601"]).1,
        common::native(
            "docs/tutorial.py:1:1: SMP601 `LIMIT` is never reassigned; annotate it `Final`\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A write sets the name of the module it is made on, so another module's variable of the same name stays `Final`.
#[test_case("def test_app(monkeypatch) -> None:\n    with monkeypatch.context() as scoped:\n        scoped.setattr(tutorial, \"LIMIT\", 5)\n" ; "context_target")]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(tutorial, \"LIMIT\", 5)\n" ; "setattr_module")]
#[test_case("def test_app(monkeypatch) -> None:\n    monkeypatch.setattr(\"docs.tutorial.LIMIT\", 5)\n" ; "dotted_string")]
#[test_case("def test_app(mocker) -> None:\n    mocker.patch.object(tutorial, \"LIMIT\", 5)\n" ; "mocker_patch_object")]
#[test_case("from unittest import mock\n\n\ndef test_app() -> None:\n    with mock.patch.object(tutorial, \"LIMIT\", 5):\n        pass\n" ; "mock_patch_object_context")]
#[test_case("from unittest import mock\n\n\n@mock.patch.object(tutorial, \"LIMIT\", 5)\ndef test_app() -> None:\n    pass\n" ; "mock_patch_object_decorator")]
#[test_case("from unittest import mock\n\n\ndef test_app() -> None:\n    with mock.patch(\"docs.tutorial.LIMIT\", 5):\n        pass\n" ; "mock_patch_dotted_string")]
#[test_case("from unittest import mock\n\n\ndef test_app() -> None:\n    for name in (\"LIMIT\",):\n        mock.patch.object(tutorial, name, 5).start()\n" ; "patch_object_loop")]
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
#[test_case("def test_app(monkeypatch, name: str) -> None:\n    monkeypatch.setattr(tutorial, name, 5)\n" ; "parameter_name")]
#[test_case("def names() -> list[str]:\n    return []\n\n\ndef test_app(monkeypatch) -> None:\n    for name in names():\n        monkeypatch.setattr(tutorial, name, None)\n" ; "loop_over_call")]
#[test_case("from unittest import mock\n\n\ndef test_app(name: str) -> None:\n    with mock.patch.object(tutorial, name, 5):\n        pass\n" ; "patch_object_parameter")]
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

// A docstring that mentions a constant is prose, which neither patches nor looks it up, so the constant still moves into
// the one function that reads it.
#[test]
fn docstring_mention_does_not_keep_a_constant() {
    let root = project(&[(
        "pkg/helpers.py",
        "_LIMIT = 3\n\n\ndef check(value):\n    \"\"\"Compare with the limit.\n\n    The ``_LIMIT`` constant sets the bound.\n    \"\"\"\n    return value < _LIMIT\n",
    )]);
    assert_eq!(
        check(root.path(), &["--select", "SMP202"]).1,
        common::native(
            "pkg/helpers.py:1:1: SMP202 `_LIMIT` is used by one function only; define it there\nFound 1 error.\n1 unsafe fix \
             available with `--fix --unsafe-fixes`.\n"
        )
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

const LIBRARY: &str = "[project]\nname = \"pkg\"\n[build-system]\nrequires = []\n";
const DEMO: &str = "[project]\nname = \"demo\"\nversion = \"0\"\n";
const DEMO_BUILD: &str = "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n";

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

// A public class a library re-exports from a private module passes its inherited public methods on to its users, so
// the docstring rules judge them as the class's API.
#[test]
fn inherited_methods_of_reexported_class_are_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", "from ._impl import Unix\n\n__all__ = [\"Unix\"]\n"),
        (
            "pkg/_impl.py",
            "class _Mixin:\n    def run(self) -> None: ...\n\n\nclass Unix(_Mixin):\n    \"\"\"Unix.\"\"\"\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("Public method `_Mixin.run`").count(), 1, "{reports}");
}

// A private class another public signature returns reaches its users, who call the methods it inherits from a base the
// module defines, so the docstring rules judge them as the API too.
#[test]
fn inherited_methods_of_signature_reached_class_are_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "class _Base:\n    def run(self) -> None: ...\n\n\nclass _Config(_Base):\n    \"\"\"Config.\"\"\"\n\n\n__all__ = [\"load\"]\n\n\ndef load() -> _Config:\n    \"\"\"Load.\"\"\"\n    return _Config()\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("Public method `_Base.run`").count(), 1, "{reports}");
}

// A public module with an `__all__` offers what the list names, so a name another module imports from it without listing
// it is not part of its API, and its docstring is not required.
#[test]
fn imported_name_all_omits_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/__init__.py",
            "__all__ = [\"undocumented\"]\n\n\ndef undocumented() -> None: ...\n\n\ndef helper() -> None: ...\n",
        ),
        ("pkg/app.py", "from pkg import helper\n\nhelper()\n"),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("`undocumented`").count(), 1, "{reports}");
    assert!(!reports.contains("`helper`"), "{reports}");
}

// A module whose `__getattr__` supplies names lazily offers what it defines, not a name it imports from a private module
// and another file reads off it, as the re-exports of such a module are not offered either.
#[test]
fn name_read_off_lazy_module_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_compat.py", "def f() -> None: ...\n"),
        (
            "pkg/utils.py",
            "from ._compat import f\n\n\ndef local() -> None: ...\n\n\ndef __getattr__(name: str) -> object:\n    raise AttributeError(name)\n",
        ),
        ("pkg/testing.py", "from pkg import utils\n\nold = utils.f\n"),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert!(reports.contains("`local`") && !reports.contains("`f`"), "{reports}");
}

// A class a public name's signature refers to, such as a base class, is part of the API its users reach, so the methods
// its listed subclass inherits from it are documented, while the class itself needs no docstring.
#[test]
fn signature_reached_base_needs_no_docstring_but_its_inherited_methods_do() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "_StrPath = str\n\n\nclass _Base:\n    def run(self, path: _StrPath) -> None: ...\n\n\nclass Child(_Base):\n    \"\"\"Child.\"\"\"\n\n\n__all__ = [\"Child\"]\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("Public method `_Base.run`").count(), 1, "{reports}");
    assert!(!reports.contains("`_Base`") && !reports.contains("`_StrPath`"), "{reports}");
}

// A stub-free module in a library reads the same whether its file is on disk or on standard input: its exposed names
// are the API, and a helper it does not expose is internal.
#[test]
fn standard_input_module_has_the_api_of_its_file() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP903", "--stdin-filename", "pkg/core.py", "-"],
        "def public() -> None: ...\n\n\ndef _private() -> None: ...\n",
    )
    .1;
    assert_eq!(stdout.matches("SMP903").count(), 1, "{stdout}");
    assert!(stdout.contains("`public`") && !stdout.contains("`_private`"), "{stdout}");
}

// A standard-input module that a library exposes keeps its public functions' default, since outside code may leave it out,
// and a helper it does not expose is still judged.
#[test]
fn standard_input_module_keeps_the_defaults_of_its_public_api() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP506", "--stdin-filename", "pkg/core.py", "-"],
        "def connect(host, timeout=30):\n    return host, timeout\n\n\ndef _helper(host, timeout=30):\n    return host, timeout\n\n\nconnect(\"a\", 1)\n_helper(\"a\", 1)\n",
    )
    .1;
    assert_eq!(stdout.matches("Every call to").count(), 1, "{stdout}");
    assert!(stdout.contains("`_helper`") && !stdout.contains("`connect`"), "{stdout}");
}

// A standard-input module's fix leaves the docstring of a public function alone, and rewrites the docstring of an
// internal one.
#[test]
fn standard_input_fix_keeps_the_docstring_of_public_api() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP901", "--fix", "--unsafe-fixes", "--stdin-filename", "pkg/core.py", "-"],
        "def connect(host: str) -> str:\n    \"\"\"Connect to the host.\"\"\"\n    return host\n\n\ndef _inner(host: str) -> str:\n    \"\"\"Inner.\"\"\"\n    return host\n",
    )
    .1;
    assert!(stdout.contains("\"\"\"Connect to the host.\"\"\""), "{stdout}");
    assert!(!stdout.contains("\"\"\"Inner"), "{stdout}");
}

// Signature reach follows the names a module exposes, so a class a re-exporting package exposes with a base class
// behaves the same as when the module itself exposes it.
#[test]
fn signature_reach_is_the_same_through_a_reexport() {
    let child = "class _Base:\n    def run(self) -> None: ...\n\n\nclass Child(_Base):\n    \"\"\"Child.\"\"\"\n\n\n__all__ = [\"Child\"]\n";
    let direct = project(&[("pyproject.toml", LIBRARY), ("pkg/core.py", child)]);
    let reexported = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", "from ._core import Child\n\n__all__ = [\"Child\"]\n"),
        ("pkg/_core.py", child),
    ]);
    let direct_reports = check(direct.path(), &["--select", "SMP903"]).1;
    let reexported_reports = check(reexported.path(), &["--select", "SMP903"]).1;
    assert_eq!(direct_reports.matches("Public method `_Base.run`").count(), 1, "{direct_reports}");
    assert_eq!(reexported_reports.matches("Public method `_Base.run`").count(), 1, "{reexported_reports}");
}

// A public signature reaches a class that is not listed in `__all__`, and the library's users get the
// base class of a dataclass it returns, so the dataclass keeps its options in both layouts.
#[test_case(false ; "defined_in_public_module")]
#[test_case(true ; "re-exported_from_private_module")]
fn signature_reached_dataclass_base_keeps_its_options_in_both_layouts(reexported: bool) {
    let core = "from dataclasses import dataclass\n\n\n@dataclass\nclass Base:\n    x: int\n\n\nclass Child(Base):\n    \"\"\"Child.\"\"\"\n\n\n@dataclass\nclass Hidden:\n    x: int\n\n\n__all__ = [\"Child\"]\n";
    let files: Vec<(&str, &str)> = if reexported {
        vec![
            ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
            ("pkg/__init__.py", "from ._core import Child\n\n__all__ = [\"Child\"]\n"),
            ("pkg/_core.py", core),
        ]
    } else {
        vec![("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"), ("pkg/core.py", core)]
    };
    let root = common::project(&files);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
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

// A class a listed function returns is library API, so its docstring and its method's docstring stay when SMP901's
// unsafe fix runs, while an internal helper's docstring is still turned into a comment.
#[test]
fn signature_reached_class_keeps_its_docstrings() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "class Config:\n    def describe(self) -> str:\n        \"\"\"Describe the settings.\"\"\"\n        return \"config\"\n\n\ndef load() -> Config:\n    \"\"\"Load.\"\"\"\n    return Config()\n\n\ndef _helper() -> None:\n    \"\"\"Internal.\"\"\"\n\n\n__all__ = [\"load\"]\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "SMP901");
    let fixed = common::read(root.path(), "pkg/core.py");
    assert!(fixed.contains("Describe the settings.") && fixed.contains("\"\"\"Load."), "{fixed}");
    assert!(!fixed.contains("Internal."), "{fixed}");
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
    DOCSTRINGED_PLUGIN,
    "SMP401",
    "All checks passed!\n" ; "bare_module_names_are_not_imports"
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

// A `from __future__` import binds a compiler flag that the host of a bare module entry point never reads, so it is no
// public name for `missing-from-all` to list.
#[test]
fn bare_module_future_import_is_not_public_api() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"pkg\"\nentry-points.pytest11.demo = \"pkg.plugin\"\n[build-system]\nrequires = []\n",
        ),
        ("pkg/__init__.py", ""),
        (
            "pkg/plugin.py",
            "\"\"\"Plugin.\"\"\"\n\nfrom __future__ import annotations\n\n__all__ = [\"pytest_addoption\"]\n\n\ndef pytest_addoption(parser: object) -> None:\n    \"\"\"Register the flag.\"\"\"\n    print(parser)\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, "All checks passed!\n");
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

#[test_case("import pkgutil\n\nHANDLER = pkgutil.resolve_name(\"pkg._cli._validate:run\")\n", true ; "direct_resolve_name")]
#[test_case("from pkgutil import resolve_name\n\nHANDLER = resolve_name(\"pkg._cli._validate:run\")\n", true ; "imported_resolve_name")]
#[test_case("def build_parser(parser):\n    parser.set_defaults(handler=\"pkg._cli._validate:run\")\n", true ; "argparse_handler")]
#[test_case("HANDLER = \"pkg._cli._validate:run\"\n", false ; "unresolved_string")]
#[test_case("def build_parser(parser):\n    parser.set_defaults(description=\"pkg._cli._validate:run\")\n", false ; "other_default")]
#[test_case("def build_parser(parser):\n    parser.set_defaults(handler=\"pkg._cli._validate:run!\")\n", false ; "invalid_attribute")]
#[test_case("import pkgutil\n\nHANDLER = pkgutil.resolve_name(\"pkg._cli._validate.run\")\n", true ; "dotted_resolve_name")]
fn string_named_handlers_count_as_imported(consumer: &str, imported: bool) {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_cli/__init__.py", ""),
        ("pkg/_cli/_validate.py", "def run() -> None: ...\n\n\n__all__ = [\"run\"]\n"),
        ("pkg/_cli/_parser.py", consumer),
    ]);
    let report = check(root.path(), &["--select", "SMP402"]).1;
    assert_eq!(report == "All checks passed!\n", imported, "{report}");
}

#[test]
fn string_named_handler_requires_an_export() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_cli/__init__.py", ""),
        ("pkg/_cli/_validate.py", "def run() -> None: ...\n"),
        (
            "pkg/_cli/_parser.py",
            "def build_parser(parser):\n    parser.set_defaults(handler=\"pkg._cli._validate:run\")\n",
        ),
    ]);
    assert!(check(root.path(), &["--select", "SMP401"]).1.contains("`run` is imported by other modules"));
}

#[test_case("[tool.setuptools.dynamic]\nversion = { attr = \"pkg._plugin.version\" }\n", "version" ; "setuptools_dynamic_attribute")]
#[test_case("[tool.setuptools.cmdclass]\nbuild_py = \"pkg._plugin.Build\"\n", "Build" ; "setuptools_command_class")]
#[test_case("[tool.mypy]\nplugins = [\"pkg._plugin\"]\n", "plugin" ; "mypy_plugin")]
#[test_case("[tool.mypy]\nplugins = [\"pkg._plugin:custom\"]\n", "custom" ; "mypy_custom_plugin")]
#[test_case("[tool.pylint.main]\nload-plugins = [\"pkg._plugin\"]\n", "register" ; "pylint_plugin")]
#[test_case("[tool.poetry.plugins.example]\nplugin = \"pkg._plugin:activate\"\n", "activate" ; "poetry_plugin")]
fn pyproject_named_attribute_counts_as_used(tool: &str, name: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}{tool}")),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", &format!("__all__ = [\"{name}\"]\n\n\ndef {name}() -> None: ...\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, "All checks passed!\n");
}

#[test_case("[tool.pytest.ini_options]\naddopts = \"-q -p pkg._plugin\"\n" ; "ini_options")]
#[test_case("[tool.pytest]\naddopts = [\"-q\", \"-p\", \"pkg._plugin\"]\n" ; "native_options")]
fn pyproject_pytest_plugin_counts_as_used(tool: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}{tool}")),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pytest_configure\"]\n\n\ndef pytest_configure() -> None: ...\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, "All checks passed!\n");
}

#[test]
fn disabled_pytest_plugin_does_not_count_as_used() {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}[tool.pytest.ini_options]\naddopts = \"-p no:pkg._plugin\"\n")),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pytest_configure\"]\n\n\ndef pytest_configure() -> None: ...\n"),
    ]);
    assert!(check(root.path(), &["--select", "SMP402"]).1.contains("`pytest_configure`"));
}

#[test_case("", "pdm_build.py" ; "default_hook")]
#[test_case("[tool.pdm.build]\ncustom-hook = \"build.py\"\n", "build.py" ; "custom_hook")]
fn pdm_build_hooks_count_as_used(config: &str, file: &str) {
    let root = project(&[
        (
            "pyproject.toml",
            &format!(
                "[project]\nname = \"pkg\"\n[build-system]\nbuild-backend = \"pdm.backend\"\nrequires = [\"pdm-backend\"]\n{config}"
            ),
        ),
        (file, "__all__ = [\"pdm_build_initialize\"]\n\n\ndef pdm_build_initialize(context: object) -> None: ...\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP402"]).1, "All checks passed!\n");
}

#[test]
fn missing_pdm_build_hook_names_no_function() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"pkg\"\n[build-system]\nbuild-backend = \"pdm.backend\"\nrequires = [\"pdm-backend\"]\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pdm_build_initialize\"]\n\n\ndef pdm_build_initialize() -> None: ...\n"),
    ]);
    assert!(check(root.path(), &["--select", "SMP402"]).1.contains("`pdm_build_initialize`"));
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

const BUILD_HOOK: &str = "from hatchling.builders.hooks.plugin.interface import BuildHookInterface\n\n\n\
                          class _Hook(BuildHookInterface):\n    pass\n";
const METADATA_HOOK: &str = "from hatchling.metadata.plugin.interface import MetadataHookInterface\n\n\n\
                             class _Hook(MetadataHookInterface):\n    pass\n";

// hatchling loads the class that subclasses its hook interface from the `path` of a custom hook, which defaults to
// `hatch_build.py`, so no import reaches the class.
#[test_case("[tool.hatch]\nbuild.hooks.custom = {}\n", "hatch_build.py", BUILD_HOOK ; "build_default_path")]
#[test_case("[tool.hatch.build.hooks.custom]\npath = \"build/hooks.py\"\n", "build/hooks.py", BUILD_HOOK ; "build_path")]
#[test_case("[tool.hatch.metadata.hooks.custom]\n", "hatch_build.py", METADATA_HOOK ; "metadata_default_path")]
#[test_case("[tool.hatch.metadata.hooks.custom]\npath = \"build/hooks.py\"\n", "build/hooks.py", METADATA_HOOK ; "metadata_path")]
#[test_case("[tool.hatch.build.targets.wheel.hooks.custom]\n", "hatch_build.py", BUILD_HOOK ; "target_default_path")]
#[test_case("[tool.hatch.build.targets.sdist.hooks.custom]\npath = \"build/hooks.py\"\n", "build/hooks.py", BUILD_HOOK ; "target_path")]
fn configured_hatch_hook_class_is_used(hatch: &str, file: &str, source: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\nversion = \"1\"\n{hatch}")),
        (file, source),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, "All checks passed!\n");
}

#[test_case("[tool.hatch]\nbuild.hooks.custom = { path = \"other.py\" }\n" ; "other_path")]
#[test_case("[tool.hatch]\nbuild.hooks.fancy-pypi-readme = {}\n" ; "other_hook")]
#[test_case("[tool.hatch.build.targets.wheel.hooks.custom]\npath = \"other.py\"\n" ; "other_target_path")]
#[test_case("" ; "unconfigured")]
fn unconfigured_hatch_hook_class_is_reported(hatch: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\nversion = \"1\"\n{hatch}")),
        ("hatch_build.py", BUILD_HOOK),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP409"]).1,
        "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\nFound 1 error.\n"
    );
}

// Only the hook class is read by hatchling, so another class of the file is still reported.
#[test]
fn configured_hatch_hook_file_keeps_its_other_classes_reported() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n[tool.hatch]\nbuild.hooks.custom = {}\n"),
        ("hatch_build.py", &format!("{BUILD_HOOK}\n\nclass _Helper:\n    pass\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP409"]).1,
        "hatch_build.py:8:7: SMP409 `_Helper` is never used in the run; remove it or give it a user\nFound 1 error.\n"
    );
}

// hatchling reads a `hatch.toml` next to the `pyproject.toml`, whose tables have no `tool.hatch` prefix.
#[test_case("[build.hooks.custom]\n", "hatch_build.py", BUILD_HOOK ; "build_default_path")]
#[test_case("[build.hooks.custom]\npath = \"build/hooks.py\"\n", "build/hooks.py", BUILD_HOOK ; "build_path")]
#[test_case("[metadata.hooks.custom]\n", "hatch_build.py", METADATA_HOOK ; "metadata_default_path")]
#[test_case("[build.targets.wheel.hooks.custom]\n", "hatch_build.py", BUILD_HOOK ; "target_default_path")]
fn hatch_toml_configures_the_hook(hatch: &str, file: &str, source: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        ("hatch.toml", hatch),
        (file, source),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, "All checks passed!\n");
}

// A `hatch.toml` that cannot be parsed configures no hook, with a warning, rather than stopping the run.
#[test]
fn unparsable_hatch_toml_configures_nothing() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"),
        ("hatch.toml", "[build\n"),
        ("hatch_build.py", BUILD_HOOK),
    ]);
    let (status, stdout, stderr) = check(root.path(), &["--select", "SMP409"]);
    assert_eq!(
        (status, stdout, stderr.contains("Failed to parse")),
        (
            1,
            "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\nFound 1 error.\n"
                .to_owned(),
            true
        )
    );
}

// A `hatch.toml` is merged over the `[tool.hatch]` table key by key, so a key it sets wins and one it leaves unset keeps
// the pyproject's value.
#[test_case("[tool.hatch.build.hooks.custom]\npath = \"other.py\"\n", "[build.hooks.custom]\npath = \"hatch_build.py\"\n", "All checks passed!\n" ; "hatch_toml_overrides_the_path")]
#[test_case(
    "[tool.hatch.build.hooks.custom]\npath = \"other.py\"\n",
    "[build.hooks.custom]\n",
    "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\nFound 1 error.\n" ;
    "hatch_toml_keeps_the_unset_path"
)]
#[test_case(
    "[tool.hatch.build.hooks.custom]\n",
    "[build.hooks.custom]\npath = \"other.py\"\n",
    "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\nFound 1 error.\n" ;
    "hatch_toml_moves_the_path"
)]
fn hatch_toml_is_merged_over_the_pyproject(pyproject: &str, hatch: &str, expected: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\nversion = \"1\"\n{pyproject}")),
        ("hatch.toml", hatch),
        ("hatch_build.py", BUILD_HOOK),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, expected);
}

// hatchling joins a hook's `path` to the project's directory and normalizes it, so `..` names the directory above.
#[test_case("build/../hooks/h.py" ; "parent_directory")]
#[test_case("./hooks/./h.py" ; "current_directory")]
fn hatch_hook_path_is_normalized(path: &str) {
    let root = project(&[
        (
            "pyproject.toml",
            &format!(
                "[project]\nname = \"demo\"\nversion = \"1\"\n[tool.hatch.build.hooks.custom]\npath = \"{path}\"\n"
            ),
        ),
        ("hooks/h.py", BUILD_HOOK),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, "All checks passed!\n");
}

// hatchling uses `issubclass` over the names of the hook file, so a class that subclasses a class of the run which
// subclasses the hook interface is the hook, while a base of the run that does not is not.
#[test_case("class CommonHook(BuildHookInterface):\n    pass\n", "All checks passed!\n" ; "inherited_from_hook_class")]
#[test_case(
    "class CommonHook:\n    pass\n",
    "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\nFound 1 error.\n" ;
    "inherited_from_plain_class"
)]
fn class_inheriting_a_hook_through_the_run_is_used(shared: &str, expected: &str) {
    let shared = format!("from hatchling.builders.hooks.plugin.interface import BuildHookInterface\n\n\n{shared}");
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n[tool.hatch.build.hooks.custom]\n"),
        ("shared_hooks.py", &shared),
        ("hatch_build.py", "from shared_hooks import CommonHook\n\n\nclass _Hook(CommonHook):\n    pass\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, expected);
}

const HOOK_PROJECT: &str = "def _run() -> None:\n    print(\"hook\")\n";

// The interpreter runs the `import` lines of a `.pth` file at startup, so a name they read is used outside the tests, as
// an entry point's is. A bare import reads no name, and a path line or a line with no `import` is not run.
#[test_case("import pkg.hook; pkg.hook._run()\n", "All checks passed!\n" ; "startup_read_keeps_the_name")]
#[test_case("import sys; from pkg.hook import _run; _run()\n", "All checks passed!\n" ; "startup_from_import_keeps_the_name")]
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

#[test]
fn separate_distributions_are_not_compared_for_duplicate_code() {
    assert_eq!(
        duplicate_code_reports(&[
            ("first/pyproject.toml", "[project]\nname = \"first\"\n"),
            ("first/src/first/a.py", REPEATED),
            ("first/src/first/b.py", REPEATED),
            ("second/pyproject.toml", "[project]\nname = \"second\"\n"),
            ("second/src/second/c.py", REPEATED),
        ]),
        ["first/src/first/a.py", "first/src/first/b.py"]
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

// A nested project rooted at a `test` directory is test input, as the directory is one, so its repeated statements are not
// compared with the rest of the run; a directory of the same layout that no test directory names is compared.
#[test_case("tools/swig/test", false ; "test_directory_is_not_compared")]
#[test_case("tools/swig/checks", true ; "other_directory_is_compared")]
fn nested_test_directory_repeats_are_not_compared(directory: &str, reported: bool) {
    let suite = "    suite.addTest(unittest.TestLoader().loadTestsFromTestCase(case))\n".repeat(9);
    let block = format!(
        "import unittest\n\n\nif __name__ == \"__main__\":\n    suite = unittest.TestSuite()\n{suite}    unittest.TextTestRunner(verbosity=2).run(suite)\n"
    );
    let mut files = vec![(format!("{directory}/setup.py"), String::new())];
    files.push((format!("{directory}/a.py"), block.clone()));
    files.push((format!("{directory}/b.py"), block));
    let files: Vec<(&str, &str)> = files.iter().map(|(name, content)| (name.as_str(), content.as_str())).collect();
    let stdout = check(project(&files).path(), &["--select", "SMP530"]).1;
    assert_eq!(stdout.contains(" SMP530 "), reported, "{stdout}");
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
        ("pyproject.toml", DEMO),
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

// A nested project is indexed on its own, so the run's stores to an attribute reach its names as its reads do: a
// `missing-final` there leaves a name the run assigns alone.
#[test_case("        self.ffi = object()\n", false ; "outer_store_assigns_the_name")]
#[test_case("        pass\n", true ; "without_a_store_the_name_is_reported")]
fn nested_project_sees_the_attributes_the_run_stores_to(store: &str, reported: bool) {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/owner.py", &format!("class Owner:\n    def __init__(self) -> None:\n{store}")),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/snip.py", "ffi = object()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP601"]).1;
    assert_eq!(stdout.contains(" SMP601 "), reported, "{stdout}");
}

// A nested project's `pyproject.toml` that does not parse is an error, as the one of the project around it is.
#[test_case("[project\n" ; "invalid_toml")]
#[test_case("[project]\nname = 1\n" ; "invalid_metadata")]
fn invalid_pyproject_of_a_nested_project_is_an_error(content: &str) {
    let root = project(&[
        ("pyproject.toml", DEMO),
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
        ("pyproject.toml", DEMO),
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
        ("pyproject.toml", DEMO),
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
    let root =
        project(&[("pyproject.toml", DEMO), ("pkg/__init__.py", ""), ("pkg/api.py", source), ("index.rst", directive)]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), expected, "{stdout}");
}

// The methods of an `autoclass` with `:members:` are documented, as the class's own name is.
#[test]
fn autoclass_members_are_used() {
    let root = project(&[
        ("pyproject.toml", DEMO),
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

// `automodule` of a package documents the names its `__init__.py` defines, as it documents a submodule's.
#[test]
fn automodule_of_a_package_documents_its_init() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", FUNCTIONS),
        ("index.rst", ".. automodule:: pkg\n   :members:\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), &["_hidden"], "{stdout}");
}

// An `automodule` names the module its target ends with at a dot, so `api` does not name `pkg.myapi`.
#[test]
fn automodule_does_not_match_a_module_whose_name_only_ends_with_the_target() {
    let root = project(&[
        ("pyproject.toml", DEMO),
        ("pkg/__init__.py", ""),
        ("pkg/myapi.py", FUNCTIONS),
        ("index.rst", ".. automodule:: api\n   :members:\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(unused_names(&stdout), &["_hidden", "other", "public"], "{stdout}");
}

// A function an `automodule` with `:members:` documents is public API, so its docstring stays; without the directive it
// is internal code, whose docstring is reported.
#[test_case(Some(".. automodule:: pkg.api\n   :members:\n"), false ; "documented")]
#[test_case(None, true ; "undocumented")]
fn documented_public_function_keeps_its_docstring(page: Option<&str>, reported: bool) {
    let mut files = vec![
        ("pyproject.toml", DEMO),
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
        ("pyproject.toml", DEMO),
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
    let stdout = check(root.path(), &["--select", "SMP9,SMP102,SMP401,SMP402,SMP403,SMP404"]).1;
    assert_eq!(stdout, "All checks passed!\n");
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

// A store to one attribute of a module from another file, or a dotted string patching one, touches only that attribute:
// the module's other attributes still move into the importer.
#[test_case(
    "pkg/setup.py",
    "from pkg import exc\n\nexc._version_token = 1\n",
    "pkg/use.py:5:12: SMP304 Import `Error` from `pkg.exc` instead of reaching it through the module\npkg/use.py:5:23: \
     SMP304 Import `Other` from `pkg.exc` instead of reaching it through the module\nFound 2 errors.\n2 unsafe fixes \
     available with `--fix --unsafe-fixes`.\n" ; "store"
)]
#[test_case(
    "pkg/test_use.py",
    "from unittest import mock\n\n\ndef test_it() -> None:\n    with mock.patch(\"pkg.exc.Error\"):\n        pass\n",
    "pkg/use.py:5:23: SMP304 Import `Other` from `pkg.exc` instead of reaching it through the module\nFound 1 error.\n" ; "patched"
)]
fn stored_attribute_leaves_other_attributes_reported(path: &str, body: &str, expected: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/exc.py", "class Error(Exception):\n    pass\n\n\nclass Other(Exception):\n    pass\n"),
        ("pkg/use.py", "from pkg import exc\n\n\ndef check() -> object:\n    return exc.Error, exc.Other\n"),
        (path, body),
    ]);
    let (_, stdout, _) = check(root.path(), &["--select", "SMP304"]);
    assert_eq!(stdout, common::native(expected));
}

// A doctest reads a module's name, which is no patch of it, so the module's own reads still get the import fix.
#[test]
fn doctest_read_does_not_patch_module() {
    let root = project(&[
        ("tools/bench/__init__.py", ""),
        (
            "tools/bench/operations.py",
            "OPERATIONS = {}\n\n\ndef run() -> dict:\n    \"\"\"Return the table.\n\n    >>> from bench import operations\n    >>> operations.OPERATIONS\n    {}\n    \"\"\"\n    return OPERATIONS\n",
        ),
        (
            "tools/bench/report.py",
            "from bench import operations\n\n\ndef render() -> dict:\n    return operations.OPERATIONS\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP304"]).1,
        common::native(
            "tools/bench/report.py:5:12: SMP304 Import `OPERATIONS` from `bench.operations` instead of reaching it through \
             the module\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A module no file of the run resolves, such as `distutils.log` for a tree that shims its own `log`, reads the run's file of
// the same last name when the call's value is used, and only then, so the function returning that value is not reported.
#[test_case("from distutils import log\n\n\ndef run() -> int:\n    saved = log.set_threshold(1)\n    return saved\n", false ; "value_used_through_namesake")]
#[test_case("from distutils import log\n\n\ndef run() -> None:\n    log.set_threshold(1)\n", true ; "value_dropped_through_namesake")]
#[test_case("from elsewhere import tools\n\n\ndef run() -> int:\n    return tools.set_threshold(1)\n", true ; "unrelated_module_reads_nothing")]
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
#[test_case("    \"\"\"A release.\n\n    >>> _Trimmed(\"1.0.0\").release\n    '1'\n    \"\"\"\n", false ; "doctest_names_it")]
#[test_case("    \"\"\"A release.\"\"\"\n", true ; "docstring_only_describes_it")]
fn class_named_by_its_own_module_doctest_is_used(docstring: &str, reported: bool) {
    let source = format!(
        "class _Trimmed:\n{docstring}\n    def __init__(self, release: str) -> None:\n        self.release = release.split(\".\")[0]\n"
    );
    let root = project(&[("pkg/version.py", source.as_str())]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    assert_eq!(stdout.contains(&common::native("pkg/version.py:1:7: SMP409 `_Trimmed`")), reported, "{stdout}");
}

// A doctest reads a name off a module, which the call-site rules must see as a use, yet it imports nothing for the other
// modules: a function only a doctest calls is not reported as unread, and a name a doctest reads is not imported by
// the others.
#[test]
fn doctest_reads_are_opaque_to_call_sites_but_not_imports() {
    let root = project(&[
        ("q/__init__.py", ""),
        (
            "q/calc.py",
            "def total(value):\n    ratio(value)\n    return value\n\n\ndef ratio(value):\n    return value / 2\n",
        ),
        ("q/guide.py", "\"\"\"Guide.\n\n>>> import q.calc\n>>> q.calc.ratio(3)\n1.5\n\"\"\"\n"),
        ("q/main.py", "from q.calc import total\n\nprint(total(1))\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP411"]).1, "All checks passed!\n");
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
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP409"]).1, "All checks passed!\n");
}

// A constant a doctest of its module reads is kept, as inlining it would leave the example reading a name that is gone.
#[test]
fn constant_read_by_its_own_module_doctest_is_kept() {
    let root = project(&[(
        "pkg/consts.py",
        "_TIMEOUT = 30\n\n\ndef wait():\n    \"\"\"Wait.\n\n    >>> _TIMEOUT\n    30\n    \"\"\"\n    return _TIMEOUT\n",
    )]);
    assert_eq!(check(root.path(), &["--select", "SMP202"]).1, "All checks passed!\n");
}

// The stdlib's `json` is a module of its own, so a `json.loads` value a file of the run reads is not a read of the run's
// `codec/json.py`, and a `json.dumps` call with its own `indent` is not a call of the run's `dumps`.
#[test]
fn stdlib_module_reads_are_not_namesake_reads() {
    let root = project(&[
        ("app/__init__.py", ""),
        ("app/codec/__init__.py", ""),
        (
            "app/codec/json.py",
            "def loads(text):\n    return text.strip()\n\n\ndef dumps(obj, indent=None):\n    return f\"{obj}{indent}\"\n",
        ),
        ("app/web.py", "import json\n\n\ndef handle(body):\n    return json.loads(body), json.dumps(body, indent=2)\n"),
        ("app/main.py", "from app.codec import json\n\n\ndef run():\n    json.loads(\" x \")\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP411,SMP506"]).1;
    assert!(stdout.contains(&common::native("app/codec/json.py:1:5: SMP411")), "{stdout}");
    assert!(!stdout.contains("SMP506"), "{stdout}");
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
        ("pyproject.toml", DEMO),
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
        ("pyproject.toml", DEMO_BUILD),
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

// An autodoc directive names a package's function by the package, so the `__init__.py` function it documents is public
// documentation rather than internal code.
#[test]
fn autodoc_names_a_package_function_by_its_package() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        (
            "app/__init__.py",
            "def helper() -> int:\n    \"\"\"Compute the helper value from the environment and return it to the caller.\"\"\"\n    return 2\n",
        ),
        ("docs/conf.py", ""),
        ("docs/index.rst", "API\n===\n\n.. autofunction:: app.helper\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP901,SMP409"]).1;
    assert_eq!(stdout, "All checks passed!\n");
}

// An autodoc directive names a method by its class, so the method's docstring is documented rather than internal.
#[test]
fn autodoc_names_a_method_by_its_class() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\nversion = \"0\"\n"),
        ("app/__init__.py", ""),
        (
            "app/mod.py",
            "__all__ = [\"Foo\"]\n\n\nclass Foo:\n    def run(self) -> int:\n        \"\"\"Run the thing and give back the exit status of the subprocess it starts.\"\"\"\n        return 1\n",
        ),
        ("docs/conf.py", ""),
        ("docs/index.rst", "API\n===\n\n.. autoclass:: app.mod.Foo\n   :members:\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP901"]).1;
    assert_eq!(stdout, "All checks passed!\n");
}

// A nested project's files are test input, so the rules it leaves out do not run there. Its results are cached apart
// from the outer project's, which judges the same file.
#[test]
fn nested_project_results_are_not_reused_by_the_outer_project() {
    let docstring = format!("def run() -> None:\n    \"\"\"{}\"\"\"\n", "word ".repeat(150).trim_end());
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"outer\"\nversion = \"0\"\n"),
        ("tests/proj/pyproject.toml", "[project]\nname = \"proj\"\nversion = \"0\"\n"),
        ("tests/proj/mod.py", &docstring),
    ]);
    let cache = root.path().join("cache");
    let cache = cache.to_str().expect("a UTF-8 path");
    let nested = root.path().join("tests/proj");
    let (_, inner, _) =
        common::simplint(&nested, &["check", "--cache-dir", cache, "--select", "SMP904", "--output-format", "concise"]);
    assert!(inner.contains("SMP904"), "{inner}");
    let (_, outer, _) = common::simplint(
        root.path(),
        &["check", "--cache-dir", cache, "--select", "SMP904", "--output-format", "concise"],
    );
    assert_eq!(outer, "All checks passed!\n");
}

const COVERAGE_PLUGIN: &str = "def coverage_init(reg: object, options: dict[str, str]) -> None:\n    pass\n\n\n";

// coverage loads a plugin module by its dotted name and calls its `coverage_init`, so no import reaches the function.
#[test_case("__all__ = [\"coverage_init\"]\n" ; "listed")]
#[test_case("__all__ = []\n" ; "unlisted")]
fn coverage_plugin_init_is_used(all: &str) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}{all}")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP402,SMP403,SMP409"]).1, "All checks passed!\n");
}

// Only `coverage_init` is called by coverage, so another function of the plugin module is still reported.
#[test]
fn coverage_plugin_keeps_its_other_functions_reported() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}def helper() -> None:\n    pass\n\n\n__all__ = []\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP409"]).1,
        common::native(
            "tasks/coverage_pragmas.py:5:5: SMP409 `helper` is never used in the run; remove it or give it a caller\n\
             Found 1 error.\n"
        )
    );
}

// A module coverage does not load as a plugin is judged as any other module, so its `coverage_init` is reported.
#[test]
fn coverage_plugin_not_listed_is_reported() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.other\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403,SMP409"]).1,
        common::native(
            "tasks/coverage_pragmas.py:1:5: SMP403 `coverage_init` is not in `__all__`; name it `_coverage_init`\n\
             tasks/coverage_pragmas.py:1:5: SMP409 `coverage_init` is never used in the run; remove it or give it a \
             caller\n\
             Found 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A plugin resolves from the directories pytest's `pythonpath` and ty's `extra-paths` put on the project's import path.
#[test_case("[tool.pytest.ini_options]\npythonpath = [\"tasks\"]\n" ; "pytest_ini_options")]
#[test_case("[tool.pytest]\npythonpath = [\"tasks\"]\n" ; "pytest")]
#[test_case("[tool.ty.environment]\nextra-paths = [\"tasks\"]\n" ; "ty")]
#[test_case("[tool.ty.environment]\nextra-paths = [\"typestubs\", \"tasks\"]\n" ; "ty_with_a_missing_directory")]
fn coverage_plugin_resolves_from_import_paths(paths: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"cov\"]\n{paths}")),
        ("tasks/cov.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP402,SMP403,SMP409"]).1, "All checks passed!\n");
}

// Without an import path naming its directory, coverage's plugin module is not found, so it is judged as any module.
#[test]
fn coverage_plugin_outside_its_import_paths_is_reported() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"cov\"]\n"),
        ("tasks/cov.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP403,SMP409"]).1,
        common::native(
            "tasks/cov.py:1:5: SMP403 `coverage_init` is not in `__all__`; name it `_coverage_init`\n\
             tasks/cov.py:1:5: SMP409 `coverage_init` is never used in the run; remove it or give it a caller\n\
             Found 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A test module that imports a plugin's `coverage_init` uses it outside the tests, as coverage does, so it is not
// reported as test-only code.
#[test]
fn coverage_plugin_init_imported_by_a_test_is_not_test_only() {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[tool.coverage.run]\nplugins = [\"tasks.coverage_pragmas\"]\n"),
        ("tasks/__init__.py", ""),
        ("tasks/coverage_pragmas.py", &format!("{COVERAGE_PLUGIN}__all__ = []\n")),
        (
            "tests/test_coverage.py",
            "from tasks.coverage_pragmas import coverage_init\n\n\n\
             def test_init() -> None:\n    coverage_init(object(), {})\n",
        ),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, "All checks passed!\n");
}
