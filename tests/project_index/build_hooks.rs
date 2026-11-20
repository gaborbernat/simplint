use super::*;
use crate::common::{check, native, project};
use test_case::test_case;

#[test_case("[tool.pytest.ini_options]\naddopts = \"-q -p pkg._plugin\"\n", None ; "ini_options")]
#[test_case("[tool.pytest]\naddopts = [\"-q\", \"-p\", \"pkg._plugin\"]\n", None ; "native_options")]
#[test_case("", Some(("pytest.ini", "[pytest]\naddopts = -q -p pkg._plugin\n")) ; "pytest_ini")]
#[test_case("", Some(("tox.ini", "[pytest]\naddopts =\n    -q\n    -ppkg._plugin\n")) ; "tox_ini")]
fn pytest_plugin_counts_as_used(tool: &str, ini: Option<(&str, &str)>) {
    let pyproject = format!("{LIBRARY}{tool}");
    let mut files = vec![
        ("pyproject.toml", pyproject.as_str()),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pytest_configure\"]\n\n\ndef pytest_configure() -> None: ...\n"),
    ];
    files.extend(ini);
    let root = project(&files);
    common::assert_clean(root.path(), "SMP402");
    let disabled = project(&[
        ("pyproject.toml", &format!("{LIBRARY}[tool.pytest.ini_options]\naddopts = \"-p no:pkg._plugin\"\n")),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pytest_configure\"]\n\n\ndef pytest_configure() -> None: ...\n"),
    ]);
    assert!(check(disabled.path(), &["--select", "SMP402"]).1.contains("`pytest_configure`"));
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
                "[project]\nname = \"pkg\"\n[build-system]\nbuild-backend = \"pdm.backend\"\nrequires = \
                [\"pdm-backend\"]\n{config}"
            ),
        ),
        (file, "__all__ = [\"pdm_build_initialize\"]\n\n\ndef pdm_build_initialize(context: object) -> None: ...\n"),
    ]);
    common::assert_clean(root.path(), "SMP402");
    let unhooked = project(&[
        (
            "pyproject.toml",
            concat!(
                "[project]\n",
                "name = \"pkg\"\n",
                "[build-system]\n",
                "build-backend = \"pdm.backend\"\n",
                "requires = [\"pdm-backend\"]\n"
            ),
        ),
        ("build.py", "__all__ = []\n\n\ndef helper() -> None: ...\n"),
    ]);
    assert!(check(unhooked.path(), &["--select", "SMP409"]).1.contains("`helper`"));
}

#[test]
fn missing_pdm_build_hook_names_no_function() {
    let root = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"pkg\"\n[build-system]\nbuild-backend = \"pdm.backend\"\nrequires = \
            [\"pdm-backend\"]\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/_plugin.py", "__all__ = [\"pdm_build_initialize\"]\n\n\ndef pdm_build_initialize() -> None: ...\n"),
    ]);
    assert!(check(root.path(), &["--select", "SMP402"]).1.contains("`pdm_build_initialize`"));
}

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

// simplint treats `__main__.py` as a script. Other dunder modules, such as `__about__.py`, can expose library API.
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

// A build frontend calls the configured backend's hooks without importing their names in project code.
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
    common::assert_clean(root.path(), "SMP402");
    let unlisted_helper = project(&[
        ("pyproject.toml", BACKEND),
        ("demo/__init__.py", ""),
        (
            "build_backend.py",
            concat!(
                "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n",
                "def helper() -> None: ...\n\n\n__all__ = [\"build_wheel\", \"helper\"]\n"
            ),
        ),
    ]);
    assert!(check(unlisted_helper.path(), &["--select", "SMP402"]).1.contains("`helper`"));
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
         build_backend.py:5:5: SMP401 `get_requires_for_build_wheel` is imported by other modules; list it in \
         `__all__`\nFound 2 errors.\n[*] 1 fixable with `--fix`.\n"
    );
}

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
    common::assert_clean(root.path(), "SMP403,SMP409");
    let unexported = project(&[
        ("pyproject.toml", "[build-system]\nbuild-backend = \"backend:Backend\"\nrequires = []\n"),
        ("backend.py", "class Backend:\n    pass\n\n\ndef helper() -> None: ...\n\n\n__all__ = []\n"),
    ]);
    assert!(check(unexported.path(), &["--select", "SMP403,SMP409"]).1.contains("`helper`"));
}

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
            "build/backend.py:5:5: SMP403 `helper` is not in `__all__`; list it there, or rename it to `_helper` by \
            hand\n\
             build/backend.py:5:5: SMP409 `helper` is never used in the run; remove it or give it a caller\n\
             Found 2 errors.\n"
        )
    );
}

