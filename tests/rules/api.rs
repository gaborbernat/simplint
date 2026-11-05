use test_case::test_case;

use crate::common;

#[test_case("missing-from-all", "missing_from_all.py" ; "missing_from_all")]
#[test_case("missing-from-all", "missing_from_all_multiline.py" ; "missing_from_all_multiline")]
#[test_case("missing-from-all", "missing_from_all_noall.py" ; "missing_from_all_creates_all")]
#[test_case("missing-from-all", "missing_from_all_guard.py" ; "missing_from_all_above_main_guard")]
#[test_case("missing-from-all", "missing_from_all_bare.py" ; "missing_from_all_bare_tuple")]
#[test_case("missing-from-all", "missing_from_all_sorted.py" ; "missing_from_all_sorted_insertion")]
#[test_case("missing-from-all", "missing_from_all_reexport.py" ; "missing_from_all_reexported_all")]
#[test_case("missing-from-all", "missing_from_all_hanging.py" ; "missing_from_all_hanging_indent")]
#[test_case("missing-from-all", "missing_from_all_hanging_comment.py" ; "missing_from_all_hanging_indent_comment")]
#[test_case("missing-from-all", "missing_from_all_aug.py" ; "missing_from_all_augmented")]
#[test_case("missing-from-all", "missing_from_all_call.py" ; "missing_from_all_dynamic")]
#[test_case("missing-from-all", "missing_from_all_nested.py" ; "missing_from_all_nested_all")]
#[test_case("missing-from-all", "missing_from_all_empty.py" ; "missing_from_all_empty")]
#[test_case("missing-from-all", "missing_from_all_imported.py" ; "missing_from_all_imported_name")]
#[test_case("missing-from-all", "missing_from_all_reached.py" ; "missing_from_all_reached_types")]
#[test_case("missing-from-all", "missing_from_all_scripts.py" ; "missing_from_all_entry_point")]
#[test_case("missing-from-all", "tests/test_module_docstring.py" ; "missing_from_all_test_module")]
#[test_case("missing-from-all", "package/__init__.py" ; "missing_from_all_init_module")]
#[test_case("unused-export", "exporting/unused_export.py" ; "unused_export")]
#[test_case("unused-export", "exporting/unused_export_inline.py" ; "unused_export_inline")]
#[test_case("unused-export", "exporting/unused_export_tuple.py" ; "unused_export_tuple")]
#[test_case("unused-export", "exporting/unused_export_all.py" ; "unused_export_all_unused")]
#[test_case("unused-export", "exporting/unused_export_comment.py" ; "unused_export_comment")]
#[test_case("unused-export", "exporting/unused_export_extend.py" ; "unused_export_extended")]
#[test_case("unused-export", "exporting/unused_export_call.py" ; "unused_export_extend_call")]
#[test_case("unused-export", "exporting/sub/__init__.py" ; "unused_export_init_module")]
#[test_case("unused-export", "exporting/unused_export_double.py" ; "unused_export_double_import")]
#[test_case("unused-export", "exporting/unused_export_comment_all.py" ; "unused_export_comment_all")]
#[test_case("unused-export", "exporting/unused_export_lazy.py" ; "unused_export_lazy_module")]
#[test_case("unused-export", "exporting/unused_export_packed.py" ; "unused_export_packed")]
#[test_case("unused-export", "exporting/unused_export_tuple_end.py" ; "unused_export_tuple_end")]
#[test_case("unused-export", "exporting/unused_export_noted.py" ; "unused_export_noted")]
#[test_case("unused-export", "exporting/unused_export_star.py" ; "unused_export_star_imported")]
#[test_case("unused-export", "unused_export_root.py" ; "unused_export_api_root")]
#[test_case("unused-export", "exporting/unused_export_reached.py" ; "unused_export_reached_types")]
#[test_case("unused-export", "tests/test_module_docstring.py" ; "unused_export_test_module")]
#[test_case("unexported-public-name", "unexported_public_name.py" ; "unexported_public_name")]
#[test_case("unexported-public-name", "unexported_public_name_extend.py" ; "unexported_public_name_extended_all")]
#[test_case("unexported-public-name", "unexported_public_name_remove.py" ; "unexported_public_name_dynamic_all")]
#[test_case("unexported-public-name", "tests/test_module_docstring.py" ; "unexported_public_name_test_module")]
#[test_case("unexported-public-name", "multiline_module_docstring_ok.py" ; "unexported_public_name_without_all")]
#[test_case("unexported-public-name", "unexported_public_name_ply.py" ; "unexported_public_name_ply_grammar")]
#[test_case("unexported-public-name", "missing_from_all_reached.py" ; "unexported_public_name_reached_types")]
#[test_case("module-variable-prefix", "module_variable_prefix.py" ; "module_variable_prefix")]
#[test_case("module-variable-prefix", "module_variable_prefix_source_text.py" ; "module_variable_prefix_source_text")]
#[test_case("module-variable-prefix", "module_variable_prefix_type_alias.py" ; "module_variable_prefix_type_alias")]
#[test_case("unexported-public-name", "unexported_public_name_source_text.py" ; "unexported_public_name_source_text")]
#[test_case("all-format", "all_format.py" ; "all_format")]
#[test_case("all-format", "all_format_tuple.py" ; "all_format_tuple")]
#[test_case("all-format", "all_format_bare.py" ; "all_format_bare_tuple")]
#[test_case("all-not-last", "all_not_last.py" ; "all_not_last")]
#[test_case("all-not-last", "all_not_last_eof.py" ; "all_not_last_at_eof")]
#[test_case("all-not-last", "all_not_last_reversed.py" ; "all_not_last_reversed_main_guard")]
#[test_case("all-not-last", "all_not_last_nested.py" ; "all_not_last_extended_in_block")]
#[test_case("all-not-last", "all_not_last_negated.py" ; "all_not_last_negated_guard")]
#[test_case("all-not-last", "all_not_last_nonl.py" ; "all_not_last_without_trailing_newline")]
#[test_case("all-not-last", "all_not_last_ann.py" ; "all_not_last_annotated_and_augmented")]
#[test_case("all-not-last", "all_not_last_fenced.py" ; "all_not_last_keeps_its_fmt_off_fence")]
#[test_case("all-not-last", "all_not_last_mutated.py" ; "all_not_last_mutated_after_binding")]
#[test_case("all-not-last", "all_not_last_mutated_before_definition.py" ; "all_not_last_mutated_before_definition")]
#[test_case("all-not-last", "multiline_module_docstring_ok.py" ; "all_not_last_without_all")]
#[test_case("init-module-content", "package/__init__.py" ; "init_module_content")]
#[test_case("init-module-content", "facade/__init__.py" ; "init_module_content_facade_with_all")]
#[test_case("init-module-content", "loose/__init__.py" ; "init_module_content_facade_without_all")]
#[test_case("init-module-content", "_hidden/__init__.py" ; "init_module_content_private_package")]
#[test_case("init-module-content", "package/sub/__init__.py" ; "init_module_content_docstring_only")]
#[test_case("init-module-content", "multiline_module_docstring_ok.py" ; "init_module_content_regular_module")]
#[test_case("init-module-content", "lazy_modules/__init__.py" ; "init_module_content_lazy_modules")]
#[test_case("init-module-content", "lazy_annotated/__init__.py" ; "init_module_content_lazy_annotated")]
#[test_case("test-only-code", "tonly_builders.py" ; "test_only_code")]
#[test_case("test-only-code", "tonly_app.py" ; "test_only_code_used_outside_tests")]
#[test_case("test-only-code", "test_tonly_builders.py" ; "test_only_code_test_module")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test]
fn all_format_follows_skip_magic_trailing_comma() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff.format]\nskip-magic-trailing-comma = true\n"),
        ("listed.py", "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n"),
        ("paired.py", "def load() -> None: ...\n\n\ndef save() -> None: ...\n\n\n__all__ = \"load\", \"save\"\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP405"]);
    assert_eq!(
        (common::read(root.path(), "listed.py"), common::read(root.path(), "paired.py")),
        (
            "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n".to_owned(),
            "def load() -> None: ...\n\n\ndef save() -> None: ...\n\n\n__all__ = [\"load\", \"save\"]\n".to_owned()
        )
    );
}
