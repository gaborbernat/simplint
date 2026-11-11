use super::*;
use crate::common::{check, project};
use test_case::test_case;

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

// A test imports a module under `src` by its dotted path, which lists no export, whether the package has an
// `__init__.py` or is a namespace package with none at any level.
#[test_case(
    &[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from pkg.helpers import helper\n"),
    ]
    ; "regular_package"
)]
#[test_case(
    &[
        ("src/outer/inner/helpers.py", "def helper() -> None:\n    pass\n"),
        ("tests/test_helpers.py", "from outer.inner.helpers import helper\n"),
    ]
    ; "namespace_package"
)]
fn test_imports_under_src_do_not_require_exports(files: &[(&str, &str)]) {
    let root = project(files);
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, common::PASSED);
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
    concat!(
        "[build-system]\n",
        "requires = [\"setuptools\"]\n",
        "build-backend = \"setuptools.build_meta\"\n",
        "[project]\n",
        "scripts = { tool = \"cli:run\" }\n",
        "[tool.setuptools]\n",
        "py-modules = [\"cli\"]\n",
        "[tool.ruff]\n",
        "src = [\"src\"]\n"
    ) ; "flat_layout_py_modules"
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
            "{name}:4:5: SMP401 [*] `run` is imported by other modules; list it in `__all__`\nFound 1 error.\n[*] 1 \
            fixable \
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
    assert_eq!(check(root.path(), &["--select", "SMP401,SMP403,SMP409"]).1, common::PASSED);
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
    let tomllib = "import sys\n\nif sys.version_info >= (3, 11):\n    from tomllib import TOMLDecodeError, load, \
    loads\n\
                   else:\n    from tomli import TOMLDecodeError, load, loads\n\n__all__ = [\"TOMLDecodeError\", \
                   \"load\", \
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
            "pkg/_compat/tomllib.py:8:12: SMP402 `TOMLDecodeError` is in `__all__`, yet no other module imports it; \
            list \
             only what other modules use\npkg/_compat/tomllib.py:8:39: SMP402 `loads` is in `__all__`, yet no other \
             module \
             imports it; list only what other modules use\nFound 2 errors.\n"
        )
    );
}

// A doctest in a module that shares its package's name reads `tz.gettz`, which names the module it is in. That name is
// no patch of the package's `__init__.py`, so `tz.tzstr` in another module of the run is still an import to offer.
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
            "pkg/parser/_parser.py:5:12: SMP304 Import `tzstr` from `..tz` instead of reaching it through the \
            module\nFound \
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
    assert_eq!(check(root.path(), &["--select", "SMP304"]).1, common::PASSED);
}

#[test_case("backend.pyi", common::PASSED ; "stub_of_the_module")]
#[test_case(
    "other.pyi",
    concat!(
        "backend.py:10:7: SMP403 `Missing` is not in `__all__`; name it `_Missing`\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "stub_of_another_module"
)]
fn stub_declarations_are_public_api(stub: &str, stdout: &str) {
    let root = project(&[
        (
            "backend.py",
            "__all__ = [\"run\"]\nLIMIT = 1\nNAME: str = \"x\"\n\n\ndef run() -> None:\n    pass\n\n\nclass \
            Missing(TypeError):\n    pass\n",
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
    assert_eq!(check(root.path(), &["--select", "SMP401"]).1, common::PASSED);
}

#[test_case("pkg.manager._load", common::PASSED ; "patched")]
#[test_case(
    "pkg.manager.run",
    concat!(
        "pkg/manager.py:1:5: SMP501 `_load` only forwards to `import_module`; call `import_module` at ",
        "its call sites instead\n",
        "Found 1 error.\n"
    ) ; "not_patched"
)]
fn patched_forwarding_function_is_a_seam(target: &str, stdout: &str) {
    let root = project(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/manager.py",
            "def _load(name: str) -> str:\n    return import_module(name)\n\n\ndef run() -> str:\n    return \
            _load(\"x\")\n",
        ),
        ("test_manager.py", &format!("def test_run(mocker):\n    mocker.patch(\"{target}\")\n")),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP501"]).1, common::native(stdout));
}

// `import *` takes the names `__all__` lists, or without one every public name, so those may be in use unnamed.
#[test_case("", common::PASSED ; "every_public_name")]
#[test_case(
    "__all__ = [\"run\"]\n",
    concat!(
        "pkg/data.py:2:1: SMP404 `TAG` is not in `__all__`; name it `_TAG`\n",
        "Found 1 error.\n",
        "1 unsafe fix available with `--fix --unsafe-fixes`.\n"
    ) ; "listed_names"
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
    let report = "from pathlib import Path\n\nTABLE_JSON_DIR: Path | None = None\n\n\ndef render() -> bool:\n    \
    return \
                  TABLE_JSON_DIR is not None\n";
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/report.py", report),
        ("pkg/cli.py", &format!("from pathlib import Path\n\nfrom pkg import report\n\n{store}")),
    ]);
    let (_, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP202,SMP404"]);
    assert_eq!((stdout, common::read(root.path(), "pkg/report.py")), (common::PASSED.to_owned(), report.to_owned()));
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
            "support/other.py:1:1: SMP404 `region` is not in `__all__`; name it `_region`\nFound 1 error.\n1 unsafe \
            fix \
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
    concat!(
        "import pickle\n",
        "\n",
        "    result = Result(log=\"\")\n",
        "    result.log = log\n",
        "    pickle.dumps(result, 1)\n"
    ),
    "",
    "@dataclass" ; "low_protocol_pickle"
)]
#[test_case(
    concat!(
        "from pickle import dumps as write\n",
        "\n",
        "    result = Result(log=\"\")\n",
        "    result.log = log\n",
        "    write(result, 1)\n"
    ),
    "",
    "@dataclass" ; "low_protocol_aliased_pickle"
)]
#[test_case(
    concat!(
        "import pickle\n",
        "\n",
        "    result = Result(log=\"\")\n",
        "    result.log = log\n",
        "    pickle.dumps(result, protocol=pickle.HIGHEST_PROTOCOL)\n"
    ),
    "",
    "@dataclass" ; "highest_protocol_pickle"
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
