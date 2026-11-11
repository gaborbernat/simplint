use super::*;
use crate::common::{check, project};
use test_case::test_case;

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
