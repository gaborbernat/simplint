use test_case::test_case;

use crate::common;

#[test_case("dependency-import-guard", "dependency_import_guard.py", "py314" ; "dependency_import_guard")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", "py310" ; "dependency_import_guard_py310")]
#[test_case("future-annotations", "future_annotations.py", "py314" ; "future_annotations")]
#[test_case("future-annotations", "future_annotations.py", "py310" ; "future_annotations_py310")]
#[test_case("future-annotations", "future_annotations_checking.py", "py314" ; "future_annotations_type_checking")]
#[test_case("future-annotations", "future_annotations_joined.py", "py314" ; "future_annotations_joined_statement")]
#[test_case("init-module-content", "package/__init__.py", "py314" ; "init_module_content")]
#[test_case("init-module-content", "package/sub/__init__.py", "py314" ; "init_module_content_docstring_only")]
#[test_case("init-module-content", "multiline_module_docstring_ok.py", "py314" ; "init_module_content_regular_module")]
#[test_case("absolute-sibling-import", "package/sub/module.py", "py314" ; "absolute_sibling_import")]
#[test_case("absolute-sibling-import", "package/__init__.py", "py314" ; "absolute_sibling_import_init")]
#[test_case("absolute-sibling-import", "package/sub/script.py", "py314" ; "absolute_sibling_import_script")]
#[test_case(
    "absolute-sibling-import", "multiline_module_docstring_ok.py", "py314" ; "absolute_sibling_import_outside_package"
)]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
