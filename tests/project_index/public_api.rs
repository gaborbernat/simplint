use super::*;
use crate::common::{check, check_with_stdin, project};
use test_case::test_case;

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

// A public class a library re-exports from a private module passes its inherited public methods on to its users, so the
// docstring rules judge them as the class's API.
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
            "class _Base:\n    def run(self) -> None: ...\n\n\nclass _Config(_Base):\n    \
            \"\"\"Config.\"\"\"\n\n\n__all__ = [\"load\"]\n\n\ndef load() -> _Config:\n    \"\"\"Load.\"\"\"\n    \
            return _Config()\n",
        ),
    ]);
    let reports = check(root.path(), &["--select", "SMP903"]).1;
    assert_eq!(reports.matches("Public method `_Base.run`").count(), 1, "{reports}");
}

// A public module with an `__all__` offers what the list names, so a name another module imports from it without
// listing it is not part of its API, and its docstring is not required.
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

// A module whose `__getattr__` supplies names lazily offers what it defines, not a name it imports from a private
// module and another file reads off it, as the re-exports of such a module are not offered either.
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

// A class a public name's signature refers to, such as a base class, is part of the API its users reach, so the methods
// its listed subclass inherits from it are documented, while the class itself needs no docstring.
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

// A standard-input module that a library exposes keeps its public functions' default, since outside code may leave it
// out, and a helper it does not expose is still judged.
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
        "def connect(host: str) -> str:\n    \"\"\"Connect to the host.\"\"\"\n    return host\n\n\ndef _inner(host: \
        str) -> str:\n    \"\"\"Inner.\"\"\"\n    return host\n",
    )
    .1;
    assert!(stdout.contains("\"\"\"Connect to the host.\"\"\""), "{stdout}");
    assert!(!stdout.contains("\"\"\"Inner"), "{stdout}");
}

// Signature reach follows the names a module exposes, so a class a re-exporting package exposes with a base class
// behaves the same as when the module itself exposes it.
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

// A public signature reaches a class that is not listed in `__all__`, and the library's users get the base class of a
// dataclass it returns, so the dataclass keeps its options in both layouts.
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
    assert_eq!(stdout, common::PASSED);
}

// A class a listed function returns is library API, so its docstring and its method's docstring stay when SMP901's
// unsafe fix runs, while an internal helper's docstring is still turned into a comment.
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