#[test_case("pkg.backend", "pkg/backend/__init__.py" ; "package")]
#[test_case("pkg.backend", "pkg/backend.py" ; "module")]
fn build_backend_by_dotted_name(backend: &str, file: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[build-system]\nbuild-backend = \"{backend}\"\nrequires = []\n")),
        (file, "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n__all__ = []\n"),
    ]);
    common::assert_clean(root.path(), "SMP409");
    let other = project(&[
        ("pyproject.toml", "[build-system]\nbuild-backend = \"pkg.backend\"\nrequires = []\n"),
        (
            "pkg/other.py",
            "def build_wheel(wheel_directory: str) -> str:\n    return wheel_directory\n\n\n__all__ = []\n",
        ),
    ]);
    assert!(check(other.path(), &["--select", "SMP409"]).1.contains("`build_wheel`"));
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

// Hatchling looks for a hook subclass in the configured file, which defaults to `hatch_build.py`.
#[test_case("[tool.hatch]\nbuild.hooks.custom = {}\n", "hatch_build.py", BUILD_HOOK ; "build_default_path")]
#[test_case(
    concat!(
        "[tool.hatch.build.hooks.custom]\n",
        "path = \"build/hooks.py\"\n"
    ),
    "build/hooks.py",
    BUILD_HOOK ; "build_path"
)]
#[test_case("[tool.hatch.metadata.hooks.custom]\n", "hatch_build.py", METADATA_HOOK ; "metadata_default_path")]
#[test_case(
    concat!(
        "[tool.hatch.metadata.hooks.custom]\n",
        "path = \"build/hooks.py\"\n"
    ),
    "build/hooks.py",
    METADATA_HOOK ; "metadata_path"
)]
#[test_case("[tool.hatch.build.targets.wheel.hooks.custom]\n", "hatch_build.py", BUILD_HOOK ; "target_default_path")]
#[test_case(
    concat!(
        "[tool.hatch.build.targets.sdist.hooks.custom]\n",
        "path = \"build/hooks.py\"\n"
    ),
    "build/hooks.py",
    BUILD_HOOK ; "target_path"
)]
fn configured_hatch_hook_class_is_used(hatch: &str, file: &str, source: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\nversion = \"1\"\n{hatch}")),
        (file, source),
    ]);
    common::assert_clean(root.path(), "SMP409");
    let unconfigured =
        project(&[("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"), ("hatch_build.py", BUILD_HOOK)]);
    assert!(check(unconfigured.path(), &["--select", "SMP409"]).1.contains("`_Hook`"));
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

// In `hatch.toml`, Hatch tables omit the `tool.hatch` prefix.
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
    common::assert_clean(root.path(), "SMP409");
    let without_hatch_toml =
        project(&[("pyproject.toml", "[project]\nname = \"demo\"\nversion = \"1\"\n"), (file, source)]);
    assert!(check(without_hatch_toml.path(), &["--select", "SMP409"]).1.contains("`_Hook`"));
}

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

