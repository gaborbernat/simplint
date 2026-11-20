use super::*;
use crate::common::{check, check_with_stdin, native, project};
use test_case::test_case;

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
        "_lib.py:1:5: SMP903 Public function `run` has no docstring; document it for its users; `lib.py` re-exports \
        it\n\
         Found 1 error.\n"
    );
}

#[test]
fn reexports_in_top_level_blocks_count() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/__init__.py",
            "import sys\n\ntry:\n    from ._fast import run\nexcept ImportError:\n    from ._slow import \
            run\nelse:\n    \
             pass\nfinally:\n    pass\nif sys.platform == \"win32\":\n    from ._windows import path\nelif \
             sys.platform == \
             \"linux\":\n    from ._linux import path\nelse:\n    from ._posix import path\n\n__all__ = [\"path\", \
             \"run\"]\n",
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

// Users can call inherited methods through the re-exported subclass.
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

// A public function can return a private class whose inherited methods are available to callers.
#[test]
fn inherited_methods_of_signature_reached_class_are_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "class _Base:\n    def run(self) -> None: ...\n\n\nclass _Config(_Base):\n    \
            \"\"\"Config.\"\"\"\n\n\n__all__ = [\"load\"]\n\n\ndef load() -> _Config:\n    \"\"\"Load.\"\"\"\n    \
            return _Config()\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("Public method `_Base.run`").count(), 1, "{reports}");
}

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
    assert_eq!((reports.matches("`undocumented`").count(), reports.contains("`helper`")), (1, false), "{reports}");
}

#[test]
fn name_read_off_lazy_module_is_not_public_api() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", ""),
        ("pkg/_compat.py", "def f() -> None: ...\n"),
        (
            "pkg/utils.py",
            "from ._compat import f\n\n\ndef local() -> None: ...\n\n\ndef __getattr__(name: str) -> object:\n    \
            raise AttributeError(name)\n",
        ),
        ("pkg/testing.py", "from pkg import utils\n\nold = utils.f\n"),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert!(reports.contains("`local`") && !reports.contains("`f`"), "{reports}");
}

// An inherited method is visible through the public subclass even when the base class needs no documentation of its
// own.
#[test]
fn signature_reached_base_needs_no_docstring_but_its_inherited_methods_do() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "_StrPath = str\n\n\nclass _Base:\n    def run(self, path: _StrPath) -> None: ...\n\n\nclass \
            Child(_Base):\n    \"\"\"Child.\"\"\"\n\n\n__all__ = [\"Child\"]\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(
        (
            reports.matches("Public method `_Base.run`").count(),
            !reports.contains("`_Base`") && !reports.contains("`_StrPath`")
        ),
        (1, true),
        "{reports}"
    );
}

#[test]
fn standard_input_module_has_the_api_of_its_file() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP903", "--stdin-filename", "pkg/core.py", "-"],
        "def public() -> None: ...\n\n\ndef _private() -> None: ...\n",
    )
    .1;
    assert_eq!(
        (stdout.matches("SMP903").count(), stdout.contains("`public`") && !stdout.contains("`_private`")),
        (1, true),
        "{stdout}"
    );
}

// An external caller may rely on an optional argument even when every call in the project supplies it.
#[test]
fn standard_input_module_keeps_the_defaults_of_its_public_api() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP506", "--stdin-filename", "pkg/core.py", "-"],
        "def connect(host, timeout=30):\n    return host, timeout\n\n\ndef _helper(host, timeout=30):\n    return \
        host, timeout\n\n\nconnect(\"a\", 1)\n_helper(\"a\", 1)\n",
    )
    .1;
    assert_eq!(
        (stdout.matches("Every call to").count(), stdout.contains("`_helper`") && !stdout.contains("`connect`")),
        (1, true),
        "{stdout}"
    );
}

#[test]
fn standard_input_fix_keeps_the_docstring_of_public_api() {
    let root = project(&[("pyproject.toml", LIBRARY), ("pkg/__init__.py", "")]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP901", "--fix", "--unsafe-fixes", "--stdin-filename", "pkg/core.py", "-"],
        "def connect(host: str) -> str:\n    \"\"\"Connect to the host.\"\"\"\n    return host\n\n\ndef _inner(host: \
        str) -> str:\n    \"\"\"Inner.\"\"\"\n    return host\n",
    )
    .1;
    assert_eq!(
        (stdout.contains("\"\"\"Connect to the host.\"\"\""), stdout.contains("\"\"\"Inner")),
        (true, false),
        "{stdout}"
    );
}

