use test_case::test_case;

use crate::common;

#[test_case("future-annotations", "future_annotations.py", "py314" ; "future_annotations")]
#[test_case("future-annotations", "future_annotations.py", "py310" ; "future_annotations_py310")]
#[test_case("future-annotations", "future_annotations_checking.py", "py314" ; "future_annotations_type_checking")]
#[test_case("future-annotations", "future_annotations_joined.py", "py314" ; "future_annotations_joined_statement")]
#[test_case("future-annotations", "future_annotations_class_bindings.py", "py314" ; "future_annotations_class_bindings")]
#[test_case("future-annotations", "future_annotations_class_shadow.py", "py314" ; "future_annotations_class_shadow")]
#[test_case("import-cycle", "import_cycle.py", "py314" ; "import_cycle")]
#[test_case("import-cycle", "import_cycle_partner.py", "py314" ; "import_cycle_partner")]
#[test_case("import-cycle", "import_cycle_ring_a.py", "py314" ; "import_cycle_ring")]
#[test_case("import-cycle", "import_cycle_exceptions.py", "py314" ; "import_cycle_exceptions")]
#[test_case("import-cycle", "import_cycle_lazy.py", "py315" ; "import_cycle_lazy")]
#[test_case("import-cycle", "import_cycle_pkg/core.py", "py314" ; "import_cycle_package")]
#[test_case("import-cycle", "import_cycle_dead.py", "py314" ; "import_cycle_dead")]
#[test_case("import-cycle", "import_cycle_platform.py", "py314" ; "import_cycle_platform")]
#[test_case("import-cycle", "import_cycle_platform_partner.py", "py314" ; "import_cycle_platform_partner")]
#[test_case("import-cycle", "test_import_cycle_fixture.py", "py314" ; "import_cycle_test_module")]
#[test_case("star-import", "star_import_listed.py", "py314" ; "star_import_listed")]
#[test_case("star-import", "star_import_unlisted.py", "py314" ; "star_import_unlisted")]
#[test_case("star-import", "star_import_unlisted_import.py", "py314" ; "star_import_unlisted_import")]
#[test_case("star-import", "star_import_own_all.py", "py314" ; "star_import_own_all")]
#[test_case("star-import", "star_import_multiple.py", "py314" ; "star_import_multiple")]
#[test_case("star-import", "star_import_dynamic.py", "py314" ; "star_import_dynamic")]
#[test_case("star-import", "star_import_unread.py", "py314" ; "star_import_unread")]
#[test_case("star-import", "star_import_external.py", "py314" ; "star_import_external")]
#[test_case("star-import", "star_pkg/consumer.py", "py314" ; "star_import_package")]
#[test_case("star-import", "star_import_reexport.py", "py314" ; "star_import_reexport")]
#[test_case("star-import", "star_import_reexporter.py", "py314" ; "star_import_reexporter")]
#[test_case("star-import", "star_import_builtin.py", "py314" ; "star_import_builtin")]
#[test_case("star-import", "star_import_undefined.py", "py314" ; "star_import_undefined")]
#[test_case("star-import", "star_import_conditional.py", "py314" ; "star_import_conditional")]
#[test_case("star-import", "star_import_dynamic_all.py", "py314" ; "star_import_dynamic_all")]
#[test_case("star-import", "star_import_fallback.py", "py314" ; "star_import_fallback")]
#[test_case("star-import", "star_import_comment.py", "py314" ; "star_import_comment")]
#[test_case("star-import", "star_reexport_pkg/__init__.py", "py314" ; "star_import_init")]
#[test_case("star-import", "star_import_hidden.py", "py314" ; "star_import_hidden")]
#[test_case("star-import", "star_import_partial.py", "py314" ; "star_import_partial")]
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
#[test_case("up-backport-import", "up_backport_import.py", "py311" ; "backport_import")]
#[test_case("up-backport-import", "up_backport_import.py", "py310" ; "backport_import_py310")]
#[test_case("up-backport-import", "up_backport_import_single.py", "py311" ; "backport_import_single")]
#[test_case("up-backport-import", "up_backport_import_inline.py", "py311" ; "backport_import_inline")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import.py", "py315" ; "type_checking_lazy_import")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import.py", "py314" ; "type_checking_lazy_import_py314")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_shared.py", "py315" ; "type_checking_lazy_import_shared")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_attribute.py", "py315" ; "type_checking_lazy_import_attribute")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_comment.py", "py315" ; "type_checking_lazy_import_comment")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed.py", "py315" ; "type_checking_lazy_import_typeshed")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_inline.py", "py315" ; "type_checking_lazy_import_inline")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_ignored.py", "py315" ; "type_checking_lazy_import_ignored")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_module.py", "py315" ; "type_checking_lazy_import_module")]
#[test_case("up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed_module.py", "py315" ; "type_checking_lazy_import_typeshed_module")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", "py315" ; "lazy_modules_list")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", "py314" ; "lazy_modules_list_py314")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_partial.py", "py315" ; "lazy_modules_list_partial")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_mixed.py", "py315" ; "lazy_modules_list_mixed")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_nonliteral.py", "py315" ; "lazy_modules_list_nonliteral")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_comprehension.py", "py315" ; "lazy_modules_list_comprehension")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_guarded.py", "py315" ; "lazy_modules_list_guarded")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_inline.py", "py315" ; "lazy_modules_list_inline")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_tuple.py", "py315" ; "lazy_modules_list_tuple")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_set.py", "py315" ; "lazy_modules_list_set")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// A function that rebinds a module variable through `global` changes it after an import would have bound it.
#[test]
fn module_attribute_import_keeps_names_a_function_rebinds_through_global() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/config.py", "_run = True\n\n\ndef set_run(run: bool) -> None:\n    global _run\n    _run = run\n"),
        ("pkg/setters.py", "from . import config\n\n\ndef check() -> bool:\n    return config._run\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP304"]).1;
    assert!(stdout.contains("SMP304 Import `_run`"));
    assert!(!stdout.contains("SMP304 [*]"));
}