// A partial override in `hatch.toml` can leave a hook setting from `pyproject.toml` in effect.
#[test_case(
    concat!(
        "[tool.hatch.build.hooks.custom]\n",
        "path = \"other.py\"\n"
    ),
    concat!(
        "[build.hooks.custom]\n",
        "path = \"hatch_build.py\"\n"
    ),
    common::PASSED ; "hatch_toml_overrides_the_path"
)]
#[test_case(
    concat!(
        "[tool.hatch.build.hooks.custom]\n",
        "path = \"other.py\"\n"
    ),
    "[build.hooks.custom]\n",
    concat!(
        "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\n",
        "Found 1 error.\n"
    ) ; "hatch_toml_keeps_the_unset_path"
)]
#[test_case(
    "[tool.hatch.build.hooks.custom]\n",
    concat!(
        "[build.hooks.custom]\n",
        "path = \"other.py\"\n"
    ),
    concat!(
        "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\n",
        "Found 1 error.\n"
    ) ; "hatch_toml_moves_the_path"
)]
fn hatch_toml_is_merged_over_the_pyproject(pyproject: &str, hatch: &str, expected: &str) {
    let root = project(&[
        ("pyproject.toml", &format!("[project]\nname = \"demo\"\nversion = \"1\"\n{pyproject}")),
        ("hatch.toml", hatch),
        ("hatch_build.py", BUILD_HOOK),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP409"]).1, expected);
}

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
    common::assert_clean(root.path(), "SMP409");
    let other_path = project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"1\"\n[tool.hatch.build.hooks.custom]\npath = \"other/h.py\"\n",
        ),
        ("hooks/h.py", BUILD_HOOK),
    ]);
    assert!(check(other_path.path(), &["--select", "SMP409"]).1.contains("`_Hook`"));
}

// Hatchling can discover a hook through an intermediate base class.
#[test_case("class CommonHook(BuildHookInterface):\n    pass\n", common::PASSED ; "inherited_from_hook_class")]
#[test_case(
    concat!(
        "class CommonHook:\n",
        "    pass\n"
    ),
    concat!(
        "hatch_build.py:4:7: SMP409 `_Hook` is never used in the run; remove it or give it a user\n",
        "Found 1 error.\n"
    ) ; "inherited_from_plain_class"
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

// Startup code can reference a function even when only tests import it. Path entries in the same `.pth` file do not
// call Python functions.
#[test_case("import pkg.hook; pkg.hook._run()\n", common::PASSED ; "startup_read_keeps_the_name")]
#[test_case("import sys; from pkg.hook import _run; _run()\n", common::PASSED ; "startup_from_import_keeps_the_name")]
#[test_case("import\tpkg.hook; pkg.hook._run()\n", common::PASSED ; "tab_after_import_keeps_the_name")]
#[test_case("import pkg.hook as hook; hook._run()\n", common::PASSED ; "alias_keeps_the_name")]
#[test_case(
    "import pkg.hook\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "bare_import_reads_no_name"
)]
#[test_case(
    "pkg.hook._run()\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "line_without_import_is_not_run"
)]
#[test_case(
    "pkg/hook.py\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "path_line_names_nothing"
)]
#[test_case(
    "import pkg.hook; pkg.hook._run(\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "unparsable_line_names_nothing"
)]
#[test_case(
    "import pkg.hook; os.sep\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "unbound_attribute_names_nothing"
)]
#[test_case(
    "import missing.hook; missing.hook._run()\n",
    concat!(
        "pkg/hook.py:1:5: SMP408 `_run` is used only by tests; move it into the tests or a test helper ",
        "module\n",
        "Found 1 error.\n"
    ) ; "unresolved_module_names_nothing"
)]
fn startup_pth_reads_are_production_use(pth: &str, stdout: &str) {
    let root = project(&[
        ("pkg.pth", pth),
        ("pkg/__init__.py", ""),
        ("pkg/hook.py", HOOK_PROJECT),
        ("tests/test_hook.py", "from pkg.hook import _run\n\n\ndef test_run() -> None:\n    _run()\n"),
    ]);
    assert_eq!(check(root.path(), &["--select", "SMP408"]).1, stdout.replace("pkg/", &native("pkg/")));
}