#[test]
fn signature_reach_is_the_same_through_a_reexport() {
    let child = "class _Base:\n    def run(self) -> None: ...\n\n\nclass Child(_Base):\n    \
    \"\"\"Child.\"\"\"\n\n\n__all__ = [\"Child\"]\n";
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

#[test_case(false ; "defined_in_public_module")]
#[test_case(true ; "re-exported_from_private_module")]
fn signature_reached_dataclass_base_keeps_its_options_in_both_layouts(reexported: bool) {
    let core = "from dataclasses import dataclass\n\n\n@dataclass\nclass Base:\n    x: int\n\n\nclass \
    Child(Base):\n    \"\"\"Child.\"\"\"\n\n\n@dataclass\nclass Hidden:\n    x: int\n\n\n__all__ = [\"Child\"]\n";
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

// An excluded package initializer can still re-export definitions from files that simplint checks.
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
            .replace("pkg/", &native("pkg/"))
    );
}

// With no project index, the stdin check can still read the module's own `__all__`.
#[test_case("\"D103\"", common::PASSED ; "left_to_d103")]
#[test_case(
    "",
    "pkg/mod.py:1:5: SMP903 Public function `run` has no docstring; document it for its users\nFound 1 error.\n" ;
    "reported_without_d103"
)]
fn standard_input_reads_its_own_exports(select: &str, expected: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("{LIBRARY}[tool.ruff.lint]\nselect = [{select}]\n")),
        ("pkg/__init__.py", ""),
    ]);
    let stdout = check_with_stdin(
        root.path(),
        &["--select", "SMP903", "--stdin-filename", "pkg/mod.py", "-"],
        "def run() -> None: ...\n\n\ndef helper() -> None: ...\n\n\n__all__ = [\"run\"]\n",
    )
    .1;
    assert_eq!(stdout, expected);
}

#[test]
fn signature_reached_class_keeps_its_docstrings() {
    let root = project(&[
        ("pyproject.toml", LIBRARY),
        (
            "pkg/core.py",
            "class Config:\n    def describe(self) -> str:\n        \"\"\"Describe the settings.\"\"\"\n        return \
            \"config\"\n\n\ndef load() -> Config:\n    \"\"\"Load.\"\"\"\n    return Config()\n\n\ndef _helper() -> \
            None:\n    \"\"\"Internal.\"\"\"\n\n\n__all__ = [\"load\"]\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "SMP901");
    let fixed = common::read(root.path(), "pkg/core.py");
    assert_eq!(
        (fixed.contains("Describe the settings.") && fixed.contains("\"\"\"Load."), fixed.contains("Internal.")),
        (true, false),
        "{fixed}"
    );
}

// A module-level `__getattr__` can supply names dynamically, so the index cannot enumerate its re-exports.
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

// Changing `__all__` from a tuple to a list can break a test that checks its type.
#[test_case("tests/test_all.py", false ; "read_by_a_test")]
#[test_case("tools/all.py", true ; "read_outside_the_tests")]
fn tuple_all_a_test_reads_keeps_its_type(reader: &str, fixed: bool) {
    let root = project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n"),
        ("demo/__init__.py", "def run() -> None:\n    pass\n\n\n__all__ = (\n    \"run\",\n)\n"),
        (
            reader,
            "from importlib import import_module\n\n\ndef test_all() -> None:\n    \
             assert isinstance(getattr(import_module(\"demo\"), \"__all__\"), tuple)\n",
        ),
    ]);
    let stdout = check(root.path(), &["--select", "SMP405"]).1;
    assert_eq!(
        (stdout.contains("`__all__` is a tuple"), stdout.contains("1 unsafe fix available")),
        (true, fixed),
        "{stdout}"
    );
}

// A native extension can look up a Python name without a reference in Python source.
#[test_case("[build-system]\nrequires = [\"maturin>=1,<2\"]\nbuild-backend = \"maturin\"\n", false ; "maturin")]
#[test_case("[build-system]\nrequires = [\"setuptools\", \"setuptools_rust\"]\n", false ; "setuptools_rust")]
#[test_case("[build-system]\nrequires = [\"setuptools\"]\n", true ; "python_only")]
fn native_build_keeps_unexported_names(build: &str, fixed: bool) {
    let root = project(&[
        ("pyproject.toml", &format!("{build}\n[project]\nname = \"demo\"\n")),
        ("demo/__init__.py", ""),
        (
            "demo/_ssh.py",
            "def serialize() -> None:\n    pass\n\n\ndef load() -> None:\n    pass\n\n\n__all__ = [\n    \"load\",\n]\n",
        ),
    ]);
    let stdout = check(root.path(), &["--select", "SMP403"]).1;
    assert_eq!(
        (stdout.contains("`serialize` is not in `__all__`"), stdout.contains("1 unsafe fix available")),
        (true, fixed),
        "{stdout}"
    );
}
