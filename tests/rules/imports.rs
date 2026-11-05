use test_case::test_case;

use crate::common;

#[test_case("future-annotations", "future_annotations.py", "py314" ; "future_annotations")]
#[test_case("future-annotations", "future_annotations.py", "py310" ; "future_annotations_py310")]
#[test_case("future-annotations", "future_annotations_checking.py", "py314" ; "future_annotations_type_checking")]
#[test_case("future-annotations", "future_annotations_joined.py", "py314" ; "future_annotations_joined_statement")]
#[test_case("future-annotations", "future_annotations_class_bindings.py", "py314" ; "future_annotations_class_bindings")]
#[test_case("future-annotations", "future_annotations_class_shadow.py", "py314" ; "future_annotations_class_shadow")]
#[test_case("absolute-sibling-import", "package/sub/module.py", "py314" ; "absolute_sibling_import")]
#[test_case("absolute-sibling-import", "package/__init__.py", "py314" ; "absolute_sibling_import_init")]
#[test_case("absolute-sibling-import", "package/sub/script.py", "py314" ; "absolute_sibling_import_script")]
#[test_case("absolute-sibling-import", "package/sub/channel_exec.py", "py314" ; "absolute_sibling_import_channel_exec")]
#[test_case("absolute-sibling-import", "package/sub/remote_worker.py", "py314" ; "absolute_sibling_import_remote_worker")]
#[test_case("absolute-sibling-import", "package/sub/reversed_guard.py", "py314" ; "absolute_sibling_import_reversed_guard")]
#[test_case(
    "absolute-sibling-import", "multiline_module_docstring_ok.py", "py314" ; "absolute_sibling_import_outside_package"
)]
#[test_case("module-attribute-import", "writers/module_fix.py", "py314" ; "module_attribute_import_fix")]
#[test_case("module-attribute-import", "writers/module_alias.py", "py314" ; "module_attribute_import_alias")]
#[test_case("module-attribute-import", "writers/module_mixed.py", "py314" ; "module_attribute_import_store_without_fix")]
#[test_case("module-attribute-import", "writers/module_undefined.py", "py314" ; "module_attribute_import_undefined")]
#[test_case("module-attribute-import", "writers/module_value.py", "py314" ; "module_attribute_import_value")]
#[test_case("module-attribute-import", "writers/module_type_param.py", "py314" ; "module_attribute_import_type_param")]
#[test_case("module-attribute-import", "writers/module_parameter.py", "py314" ; "module_attribute_import_parameter")]
#[test_case("module-attribute-import", "writers/module_clash.py", "py314" ; "module_attribute_import_clash")]
#[test_case("module-attribute-import", "writers/module_rebound.py", "py314" ; "module_attribute_import_rebound")]
#[test_case("module-attribute-import", "writers/module_dunder.py", "py314" ; "module_attribute_import_dunder")]
#[test_case("module-attribute-import", "writers/module_third_party.py", "py314" ; "module_attribute_import_third_party")]
#[test_case("module-attribute-import", "writers/module_patched.py", "py314" ; "module_attribute_import_patched")]
#[test_case("module-attribute-import", "writers/module_lazy.py", "py315" ; "module_attribute_import_lazy")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", "py314" ; "dependency_import_guard")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", "py310" ; "dependency_import_guard_py310")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
