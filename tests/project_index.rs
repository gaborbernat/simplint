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

// Another file's writes count by the class of their receiver: a mock's never, an instance's or an unknown object's
// always.
#[test_case("mock = MagicMock()\n    mock.log = log\n", "[*] ", "@dataclass(frozen=True, slots=True)" ; "mock")]
#[test_case("result = Result(log=\"\")\n    result.log = log\n", "[*] ", "@dataclass(slots=True)" ; "instance")]
#[test_case("thing = make()\n    thing.log = log\n", "[*] ", "@dataclass(slots=True)" ; "unknown")]
#[test_case("results: dict[str, Result] = {}\n    results[\"a\"].log = log\n", "[*] ", "@dataclass(slots=True)" ; "subscript")]
#[test_case(
    "pass\n\n\nclass Result:\n    def __init__(self) -> None:\n        self.extra = 1\n",
    "",
    "@dataclass(frozen=True, slots=True)"
    ; "namesake"
)]
#[test_case(
    "import pickle\n\n    result = Result(log=\"\")\n    result.log = log\n    pickle.dumps(result, 1)\n", "", "@dataclass" ; "low_protocol_pickle"
)]
#[test_case(
    "import pickle\n\n    result = Result(log=\"\")\n    result.log = log\n    pickle.dumps(result, protocol=pickle.HIGHEST_PROTOCOL)\n",
    "[*] ",
    "@dataclass(slots=True)"
    ; "highest_protocol_pickle"
)]
fn dataclass_options_reads_writes_in_other_files(body: &str, marker: &str, decorator: &str) {
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
    let select = ["--no-cache", "--select", "SMP111", "--target-version", "py314"];
    let report = check(root.path(), &select).1;
    check(root.path(), &[&select[..], &["--fix", "--unsafe-fixes"]].concat());
    assert_eq!(
        (report.lines().next(), common::read(root.path(), "models.py").lines().nth(3)),
        (
            Some(
                common::native(&format!(
                    "models.py:4:2: SMP111 {marker}Dataclass is missing frozen, slots; set `frozen=True, slots=True`"
                ))
                .as_str()
            ),
            Some(decorator)
        )
    );
}

// A package's `__init__.py` defines the package, so a write on an instance of a class it imports from there counts.
#[test]
fn dataclass_options_reads_writes_on_package_classes() {
    let root = project(&[
        ("pkg/__init__.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Context:\n    count: int\n"),
        ("pkg/run.py", "from pkg import Context\n\n\ndef run(context: Context) -> None:\n    context.count = 1\n"),
    ]);
    check(root.path(), &["--no-cache", "--select", "SMP111", "--target-version", "py314", "--fix", "--unsafe-fixes"]);
    assert_eq!(common::read(root.path(), "pkg/__init__.py").lines().nth(3), Some("@dataclass(slots=True)"));
}

const PARTITION: &str =
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Partition:\n    topic: str\n    number: int\n";

// Each file rewrites its own positional calls in the same run that gives the decorator `kw_only=True`, so a second
// run finds nothing.
#[test_case(
    "from models import Partition, helper\n\nhelper(1)\nPartition(\"a\", 1)\n", "Partition(topic=\"a\", number=1)" ; "imported"
)]
#[test_case("import models\n\nmodels.Partition(\"a\", 1)\n", "models.Partition(topic=\"a\", number=1)" ; "module")]
#[test_case(
    "from models import Partition\n\nPartition(\"a\", 1)  # noqa: E501\n", "Partition(topic=\"a\", number=1)  # noqa: E501" ; "other_code_suppressed"
)]
#[test_case(
    "from models import Partition\n\nPartition(\"noqa\", 1)\n", "Partition(topic=\"noqa\", number=1)" ; "noqa_in_string"
)]
#[test_case("from models import Partition as P\n\nP(\"a\", 1)\n", "P(topic=\"a\", number=1)" ; "alias")]
#[test_case(
    "from models import Partition\n\nrow = {\"number\": 1}\nPartition(\"a\", **row)\n", "Partition(topic=\"a\", **row)" ; "keyword_unpacking"
)]
fn dataclass_kw_only_rewrites_calls_in_other_files(caller: &str, rewritten: &str) {
    let models = format!("{PARTITION}\n\ndef helper(value):\n    return value\n");
    let root = project(&[("models.py", &models), ("caller.py", caller)]);
    let fix = ["--select", "SMP114", "--fix", "--unsafe-fixes", "--target-version", "py314"];
    let first = check(root.path(), &fix).1;
    let second = check(root.path(), &[&fix[..], &["--diff"]].concat()).0;
    assert_eq!(
        (
            first,
            common::read(root.path(), "caller.py").lines().last().map(str::to_owned),
            common::read(root.path(), "models.py").lines().nth(3).map(str::to_owned),
            second,
        ),
        (
            "Found 2 errors (2 fixed, 0 remaining).\n".to_owned(),
            Some(rewritten.to_owned()),
            Some("@dataclass(kw_only=True)".to_owned()),
            0
        )
    );
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
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa: SMP114\n")] ; "suppressed")]
#[test_case(&[("caller.py", "from models import Partition\n\nPartition(\"a\", 1)  # noqa\n")] ; "suppressed_blanket")]
#[test_case(
    &[("caller.py", "from models import Partition\n\nPartition(\n    \"a\",  # simplint: ignore[SMP114] paper\n    1,\n)\n")]
    ; "ignored_inside"
)]
#[test_case(
    &[("caller.py", "import importlib\n\nloaded = importlib.import_module(\"models\")\nloaded.Partition(\"a\", 1)\n")]
    ; "loaded_module"
)]
#[test_case(
    &[
        ("pyproject.toml", "[tool.simplint]\nper-file-ignores = { \"caller.py\" = [\"SMP114\"] }\n"),
        ("caller.py", "from models import Partition\n\nPartition(\"a\", 1)\n"),
    ]
    ; "ignored_in_caller"
)]
fn dataclass_kw_only_withholds_fix_for_unfollowed_uses(callers: &[(&str, &str)]) {
    let root = project(&[&[("models.py", PARTITION)], callers].concat());
    let fix = ["--no-cache", "--select", "SMP114", "--fix", "--unsafe-fixes", "--target-version", "py314"];
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
    assert_eq!(check(root.path(), &["--select", "SMP300"]).1, "All checks passed!\n");
}

#[test]
fn module_docstring_read_is_no_docstring_reader() {
    let root = project(&[
        ("pkg/__init__.py", ""),
        ("pkg/helpers.py", "def _helper() -> None:\n    \"\"\"Help.\"\"\"\n"),
        ("pkg/reader.py", "print(__doc__)\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP300"]).1.lines().next(),
        Some(
            common::native(
                "pkg/helpers.py:2:5: SMP300 Function docstring on internal code; let the name, signature and types \
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
        check(root.path(), &["--select", "SMP103,SMP113"]).1,
        common::native(
            "src/pkg/config.py:8:7: SMP113 [*] `Attribute` is imported by other modules; list it in `__all__`\nFound \
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
        check(root.path(), &["--select", "SMP102"]).1,
        common::native(
            "docs/tutorial.py:1:1: SMP102 Module-scope variable `client` should be named `CLIENT`\n\
             docs/tutorial.py:2:1: SMP102 Module-scope variable `engine` should be named `_ENGINE`\nFound 2 errors.\n1 \
             unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
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
        check(root.path(), &["--select", "SMP113"]).1,
        common::native(
            "pkg/backends.py:3:1: SMP113 [*] `info` is imported by other modules; list it in `__all__`\nFound 1 \
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
        check(root.path(), &["--select", "SMP113"]).1,
        common::native(
            "other/backends.py:2:1: SMP113 [*] `backends` is imported by other modules; list it in `__all__`\nFound 1 \
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
        check(root.path(), &["--select", "SMP103"]).1,
        common::native(
            "pkg/sub/other.py:8:7: SMP103 `ViewBase` is not in `__all__`; name it `_ViewBase`\nFound 1 error.\n1 unsafe \
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
        check(root.path(), &["--select", "SMP300,SMP302"]).1,
        "tool/cli.py:2:5: SMP300 Function docstring on internal code; let the name, signature and types document it\n\
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
        ("lib.py", "def run() -> None: ...\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP302"]).1,
        "lib.py:1:5: SMP302 Public function `run` has no docstring; document it for its users\nFound 1 error.\n"
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
    let reports = check(root.path(), &["--select", "SMP302"]).1;
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
        check(root.path(), &["--select", "SMP302"]).1,
        "pkg/_core.py:1:5: SMP302 Public function `run` has no docstring; document it for its users; \
         `pkg/__init__.py` re-exports it\nFound 1 error.\n"
            .replace("pkg/", &format!("pkg{}", std::path::MAIN_SEPARATOR_STR))
    );
}

// A module read from standard input has no index, so its own `__all__` tells what it offers.
#[test]
fn standard_input_reads_its_own_exports() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP302", "--stdin-filename", "pkg/mod.py", "-"],
        "def run() -> None: ...\n\n\ndef helper() -> None: ...\n\n\n__all__ = [\"run\"]\n",
    )
    .1;
    assert_eq!(
        stdout,
        format!(
            "pkg{}mod.py:1:5: SMP302 Public function `run` has no docstring; document it for its users\nFound 1 error.\n",
            std::path::MAIN_SEPARATOR_STR
        )
    );
}

// A module-level `__getattr__` supplies names the index cannot list, so only what the module defines counts.
#[test]
fn lazy_module_offers_what_it_defines() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/__init__.py",
            "def __getattr__(name: str) -> int:\n    return len(name)\n\n\ndef helper() -> None: ...\n",
        ),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP302"]).1,
        format!(
            "pkg{}__init__.py:5:5: SMP302 Public function `helper` has no docstring; document it for its users\n\
             Found 1 error.\n",
            std::path::MAIN_SEPARATOR_STR
        )
    );
}

// `api-modules` names a module users import from directly, so its `__all__` stays as written.
#[test_case(
    "",
    "pkg/errors.py:4:12: SMP117 `Oops` is in `__all__`, yet no other module imports it; list only what other modules \
     use\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "unused_export_reported"
)]
#[test_case("[tool.simplint]\napi-modules = [\"pkg.errors\"]\n", "All checks passed!\n" ; "declared_api_module_kept")]
fn api_modules_keep_their_exports(settings: &str, stdout: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}{settings}")),
        ("pkg/__init__.py", "\"\"\"Package.\"\"\"\n"),
        ("pkg/errors.py", "class Oops(Exception): ...\n\n\n__all__ = [\"Oops\"]\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP117"]).1,
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
    check(root.path(), &["--select", "SMP117", "--fix"]);
    assert_eq!(common::read(root.path(), "tool/cli.py"), "def run() -> None: ...\n\n\n__all__ = []\n");
}

// A namespace package's API package is its dotted name, so its modules are public below the namespace.
#[test_case("[tool.uv.build-backend]\nmodule-name = \"acme.widgets\"\n" ; "build_backend")]
#[test_case("[tool.simplint]\napi-packages = [\"acme.widgets\"]\n" ; "setting")]
fn namespace_package_is_public_api(settings: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"acme-widgets\"\n[build-system]\nrequires = []\n{settings}")),
        ("src/acme/widgets/__init__.py", ""),
        ("src/acme/widgets/shapes.py", "def circle() -> int: ...\n"),
        ("src/acme/other.py", "def square() -> int: ...\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP302"]).1,
        format!(
            "{}:1:5: SMP302 Public function `circle` has no docstring; document it for its users\nFound 1 error.\n",
            ["src", "acme", "widgets", "shapes.py"].join(std::path::MAIN_SEPARATOR_STR)
        )
    );
}

// `__main__` runs as a script, so it offers no API, while another dunder module, as `__about__`, does.
#[test]
fn main_module_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/__main__.py", "def main() -> None: ...\n"),
        ("pkg/__about__.py", "def helper() -> None: ...\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP302"]).1,
        format!(
            "pkg{}__about__.py:1:5: SMP302 Public function `helper` has no docstring; document it for its users\n\
             Found 1 error.\n",
            std::path::MAIN_SEPARATOR_STR
        )
    );
}
