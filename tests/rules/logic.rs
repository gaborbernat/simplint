use tempfile::TempDir;
use test_case::test_case;

use crate::common::{self, check, read};

#[test_case("duplicate-code", "duplicate_code.py", None ; "duplicate_code")]
#[test_case("duplicate-code", "duplicate_code_copy.py", None ; "duplicate_code_copy")]
#[test_case("duplicate-code", "duplicate_code_many.py", None ; "duplicate_code_many")]
#[test_case("duplicate-code", "duplicate_code_renamed.py", None ; "duplicate_code_renamed")]
#[test_case("duplicate-code", "duplicate_code_twice.py", None ; "duplicate_code_twice")]
#[test_case("duplicate-code", "duplicate_code_short.py", None ; "duplicate_code_short")]
#[test_case("duplicate-code", "duplicate_code_nested.py", None ; "duplicate_code_nested")]
#[test_case("duplicate-code", "duplicate_code_import.py", None ; "duplicate_code_import")]
#[test_case("duplicate-code", "duplicate_code_nested_copy.py", None ; "duplicate_code_nested_copy")]
#[test_case("duplicate-code", "duplicate_code_short_copy.py", None ; "duplicate_code_short_copy")]
#[test_case("duplicate-code", "duplicate_code_import_copy.py", None ; "duplicate_code_import_copy")]
#[test_case("duplicate-code", "duplicate_code_block.py", None ; "duplicate_code_block")]
#[test_case("duplicate-code", "test_duplicate_code_module.py", None ; "duplicate_code_test_module")]
#[test_case("forwarding-function", "forwarding_function.py", None ; "forwarding_function")]
#[test_case("forwarding-function", "forwarding_function_hook.py", None ; "forwarding_function_hook")]
#[test_case("forwarding-function", "forwarding_function_reference.py", None ; "forwarding_function_reference")]
#[test_case("forwarding-function", "forwarding_function_call_lines.py", None ; "forwarding_function_call_lines")]
#[test_case("forwarding-function", "forwarding_function_receiver.py", None ; "forwarding_function_receiver")]
#[test_case("walrus-candidate", "walrus_candidate.py", None ; "walrus_candidate")]
#[test_case("walrus-candidate", "walrus_candidate.py", Some("py37") ; "walrus_candidate_before_python38")]
#[test_case("walrus-candidate", "walrus_candidate_gate.py", None ; "walrus_candidate_version_gate")]
#[test_case("walrus-candidate", "walrus_candidate_dynamic.py", None ; "walrus_candidate_computed_all")]
#[test_case("walrus-candidate", "walrus_candidate_listed.py", None ; "walrus_candidate_listed_in_all")]
#[test_case("walrus-candidate", "walrus_candidate_width.py", None ; "walrus_candidate_header_width")]
#[test_case("walrus-candidate", "walrus_candidate_locals.py", None ; "walrus_candidate_locals")]
#[test_case("walrus-candidate", "walrus_candidate_globals.py", None ; "walrus_candidate_globals")]
#[test_case("single-caller-helper", "single_caller_helper.py", None ; "single_caller_helper")]
#[test_case("single-caller-helper", "single_caller_helper_dynamic.py", None ; "single_caller_helper_computed_all")]
#[test_case("single-caller-helper", "single_caller_helper_recursive.py", None ; "single_caller_helper_recursive")]
#[test_case("single-caller-helper", "single_caller_helper_reference.py", None ; "single_caller_helper_reference")]
#[test_case("single-caller-helper", "single_caller_helper_call_lines.py", None ; "single_caller_helper_call_lines")]
#[test_case(
    "single-caller-helper", "tests/test_single_caller_helper_from_test.py", None ; "single_caller_helper_from_test"
)]
#[test_case("single-use-variable", "single_use_variable.py", None ; "single_use_variable")]
#[test_case("single-use-variable", "single_use_variable_width.py", None ; "single_use_variable_width")]
#[test_case("single-use-variable", "single_use_variable_debug.py", None ; "single_use_variable_debug")]
#[test_case("single-use-variable", "single_use_variable_raw_debug.py", None ; "single_use_variable_raw_debug")]
#[test_case("single-use-variable", "single_use_variable_requote.py", Some("py311") ; "single_use_variable_requote")]
#[test_case("single-use-variable", "single_use_variable_splice.py", None ; "single_use_variable_splice")]
#[test_case(
    "single-use-variable",
    "single_use_variable_splice.py",
    Some("py311") ;
    "single_use_variable_splice_before_python312"
)]
#[test_case(
    "single-use-variable", "single_use_variable_nested_fstring.py", None ; "single_use_variable_nested_fstring"
)]
#[test_case(
    "single-use-variable",
    "single_use_variable_nested_fstring.py",
    Some("py311") ;
    "single_use_variable_nested_fstring_before_python312"
)]
#[test_case("single-use-variable", "single_use_variable_parens.py", None ; "single_use_variable_parens")]
#[test_case("single-use-variable", "single_use_variable_field.py", None ; "single_use_variable_field")]
#[test_case("single-use-variable", "single_use_variable_lifetime.py", None ; "single_use_variable_lifetime")]
#[test_case("single-use-variable", "single_use_variable_augassign.py", None ; "single_use_variable_augassign")]
#[test_case("single-use-variable", "single_use_variable_operator.py", None ; "single_use_variable_operator")]
#[test_case("single-use-variable", "single_use_variable_walrus.py", None ; "single_use_variable_walrus")]
#[test_case("single-use-variable", "single_use_variable_tstring.py", None ; "single_use_variable_tstring")]
#[test_case("single-use-variable", "single_use_variable_format_spec.py", None ; "single_use_variable_format_spec")]
#[test_case("string-concatenation", "string_concatenation.py", None ; "string_concatenation")]
#[test_case("string-concatenation", "string_concatenation_old.py", Some("py310") ; "string_concatenation_py310")]
#[test_case("string-concatenation", "string_concatenation_types.py", None ; "string_concatenation_types")]
#[test_case("string-concatenation", "string_concatenation_kinds.py", None ; "string_concatenation_kinds")]
#[test_case("string-concatenation", "string_concatenation_class.py", None ; "string_concatenation_class")]
#[test_case("string-concatenation", "string_concatenation_genexp.py", None ; "string_concatenation_genexp")]
#[test_case("string-concatenation", "string_concatenation_type_params.py", None ; "string_concatenation_type_params")]
#[test_case("string-concatenation", "string_concatenation_ruff_overlap.py", None ; "string_concatenation_ruff_overlap")]
#[test_case("string-concatenation", "string_concatenation_imported.py", None ; "string_concatenation_imported")]
#[test_case(
    "string-concatenation", "string_concatenation_imported_values.py", None ; "string_concatenation_imported_values"
)]
#[test_case("string-concatenation", "string_concatenation_class_values.py", None ; "string_concatenation_class_values")]
#[test_case("string-concatenation", "string_concatenation_bindings.py", None ; "string_concatenation_bindings")]
#[test_case("default-argument-value", "default_argument_value.py", None ; "default_argument_value")]
#[test_case("unused-default", "unused_default.py", None ; "unused_default")]
#[test_case("unused-default", "unused_default_public_module.py", None ; "unused_default_public_module")]
#[test_case("unused-default", "unused_default_comment.py", None ; "unused_default_comment")]
#[test_case("error-message-matching", "error_message_matching.py", None ; "error_message_matching")]
#[test_case("up-comprehension-unpacking", "up_comprehension_unpacking.py", Some("py315") ; "comprehension_unpacking")]
#[test_case(
    "up-comprehension-unpacking", "up_comprehension_unpacking.py", None ; "comprehension_unpacking_before_python315"
)]
#[test_case(
    "up-traceback-exception-arguments",
    "up_traceback_exception_arguments.py",
    Some("py310") ;
    "traceback_exception_arguments"
)]
#[test_case(
    "up-traceback-exception-arguments",
    "up_traceback_exception_arguments.py",
    Some("py39") ;
    "traceback_exception_arguments_before_python310"
)]
#[test_case("up-exc-info-exception", "up_exc_info_exception.py", Some("py311") ; "exc_info_exception")]
#[test_case("up-exc-info-exception", "up_exc_info_exception.py", Some("py310") ; "exc_info_exception_before_python311")]
#[test_case("up-object-sentinel", "up_object_sentinel.py", Some("py315") ; "object_sentinel")]
#[test_case("up-object-sentinel", "up_object_sentinel.py", None ; "object_sentinel_before_python315")]
#[test_case("up-context-var-token-reset", "up_context_var_token_reset.py", None ; "context_var_token_reset")]
#[test_case(
    "up-context-var-token-reset",
    "up_context_var_token_reset.py",
    Some("py313") ;
    "context_var_token_reset_before_python314"
)]
#[test_case("up-chdir-restore", "up_chdir_restore.py", Some("py311") ; "chdir_restore")]
#[test_case("up-chdir-restore", "up_chdir_restore_future.py", Some("py311") ; "chdir_restore_after_future_import")]
#[test_case("up-chdir-restore", "up_chdir_restore_no_import.py", Some("py311") ; "chdir_restore_without_imports")]
#[test_case("up-chdir-restore", "up_chdir_restore_imported.py", Some("py311") ; "chdir_restore_contextlib_imported")]
#[test_case("up-chdir-restore", "up_chdir_restore_name.py", Some("py311") ; "chdir_restore_contextlib_name_taken")]
#[test_case("up-chdir-restore", "up_chdir_restore.py", Some("py310") ; "chdir_restore_before_python311")]
#[test_case("up-chdir-restore", "up_chdir_restore_shared_line.py", Some("py311") ; "chdir_restore_shared_line")]
#[test_case("up-chdir-restore", "up_chdir_restore_shadow.py", Some("py311") ; "chdir_restore_shadow")]
#[test_case(
    "up-context-var-token-reset",
    "up_context_var_token_reset_shared_line.py",
    None ;
    "context_var_token_reset_shared_line"
)]
#[test_case("up-strptime-date", "up_strptime_date.py", None ; "strptime_date")]
#[test_case("up-strptime-date", "up_strptime_date_name.py", None ; "strptime_date_class_name")]
#[test_case("up-strptime-date", "up_strptime_date_spelled.py", None ; "strptime_date_name_taken")]
#[test_case("up-strptime-date", "up_strptime_date_class_import.py", None ; "strptime_date_class_imported_by_function")]
#[test_case("up-strptime-date", "up_strptime_date_imported.py", None ; "strptime_date_name_imported")]
#[test_case("up-strptime-date", "up_strptime_date_no_class_import.py", None ; "strptime_date_without_class_import")]
#[test_case("up-strptime-date", "up_strptime_date.py", Some("py313") ; "strptime_date_before_python314")]
#[test_case("up-event-loop-in-coroutine", "up_event_loop_in_coroutine.py", None ; "event_loop_in_coroutine")]
#[test_case(
    "up-event-loop-in-coroutine",
    "up_event_loop_in_coroutine.py",
    Some("py313") ;
    "event_loop_in_coroutine_before_python314"
)]
#[test_case(
    "up-event-loop-in-coroutine",
    "up_event_loop_in_coroutine_future.py",
    Some("py313") ;
    "event_loop_in_coroutine_future_import"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction.py", None ; "asyncio_iscoroutinefunction"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_inspect.py",
    None ;
    "asyncio_iscoroutinefunction_imported"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_bound.py",
    None ;
    "asyncio_iscoroutinefunction_bound"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_sorted.py",
    None ;
    "asyncio_iscoroutinefunction_sorted"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_guarded.py",
    None ;
    "asyncio_iscoroutinefunction_guarded"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_name.py",
    None ;
    "asyncio_iscoroutinefunction_name"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_parameter.py",
    None ;
    "asyncio_iscoroutinefunction_parameter"
)]
#[test_case(
    "up-asyncio-iscoroutinefunction",
    "up_asyncio_iscoroutinefunction_shadow.py",
    None ;
    "asyncio_iscoroutinefunction_shadow"
)]
#[test_case("up-codecs-open", "up_codecs_open_comprehension.py", None ; "codecs_open_comprehension")]
#[test_case("up-codecs-open", "up_codecs_open.py", None ; "codecs_open")]
#[test_case("up-codecs-open", "up_codecs_open_shadowed.py", None ; "codecs_open_renamed")]
#[test_case("up-codecs-open", "up_codecs_open_builtin.py", None ; "codecs_open_builtin")]
#[test_case("up-codecs-open", "up_codecs_open_parameter.py", None ; "codecs_open_parameter")]
#[test_case("up-codecs-open", "up_codecs_open_walrus.py", None ; "codecs_open_walrus")]
#[test_case("up-codecs-open", "up_codecs_open_parenthesized.py", None ; "codecs_open_parenthesized")]
#[test_case("up-re-error-alias", "up_re_error_alias.py", None ; "re_error_alias")]
#[test_case("up-re-error-alias", "up_re_error_alias_fallback.py", None ; "re_error_alias_fallback_import")]
#[test_case("up-re-error-alias", "up_re_error_alias_deleted.py", None ; "re_error_alias_deleted_name")]
#[test_case("up-re-error-alias", "up_re_error_alias.py", Some("py312") ; "re_error_alias_before_python313")]
#[test_case(
    "up-argparse-suggest-on-error", "up_argparse_suggest_on_error.py", Some("py315") ; "argparse_suggest_on_error"
)]
#[test_case(
    "up-argparse-suggest-on-error",
    "up_argparse_suggest_on_error.py",
    Some("py310") ;
    "argparse_suggest_on_error_before_python315"
)]
#[test_case(
    "up-argparse-suggest-on-error",
    "up_argparse_suggest_on_error_commented.py",
    Some("py315") ;
    "argparse_suggest_on_error_commented"
)]
#[test_case("up-removed-ast-node", "up_removed_ast_node.py", None ; "removed_ast_node")]
#[test_case("up-removed-ast-node", "up_removed_ast_node_shadowed.py", None ; "removed_ast_node_shadowed")]
#[test_case("up-removed-ast-node", "up_removed_ast_node_parameter.py", None ; "removed_ast_node_parameter")]
#[test_case("early-assignment", "early_assignment.py", None ; "early_assignment")]
#[test_case("early-assignment", "early_assignment_blocked.py", None ; "early_assignment_blocked")]
#[test_case("early-assignment", "early_assignment_comment.py", None ; "early_assignment_comment")]
#[test_case("super-forwarding-method", "super_forwarding_method.py", None ; "super_forwarding_method")]
#[test_case(
    "super-forwarding-method", "super_forwarding_method_decorated.py", None ; "super_forwarding_method_decorated"
)]
#[test_case(
    "super-forwarding-method", "super_forwarding_method_class_body.py", None ; "super_forwarding_method_class_body"
)]
#[test_case(
    "super-forwarding-method",
    "super_forwarding_method_class_body_forms.py",
    None ;
    "super_forwarding_method_class_body_forms"
)]
#[test_case("long-tuple-return", "long_tuple_return.py", None ; "long_tuple_return")]
#[test_case("long-tuple-return", "long_tuple_return_quoted.py", None ; "long_tuple_return_quoted")]
#[test_case("long-tuple-return", "long_tuple_return_class_body.py", None ; "long_tuple_return_class_body")]
#[test_case("cognitive-complexity", "cognitive_complexity.py", None ; "cognitive_complexity")]
#[test_case(
    "collection-built-by-statements", "collection_built_by_statements.py", None ; "collection_built_by_statements"
)]
#[test_case(
    "collection-built-by-statements",
    "collection_built_by_statements_names.py",
    None ;
    "collection_built_by_statements_names"
)]
#[test_case("unnamed-regex-group", "unnamed_regex_group.py", None ; "unnamed_regex_group")]
#[test_case("unreachable-code", "unreachable_code.py", None ; "unreachable_code")]
#[test_case("dead-store", "dead_store.py", None ; "dead_store")]
#[test_case("loop-variable-after-loop", "loop_variable_after_loop.py", None ; "loop_variable_after_loop")]
#[test_case("side-effect-expression", "side_effect_expression.py", None ; "side_effect_expression")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_unknown_flags.py", None ; "unnamed_regex_group_unknown_flags")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_default_scope.py", None ; "unnamed_regex_group_default_scope")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_escaping.py", None ; "unnamed_regex_group_escaping")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_prefixmatch.py", None ; "unnamed_regex_group_prefixmatch")]
#[test_case(
    "unnamed-regex-group", "unnamed_regex_group_scoped_verbose.py", None ; "unnamed_regex_group_scoped_verbose"
)]
#[test_case("unexplained-regex", "unexplained_regex.py", None ; "unexplained_regex")]
#[test_case("unexplained-regex", "unexplained_regex_unknown_flags.py", None ; "unexplained_regex_unknown_flags")]
#[test_case("unexplained-regex", "unexplained_regex_prefixmatch.py", None ; "unexplained_regex_prefixmatch")]
#[test_case("inline-regex", "inline_regex.py", None ; "inline_regex")]
#[test_case("inline-regex", "inline_regex_skipped.py", None ; "inline_regex_skipped")]
#[test_case("inline-regex", "inline_regex_imports.py", None ; "inline_regex_imports")]
#[test_case("inline-regex", "inline_regex_shadowed.py", None ; "inline_regex_shadowed")]
#[test_case("inline-regex", "inline_regex_after_import.py", None ; "inline_regex_after_import")]
#[test_case("inline-regex", "inline_regex_typing.py", None ; "inline_regex_typing")]
#[test_case("inline-regex", "inline_regex_ruff.py", None ; "inline_regex_ruff_plain_text")]
#[test_case("inline-regex", "inline_regex_names.py", None ; "inline_regex_constant_names")]
#[test_case("inline-regex", "inline_regex_nested.py", None ; "inline_regex_nested_calls")]
#[test_case("inline-regex", "inline_regex_alias.py", None ; "inline_regex_alias")]
#[test_case("inline-regex", "inline_regex_final_taken.py", Some("py311") ; "inline_regex_final_taken")]
#[test_case("iterator-reuse", "iterator_reuse.py", None ; "iterator_reuse")]
#[test_case("range-len-loop", "range_len_loop.py", None ; "range_len_loop")]
#[test_case("accumulator-loop", "accumulator_loop.py", None ; "accumulator_loop")]
#[test_case("empty-collection-comparison", "empty_collection_comparison.py", None ; "empty_collection_comparison")]
#[test_case("slice-affix-comparison", "slice_affix_comparison.py", None ; "slice_affix_comparison")]
#[test_case("slice-affix-comparison", "slice_affix_comparison_kinds.py", None ; "slice_affix_comparison_kinds")]
#[test_case(
    "slice-affix-comparison", "slice_affix_comparison_walrus_annotation.py", Some("py311") ;
    "slice_affix_comparison_walrus_annotation"
)]
#[test_case(
    "slice-affix-comparison", "slice_affix_comparison_unpacked_comprehension.py", Some("py315") ;
    "slice_affix_comparison_unpacked_comprehension"
)]
#[test_case("anchored-regex-match", "anchored_regex_match.py", None ; "anchored_regex_match")]
#[test_case("wall-clock-duration", "wall_clock_duration.py", None ; "wall_clock_duration")]
#[test_case("up-utf8-encoding-default", "up_utf8_encoding_default.py", Some("py315") ; "utf8_encoding_default")]
#[test_case("up-utf8-encoding-default", "up_utf8_encoding_default.py", None ; "utf8_encoding_default_before_python315")]
#[test_case(
    "up-utf8-encoding-default",
    "up_utf8_encoding_default_shadowed.py",
    Some("py315") ;
    "utf8_encoding_default_shadowed_open"
)]
#[test_case("up-re-prefixmatch", "up_re_prefixmatch.py", Some("py315") ; "re_prefixmatch")]
#[test_case("up-re-prefixmatch", "up_re_prefixmatch_fallback.py", Some("py315") ; "re_prefixmatch_fallback_import")]
#[test_case("up-re-prefixmatch", "up_re_prefixmatch.py", None ; "re_prefixmatch_before_python315")]
#[test_case("up-compression-modules", "up_compression_modules.py", None ; "compression_modules")]
#[test_case(
    "up-compression-modules", "up_compression_modules.py", Some("py313") ; "compression_modules_before_python314"
)]
#[test_case(
    "up-compression-modules",
    "compression/up_compression_modules_shim.py",
    None ;
    "compression_modules_in_compression_package"
)]
#[test_case("up-compression-modules", "up_compression_modules_lazy.py", Some("py315") ; "compression_modules_lazy")]
#[test_case("up-warnings-deprecated", "up_warnings_deprecated.py", Some("py313") ; "warnings_deprecated")]
#[test_case(
    "up-warnings-deprecated", "up_warnings_deprecated.py", Some("py312") ; "warnings_deprecated_before_python313"
)]
#[test_case("up-deprecated-stdlib-alias", "up_deprecated_stdlib_alias.py", Some("py312") ; "deprecated_stdlib_alias")]
#[test_case(
    "up-deprecated-stdlib-alias", "up_deprecated_stdlib_alias.py", Some("py310") ; "deprecated_stdlib_alias_python310"
)]
#[test_case("shared-mutable-repetition", "shared_mutable_repetition.py", None ; "shared_mutable_repetition")]
#[test_case("duplicate-condition", "duplicate_condition.py", None ; "duplicate_condition")]
#[test_case("ambiguous-operator-chain", "ambiguous_operator_chain.py", None ; "ambiguous_operator_chain")]
#[test_case("failed-swap", "failed_swap.py", None ; "failed_swap")]
#[test_case("in-place-result", "in_place_result.py", None ; "in_place_result")]
#[test_case("unformatted-exception-args", "unformatted_exception_args.py", None ; "unformatted_exception_args")]
#[test_case("shadowed-except-handler", "shadowed_except_handler.py", None ; "shadowed_except_handler")]
#[test_case("dropped-exception-note", "dropped_exception_note.py", None ; "dropped_exception_note")]
#[test_case("expression-complexity", "expression_complexity.py", None ; "expression_complexity")]
#[test_case("comprehension-complexity", "comprehension_complexity.py", None ; "comprehension_complexity")]
#[test_case(
    "comprehension-complexity", "comprehension_complexity.py", Some("py315") ; "comprehension_complexity_python315"
)]
#[test_case("cached-generator", "cached_generator.py", None ; "cached_generator")]
#[test_case("unawaited-coroutine", "unawaited_coroutine.py", None ; "unawaited_coroutine")]
#[test_case("misplaced-arguments", "misplaced_arguments.py", None ; "misplaced_arguments")]
#[test_case("wrapper-without-wraps", "wrapper_without_wraps.py", None ; "wrapper_without_wraps")]
#[test_case("wrapper-without-wraps", "wrapper_without_wraps_from_import.py", None ; "wrapper_without_wraps_from_import")]
#[test_case("wrapper-without-wraps", "wrapper_without_wraps_no_import.py", None ; "wrapper_without_wraps_no_import")]
#[test_case("function-as-condition", "function_as_condition.py", None ; "function_as_condition")]
#[test_case("catastrophic-regex", "catastrophic_regex.py", None ; "catastrophic_regex")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

// Ruff's cleanup merges the `if` the fix leaves alone in an outer `if`, whatever the layout and line length.
#[test_case("if flag:", "        size = len(text)", "if flag and len(text) > 3:" ; "fits")]
#[test_case(
    "if flag:", "        size = len(text                )", "if flag and len(text                ) > 3:" ; "spaced"
)]
#[test_case(
    "if not text:\n        return \"\"\n    elif flag:",
    "        size = len(text)",
    "if not text:\n        return \"\"\n    elif flag and len(text) > 3:"
    ; "elif"
)]
fn walrus_candidate_leaving_collapsible_if_is_merged(outer: &str, assignment: &str, merged: &str) {
    let source =
        format!("def run(text, flag):\n    {outer}\n{assignment}\n        if size > 3:\n            return text\n");
    let root = collapsible_if_project(&source);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    let result = read(root.path(), "module.py");
    assert_eq!(
        (code, stdout.as_str(), result),
        (0, common::ONE_FIXED, format!("def run(text, flag):\n    {merged}\n        return text\n"))
    );
}

#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n        else:\n            \
        return \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n        else:\n            return \"\"\n"
    ; "inner_else"
)]
#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n    else:\n        return \
        \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n    else:\n        return \"\"\n"
    ; "outer_else"
)]
#[test_case(
    "    if flag:  # keep\n        size = len(text)\n        if size > 3:\n            return text\n        else:\
        \n            return \"\"\n",
    "    if flag:  # keep\n        if len(text) > 3:\n            return text\n        else:\n            return \"\"\n"
    ; "inner_else_under_comment"
)]
fn walrus_candidate_keeping_if_branches_is_fixed(body: &str, fixed: &str) {
    let root = collapsible_if_project(&format!("def run(text, flag):\n{body}"));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (0, common::ONE_FIXED, format!("def run(text, flag):\n{fixed}"))
    );
}

// Ruff does not merge an `if` into an outer `if` whose header holds a comment, so the fix would leave work the fix
// loop withdraws. The finding stays, without its fix, and the other candidates of the file are still fixed.
#[test]
fn walrus_candidate_under_a_comment_in_the_outer_if_is_reported_without_a_fix() {
    let source = "def run(text, flag):\n    if flag:  # keep\n        size = len(text)\n        if size > 3:\
        \n            \
                  return text\n    return \"\"\n";
    let root = collapsible_if_project(source);
    let report = "module.py:3:9: SMP502 Inline single-use `size` into the `if` test\nFound 1 error.\n";
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502"]);
    assert_eq!((code, stdout.as_str()), (1, report));
    common::assert_fix_keeps_report(root.path(), "SMP502", &["--fix", "--unsafe-fixes"], "module.py", report);
}

#[test]
fn walrus_candidate_under_a_comment_leaves_the_other_candidates_fixed() {
    let source = "def run(text, flag):\n    if flag:  # keep\n        size = len(text)\n        if size > 3:\
        \n            \
                  return text\n    return \"\"\n\n\ndef first(text):\n    found = text.strip()\n    if found:\n        \
                  return found\n    return text\n";
    let root = collapsible_if_project(source);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (
            1,
            "module.py:3:9: SMP502 Inline single-use `size` into the `if` test\nFound 2 errors (1 fixed, 1 remaining).\
                \n",
            source.replace(
                "    found = text.strip()\n    if found:\n        return found\n",
                "    if found := text.strip():\n        return found\n"
            )
        )
    );
}

// Ruff collapses the nested `if` the walrus leaves only when the first line of the result fits the line length, so the
// unformatted line would decide what a second run, over the formatted one, does. At 84 columns the `if` the walrus
// writes fits, and the merged one does not.
#[test]
fn walrus_candidate_leaving_collapsible_if_is_merged_whatever_the_layout() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff]\nline-length = 84\n[tool.ruff.lint]\nselect = [\"SIM102\"]\n"),
        (
            "module.py",
            "import re\n\n_PATTERN = \"descendant-or-self::label[@for]\"\n\n\ndef find(text: str, flag: bool) \
             -> list[str] | None:\n    if flag:\n        found = re.findall(_PATTERN, text)\n        if found:\
             \n            \
             return found\n    return None\n",
        ),
    ]);
    let select = ["--unsafe-fixes", "--select", "SMP202,SMP502"];
    check(root.path(), &[&["--fix"], &select[..]].concat());
    let second = check(root.path(), &[&["--diff"], &select[..]].concat()).0;
    assert_eq!(
        (read(root.path(), "module.py"), second),
        (
            "import re\n\n\ndef find(text: str, flag: bool) -> list[str] | None:\n    if flag and (found := \
             re.findall(\"descendant-or-self::label[@for]\", text)):\n        return found\n    return None\n"
                .to_owned(),
            0
        )
    );
}

#[test]
fn walrus_candidate_lets_ruff_collapse_the_ifs_its_lines_now_fit() {
    let root = collapsible_if_project(
        "def run(text, flag):\n    size = len(text)\n    if size > 3 and flag:\n        if text.isupper():\
            \n            \
         return text\n",
    );
    check(root.path(), &["--fix", "--select", "SMP502"]);
    let second = check(root.path(), &["--diff", "--select", "SMP502"]).0;
    assert_eq!(
        (read(root.path(), "module.py"), second),
        (
            "def run(text, flag):\n    if len(text) > 3 and flag and text.isupper():\n        return text\n".to_owned(),
            0
        )
    );
}

fn collapsible_if_project(source: &str) -> TempDir {
    let ruff = "[tool.ruff]\nline-length = 40\n\n[tool.ruff.lint]\nselect = [\"SIM102\"]\n";
    common::project(&[("pyproject.toml", ruff), ("module.py", source)])
}

// Before Python 3.9 a decorator is a dotted name, optionally called, so a value that is not one stays where it is.
#[test_case("py38", "registry[0]", "@decorate", None ; "subscript_before_python39")]
#[test_case("py38", "registry.get", "@decorate", Some("@registry.get") ; "dotted_before_python39")]
#[test_case("py38", "registry[0]", "@decorate(1)", None ; "called_before_python39")]
#[test_case("py38", "registry[0]", "@register(decorate)", Some("@register(registry[0])") ; "argument_before_python39")]
#[test_case("py39", "registry[0]", "@decorate", Some("@registry[0]") ; "subscript_from_python39")]
fn single_use_decorator(target: &str, value: &str, decorator: &str, inlined: Option<&str>) {
    let source = format!(
        "def run(registry):\n    decorate = {value}\n    {decorator}\n    def inner():\n        return 1\n    return \
            inner\n"
    );
    let root = single_use_project(target, &source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    let expected = inlined.map_or(source.clone(), |line| {
        format!("def run(registry):\n    {line}\n    def inner():\n        return 1\n    return inner\n")
    });
    assert_eq!(read(root.path(), "module.py"), expected);
}

// Before Python 3.12 a replacement field cannot hold a backslash or the f-string's own quote (PEP 701).
#[test_case(
    "py311",
    "def quote(value):\n    quoted = value.replace(b\"'\", b\"''\")\n    return f\"'{quoted}'\"\n",
    None
    ; "quote_before_python312"
)]
#[test_case(
    "py312",
    "def quote(value):\n    quoted = value.replace(b\"'\", b\"''\")\n    return f\"'{quoted}'\"\n",
    Some("def quote(value):\n    return f\"'{value.replace(b\"'\", b\"''\")}'\"\n")
    ; "quote_from_python312"
)]
#[test_case(
    "py311",
    "def join(items):\n    joined = b\"\\n\".join(items)\n    return f\"{joined}\"\n",
    None
    ; "backslash_before_python312"
)]
#[test_case(
    "py312",
    "def join(items):\n    joined = b\"\\n\".join(items)\n    return f\"{joined}\"\n",
    Some("def join(items):\n    return f\"{b\"\\n\".join(items)}\"\n")
    ; "backslash_from_python312"
)]
// A replacement field before Python 3.12 cannot hold a line break, so a value the source spreads over lines goes in on
// one line, as the formatter joins it.
#[test_case(
    "py311",
    "def join(items):\n    joined = (\n        b', '.join(items)\n    )\n    return f\"Items({joined})\"\n\n\ndef size(\
        items):\n    count = len(items)\n    return f\"{count}\"\n",
    Some("def join(items):\n    return f\"Items({b', '.join(items)})\"\n\n\ndef size(items):\n    return f\"\
        {len(items)}\"\n")
    ; "line_break_before_python312"
)]
#[test_case(
    "py311",
    "def size(items):\n    count = len(items)\n    return f\"{count}\"\n",
    Some("def size(items):\n    return f\"{len(items)}\"\n")
    ; "other_value_before_python312"
)]
// An f-string value in a bare field is spliced into the f-string around it, so its strings keep their quote before
// Python 3.12 too, and the fixes it does not block still apply.
#[test_case(
    "py311",
    "def render(items):\n    prepr = f\"[{','.join(items)}]\"\n    return f\"Items({prepr})\"\n\n\ndef size(items):\
        \n    count = len(items)\n    return f\"{count}\"\n",
    Some("def render(items):\n    return f\"Items([{','.join(items)}])\"\n\n\ndef size(items):\n    return f\"\
        {len(items)}\"\n")
    ; "nested_string_before_python312"
)]
#[test_case(
    "py312",
    "def render(items):\n    prepr = f\"[{','.join(items)}]\"\n    return f\"Items({prepr})\"\n",
    Some("def render(items):\n    return f\"Items([{','.join(items)}])\"\n")
    ; "nested_string_from_python312"
)]
#[test_case(
    "py314",
    "def info(obj):\n    extra = \"\"\n    return f\"data{extra}{obj}\"\n",
    Some("def info(obj):\n    return f\"data{obj}\"\n")
    ; "empty_string_field_is_deleted"
)]
fn single_use_fstring_quote(target: &str, source: &str, inlined: Option<&str>) {
    let root = single_use_project(target, source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), inlined.unwrap_or(source));
}

// `yield name` is never reported. Ruff's RUF070 covers a `yield from` of the assigned name, so SMP504 leaves that pair
// to it whether or not the project selects the rule. The other next statements are inlined.
#[test_case(
    "def run(items):\n    count = len(items)\n    yield count\n",
    "def run(items):\n    count = len(items)\n    yield count\n" ;
    "yield_never_reported"
)]
#[test_case(
    "def run(items):\n    count = len(items)\n    yield from count\n",
    "def run(items):\n    count = len(items)\n    yield from count\n" ;
    "leaves_yield_from_to_ruff"
)]
#[test_case(
    "def run(first, second):\n    pair = first, second\n    yield from pair\n",
    "def run(first, second):\n    pair = first, second\n    yield from pair\n" ;
    "leaves_tuple_yield_from_to_ruff"
)]
#[test_case(
    "def run(first, second):\n    pair = first, second\n    yield pair\n",
    "def run(first, second):\n    pair = first, second\n    yield pair\n" ;
    "tuple_yield_never_reported"
)]
#[test_case(
    "def run(items):\n    count = len(items)\n    print(count)\n",
    "def run(items):\n    print(len(items))\n" ;
    "inlines_into_call"
)]
#[test_case(
    "def run(items):\n    count = len(items)\n    total = count\n    return total\n",
    "def run(items):\n    total = len(items)\n    return total\n" ;
    "inlines_into_assignment"
)]
fn single_use_variable_leaves_ruff_yield_froms_to_ruff(source: &str, fixed: &str) {
    let root = single_use_project("py314", source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), fixed);
}

// `ruff format` closes the gap a removed statement leaves, but a `# fmt: off` file keeps it, so the fix takes one of
// the two blank lines with it.
#[test]
fn single_use_variable_keeps_one_blank_line_under_fmt_off() {
    let source = "# fmt: off\ndef make(flags, kwargs):\n    configure(kwargs)\n\n    cli_kwargs = to_args(kwargs)\n\
        \n    return flags + cli_kwargs\n";
    let root = single_use_project("py314", source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "# fmt: off\ndef make(flags, kwargs):\n    configure(kwargs)\n\n    return flags + to_args(kwargs)\n"
    );
}

// The layout a value has in the source does not decide whether it is inlined: `ruff format` joins it, so a second run
// would inline it, and the fix does the same on the first.
#[test]
fn single_use_variable_judges_a_split_value_by_its_formatted_width() {
    let source = "def f(conn):\n    request = (\n        b\"POST / HTTP/1.1\\r\\n\"\n        b\"Host: example.com\\r\\n\
        \"\n    )\n    conn.receive_data(request)\n";
    let root = single_use_project("py314", source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix", "--unsafe-fixes"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "def f(conn):\n    conn.receive_data(b\"POST / HTTP/1.1\\r\\n\"\n        b\"Host: example.com\\r\\n\")\n"
    );
}

// Before Python 3.12 a value spread over lines goes into a replacement field as `ruff format` joins it, which a magic
// trailing comma stops unless the formatter skips it.
#[test_case(
    "",
    "def pair(first, second):\n    items = (\n        first,\n        second,\n    )\n    return f\"{items}\"\n" ;
    "magic_trailing_comma_keeps_the_value"
)]
#[test_case(
    "[tool.ruff.format]\nskip-magic-trailing-comma = true\n",
    "def pair(first, second):\n    return f\"{(first, second)}\"\n" ;
    "skipped_magic_trailing_comma_joins_the_value"
)]
fn single_use_fstring_value_follows_magic_trailing_comma(format: &str, fixed: &str) {
    let source =
        "def pair(first, second):\n    items = (\n        first,\n        second,\n    )\n    return f\"{items}\"\n";
    let pyproject = format!("[tool.ruff]\ntarget-version = \"py311\"\n{format}");
    let root = common::project(&[("pyproject.toml", pyproject.as_str()), ("module.py", source)]);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix", "--unsafe-fixes"]);
    assert_eq!(read(root.path(), "module.py"), fixed);
}

// A function another module defines on both sides of a version check is the same function when both definitions
// agree, and has no single signature when they do not.
#[test_case("def ready() -> bool:\n        return True", true ; "same_shape")]
#[test_case("def ready(strict: bool) -> bool:\n        return strict", false ; "other_shape")]
fn function_as_condition_reads_a_twice_defined_import(other: &str, reported: bool) {
    let checks = format!(
        "import sys\n\nif sys.version_info >= (3, 12):\n\n    def ready() -> bool:\n        return True\n\nelse:\n\n    \
         {other}\n"
    );
    let root = common::project(&[
        ("checks.py", checks.as_str()),
        ("main.py", "from checks import ready\n\nif ready:\n    print(1)\n"),
    ]);
    let stdout = check(root.path(), &["--no-cache", "--select", "SMP558"]).1;
    assert_eq!(stdout.contains("main.py:3:4"), reported, "{stdout}");
}

// A name a string annotation or a `ForwardRef` reads is read there too, so inlining its value would leave the string
// naming an undefined name.
#[test_case("class Box(Generic[T]):\n        item: \"list[T]\"" ; "string_annotation")]
#[test_case("box = Generic[T]\n    ref = ForwardRef(\"list[T]\")\n    return box, ref" ; "forward_ref")]
#[test_case("box = Generic[T]\n    refs = ForwardRef(str(box)), ForwardRef(\"list[T]\")\n    return refs" ; "forward_ref_among_other_arguments")]
fn single_use_string_annotation_is_a_read(body: &str) {
    let source = format!(
        "from typing import ForwardRef, Generic, TypeVar\n\n\ndef build() -> type:\n    T = TypeVar(\"T\")\n\n    \
            {body}\n"
    );
    let root = single_use_project("py314", &source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert!(read(root.path(), "module.py").contains("    T = TypeVar(\"T\")\n"));
}

// A string annotation that does not parse spells no name, so the name it cannot read is inlined as usual.
#[test]
fn single_use_unparsable_string_annotation_reads_nothing() {
    let root = single_use_project(
        "py314",
        "from typing import Generic, TypeVar\n\n\ndef build() -> type:\n    T = TypeVar(\"T\")\n\n    class Box(Generic\
            [T]):\n        item: \"list[\"\n\n    return Box\n",
    );
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert!(read(root.path(), "module.py").contains("class Box(Generic[TypeVar(\"T\")]):"));
}

fn single_use_project(target: &str, source: &str) -> TempDir {
    let ruff = format!("[tool.ruff]\ntarget-version = \"{target}\"\n");
    common::project(&[("pyproject.toml", ruff.as_str()), ("module.py", source)])
}

// The calls of a function in other files count toward whether each of its defaults is ever left out.
#[test]
fn unused_default_counts_calls_from_other_files() {
    let root = common::project(&[
        ("core.py", "def connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import connect\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(
        (code, stdout.as_str()),
        (
            1,
            "core.py:1:27: SMP506 Every call to `connect` passes `timeout`; drop its default `30`\nFound 1 error.\n1 \
             unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

// A call that suppresses `dataclass-kw-only` still passes its arguments, so it counts like any other call.
#[test_case("connect(\"a\", 1)  # noqa: SMP703\n" ; "noqa")]
#[test_case("connect(\"a\", 1)  # simplint: ignore[dataclass-kw-only] positional on purpose\n" ; "ignore")]
fn unused_default_counts_calls_that_suppress_dataclass_kw_only(call: &str) {
    let caller = format!("from core import connect\n\n{call}");
    let root = common::project(&[
        ("core.py", "def connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", caller.as_str()),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert!(stdout.contains("SMP506 Every call to `connect` passes `timeout`"), "{stdout}");
}

// A library's public functions may be called by outside code that omits the argument.
#[test]
fn unused_default_leaves_the_public_api_alone() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("core.py", "__all__ = [\"connect\"]\n\n\ndef connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import connect\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.as_str()), (0, common::PASSED));
}

// Outside code calls the public methods of a public class, not its private ones, so a private method keeps its
// judgment.
#[test]
fn unused_default_judges_private_methods_of_public_classes() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "core.py",
            "__all__ = [\"Client\"]\n\n\nclass Client:\n    def run(self) -> None:\n        self._verify(\"a\", 1)\n\
                \n    def _verify(self, host, timeout=30):\n        return host, timeout\n",
        ),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.contains("SMP506 Every call to `_verify` passes `timeout`")), (1, true), "{stdout}");
}

// A private function a public module re-exports is part of the API its users call, so outside code may leave out the
// default the run's calls all pass. A private function nothing re-exports is still judged.
#[test]
fn unused_default_leaves_reexported_functions_alone() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("pkg/__init__.py", "from pkg._core import _connect\n\n__all__ = [\"_connect\"]\n"),
        (
            "pkg/_core.py",
            "def _connect(host, timeout=30):\n    return host, timeout\n\n\ndef _helper(host, timeout=30):\n    return \
                host, timeout\n\n\n_connect(\"a\", 5)\n_helper(\"a\", 5)\n",
        ),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(
        (stdout.matches("Every call to").count(), stdout.contains("`_helper`") && !stdout.contains("`_connect`")),
        (1, true),
        "{stdout}"
    );
}

// A package with a dynamic `__all__` still offers the names other files import from it, so the default they pass is the
// library's, and a helper nothing imports is judged.
#[test]
fn unused_default_keeps_names_imported_from_a_package_with_dynamic_all() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("pkg/__init__.py", "from ._impl import make\n\n__all__ = sorted([\"make\"])\n"),
        (
            "pkg/_impl.py",
            "def make(x: int = 0) -> int:\n    return x\n\n\ndef _unused(x: int = 1) -> int:\n    return x\n\n\n_unused\
                (2)\n",
        ),
        ("pkg/cli.py", "from pkg import make\n\nprint(make(2))\n"),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(
        (stdout.matches("Every call to").count(), stdout.contains("`_unused`") && !stdout.contains("`make`")),
        (1, true),
        "{stdout}"
    );
}

// The `__all__` of a library's public module declares its interface, so a public function it leaves out is judged like
// a private one, as the other API rules judge it.
#[test]
fn unused_default_judges_public_functions_all_omits() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "core.py",
            "__all__ = [\"connect\"]\n\n\ndef connect(host, timeout=30):\n    return host, timeout\n\n\ndef helper(host\
                , timeout=30):\n    return host, timeout\n\n\nconnect(\"a\", 1)\nhelper(\"a\", 1)\n",
        ),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(
        (stdout.matches("Every call to").count(), stdout.contains("`helper`") && !stdout.contains("`connect`")),
        (1, true),
        "{stdout}"
    );
}

// A star import hands the module's names to code that the calls of the run do not show.
#[test]
fn unused_default_leaves_star_imported_modules_alone() {
    let root = common::project(&[
        ("core.py", "def connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import *\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.as_str()), (0, common::PASSED));
}

// PT028 reports every default of a pytest test function, with a display-only fix, so a default this rule cannot fix is
// left to it there. A default it can fix is reported in a test as anywhere else.
#[test_case(
    "def test_connect(host, timeout=30):\n    return host, timeout\n\n\ntest_connect(\"db\", 5)\n",
    1 ;
    "fixable_default_of_test"
)]
#[test_case(
    "def test_pair(first=1, second=2):\n    return first, second\n\n\ntest_pair(1, 2)\n",
    1 ;
    "unfixable_default_of_test"
)]
#[test_case(
    "def pair(first=1, second=2):\n    return first, second\n\n\npair(1, 2)\n",
    2 ;
    "unfixable_default_of_other_function"
)]
#[test_case(
    "class TestPair:\n    def test_pair(self, first=1, second=2):\n        return first, second\n\n    def run(self):\
        \n        return self.test_pair(1, 2)\n",
    1 ;
    "unfixable_default_of_test_method"
)]
#[test_case(
    "class TestPair:\n    def __init__(self) -> None:\n        pass\n\n    def test_pair(self, first=1, second=2):\
        \n        return first, second\n\n    def run(self):\n        return self.test_pair(1, 2)\n",
    2 ;
    "test_method_of_class_with_init"
)]
#[test_case(
    "class Pair:\n    def test_pair(self, first=1, second=2):\n        return first, second\n\n    def run(self):\
        \n        return self.test_pair(1, 2)\n",
    2 ;
    "test_method_of_class_not_named_test"
)]
fn unused_default_leaves_unfixable_defaults_of_pytest_tests_to_pt028(source: &str, reports: usize) {
    let root = common::project(&[("module.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(stdout.matches("Every call to").count(), reports, "{stdout}");
}

// Ruff's PT017 reports an `assert` naming the exception, at any depth in the handler, so SMP507 leaves it to Ruff
// whether or not the project selects PT017. An `if` is not an `assert`, so its comparison stays with the rule.
#[test_case("assert \"gone\" in str(exc)", 0 ; "assert_left_to_ruff")]
#[test_case("if flag:\n            assert \"gone\" in str(exc)", 0 ; "nested_assert_left_to_ruff")]
#[test_case("if \"gone\" in str(exc):\n            pass", 1 ; "if_reported")]
fn error_message_matching_leaves_asserts_to_pt017(body: &str, reports: usize) {
    let source = format!("def run(f):\n    try:\n        f()\n    except OSError as exc:\n        {body}\n");
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// The text of two exceptions tells them apart by their wording alone, so only the text of one against a literal or a
// plain variable is reported.
#[test_case("return exc.args == other.args", 0 ; "args_of_two_exceptions")]
#[test_case("return str(exc) == str(other)", 0 ; "text_of_two_exceptions")]
#[test_case("return str(exc) == \"boom\"", 1 ; "text_against_literal")]
#[test_case("return exc.args == expected", 1 ; "args_against_variable")]
#[test_case(
    "try:\n            f()\n        except OSError as inner:\n            return str(exc) == str(inner)",
    0 ;
    "text_of_nested_handlers"
)]
#[test_case("return str(exc) == str(other) == \"boom\"", 1 ; "text_chained_with_literal")]
#[test_case("return \"boom\" == str(exc) == str(other)", 1 ; "literal_chained_before_text")]
#[test_case("other = \"timed out\"\n        return str(exc) == str(other)", 1 ; "parameter_rebound_to_text")]
#[test_case(
    "def inner(other):\n            return str(exc) == str(other)\n        return inner(expected)",
    1 ;
    "shadowed_by_unannotated_parameter"
)]
#[test_case("return (lambda other: str(exc) == str(other))(expected)", 1 ; "shadowed_by_lambda_parameter")]
#[test_case("return (lambda: str(exc) == str(other))()", 0 ; "lambda_reads_outer_parameter")]
fn error_message_matching_leaves_text_pairs_alone(body: &str, reports: usize) {
    let source = format!(
        "def run(f, other: OSError, expected):\n    try:\n        f()\n    except OSError as exc:\n        {body}\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// The parameters of a method, reached through its class, are read the same way as those of a function.
#[test_case("return replay_error.args == error.args", 0 ; "args_of_a_parameter")]
#[test_case("return replay_error.args == expected", 1 ; "args_against_variable")]
fn error_message_matching_reads_parameters_of_methods(body: &str, reports: usize) {
    let source = format!(
        "class Replay:\n    def run(self, target, error: OSError, expected):\n        try:\n            target()\
            \n        except OSError as replay_error:\n            {body}\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// A name the file does not bind is not the handler's exception, even when its text is the same as the handler's name
// would be; a name a star import brings in is not the exception either.
#[test_case("return str(undefined_name) == \"boom\"", 0 ; "unresolved_name")]
#[test_case("return str(sep) == \"/\"", 0 ; "name_from_star_import")]
#[test_case("return str(exc) == \"boom\"", 1 ; "handler_name_still_reported")]
fn error_message_matching_leaves_unresolved_names_alone(body: &str, reports: usize) {
    let source = format!(
        "from os.path import *\n\n\ndef run(f):\n    try:\n        f()\n    except OSError as exc:\n        {body}\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// A string annotation names the class the file binds under that name, which shadows the builtin of the same name.
#[test_case("\"ValueError\"", 1 ; "string_names_shadowed_builtin_without_exception")]
#[test_case("\"OSError\"", 0 ; "string_names_builtin_exception")]
#[test_case("\"int\"", 1 ; "string_names_builtin_without_exception")]
fn error_message_matching_reads_string_annotations_in_the_file(annotation: &str, reports: usize) {
    let source = format!(
        "class ValueError:\n    pass\n\n\ndef run(f, other: {annotation}):\n    try:\n        f()\n    except OSError \
            as exc:\n        return str(exc) == str(other)\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// A parameter counts as an exception through its annotation: a class of the file, an exception in a union with `None`,
// in `Optional`, or in a string annotation. An annotation that names no exception leaves the comparison reported.
#[test_case("Deep", 0 ; "two_level_local_subclass")]
#[test_case("Exception | None", 0 ; "union_with_none")]
#[test_case("Optional[Deep]", 0 ; "optional")]
#[test_case("\"Deep\"", 0 ; "string_annotation")]
#[test_case("\"Optional[Deep]\"", 0 ; "optional_in_string_annotation")]
#[test_case("int | None", 1 ; "union_without_exception")]
fn error_message_matching_reads_annotations(annotation: &str, reports: usize) {
    let source = format!(
        "from typing import Optional\n\n\nclass Local(Exception):\n    pass\n\n\nclass Deep(Local):\n    pass\n\n\ndef \
            run(f, other: {annotation}):\n    try:\n        f()\n    except OSError as exc:\n        return str(exc) =\
            = str(other)\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// An exception class the parameter's annotation imports from another file of the run.
#[test_case("class MyError(Exception):\n    pass\n", 0 ; "imported_exception_class")]
#[test_case("class MyError:\n    pass\n", 1 ; "imported_class_is_not_an_exception")]
fn error_message_matching_reads_imported_exceptions(definition: &str, reports: usize) {
    let module = "from errors import MyError\n\n\ndef run(f, other: MyError):\n    try:\n        f()\n    except \
        OSError as exc:\n        return str(exc) == str(other)\n";
    let root = common::project(&[("errors.py", definition), ("module.py", module)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// A class nested in a function is an exception class of the file, whether the annotation names it or a string does.
#[test_case("Local", 0 ; "nested_class_name")]
#[test_case("\"Local\"", 0 ; "nested_class_string")]
#[test_case("int", 1 ; "nested_control_without_exception")]
fn error_message_matching_reads_nested_exception_classes(annotation: &str, reports: usize) {
    let source = format!(
        "def outer():\n    class Local(Exception):\n        pass\n\n    def inner(f, other: {annotation}):\n        try\
            :\n            f()\n        except OSError as exc:\n            return str(exc) == str(other)\n    return \
            inner\n"
    );
    let root = common::project(&[("module.py", &source)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// Test modules may assert on the messages their code raises. An exception class of the file counts as an exception type
// for a parameter annotation. A class whose base name is rebound to a class deriving from it is followed through both
// bindings, and stops at the cycle, so the first binding still shows the exception.
#[test_case(
    "test_errors.py",
    "def test_load():\n    try:\n        load()\n    except OSError as exc:\n        assert \"gone\" in str(exc)\n" ;
    "test_module"
)]
#[test_case(
    "module.py",
    "class Offset(Exception):\n    pass\n\n\ndef replay(target, error: Offset):\n    try:\n        target()\n    \
     except Offset as replay_error:\n        return replay_error.args == error.args\n" ;
    "exception_class_of_the_file"
)]
#[test_case(
    "module.py",
    "def outer():\n    class B(Exception):\n        pass\n\n    class A(B):\n        pass\n\n    class B(A):\n        \
     pass\n\n    def inner(f, other: A):\n        try:\n            f()\n        except OSError as exc:\n            \
     return str(exc) == str(other)\n    return inner\n" ;
    "rebound_class_cycle"
)]
fn error_message_matching_leaves_alone(file: &str, source: &str) {
    let root = common::project(&[(file, source)]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!((code, stdout.as_str()), (0, common::PASSED));
}

// A class the run defines that a star import brings into the module, which the annotation then names.
#[test_case("class AppError(Exception):\n    pass\n", 0 ; "star_imported_exception_class")]
#[test_case("class AppError:\n    pass\n", 1 ; "star_imported_class_is_not_an_exception")]
fn error_message_matching_reads_star_imported_exceptions(definition: &str, reports: usize) {
    let module = "from mod_a import *\n\n\ndef run(f, other: AppError):\n    try:\n        f()\n    except OSError as \
        exc:\n        return str(exc) == str(other)\n";
    let root = common::project(&[("mod_a.py", definition), ("module.py", module)]);
    common::assert_report_count(root.path(), "SMP507", reports);
}

// The tuple length and the complexity a project allows before the rules report, read from `[tool.simplint.logic]`.
#[test_case("", 2 ; "default_limit_reports_triples")]
#[test_case("[tool.simplint.logic]\nmax-tuple-elements = 3\n", 1 ; "limit_three_reports_quadruples")]
#[test_case("[tool.simplint.logic]\nmax-tuple-elements = 4\n", 0 ; "limit_four_reports_nothing")]
fn long_tuple_return_limit(settings: &str, reports: usize) {
    let source = "def triple() -> tuple[int, int, int]:\n    return 1, 2, 3\n\n\ndef quad() -> tuple[int, int, int, int\
        ]:\n    return 1, 2, 3, 4\n";
    let root = common::project(&[("pyproject.toml", settings), ("module.py", source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP528"]);
    assert_eq!(stdout.matches("SMP528").count(), reports);
}

#[test_case("from typing import override", "@override", 0 ; "typing_override")]
#[test_case("from typing_extensions import override", "@override", 0 ; "typing_extensions_override")]
#[test_case("from compatibility import override", "@override", 0 ; "compatibility_override")]
#[test_case("", "", 1 ; "unconstrained_return")]
fn long_tuple_return_respects_override_contract(import: &str, decorator: &str, reports: usize) {
    let root = common::project(&[(
        "module.py",
        &format!(
            "{import}\n\nclass Base:\n    def fields(self) -> tuple[int, int, int]:\n        return 1, 2, 3\n\n\nclass \
                Child(Base):\n    {decorator}\n    def fields(self) -> tuple[int, int, int]:\n        return 1, 2, 3\n"
        ),
    )]);
    common::assert_report_count(root.path(), "SMP528", reports + 1);
}

#[test_case(
    "PLR1702",
    "max-nested-blocks = 1",
    "if value:\n        return _helper(value)\n    return 0",
    "if value:\n        return 1\n    return 0" ;
    "nested_blocks"
)]
#[test_case(
    "PLW0717",
    "max-statements-in-try = 0",
    "try:\n        return _helper(value)\n    except ValueError:\n        return 0",
    "copy = value\n    return copy" ;
    "try_clause"
)]
#[test_case(
    "TRY301",
    "",
    "try:\n        return _helper(value)\n    except ValueError:\n        return 0",
    "if value:\n        raise ValueError\n    return 0" ;
    "raise_in_try"
)]
fn single_caller_helper_respects_other_ruff_rules(rule: &str, option: &str, caller: &str, helper: &str) {
    let config =
        format!("[tool.ruff.lint]\npreview = true\nselect = [\"{rule}\"]\n[tool.ruff.lint.pylint]\n{option}\n");
    let source = format!("def _helper(value):\n    {helper}\n\n\ndef run(value):\n    {caller}\n");
    let root = common::project(&[("pyproject.toml", &config), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP505"]);
    assert!(!stdout.contains("`_helper` is called only"), "{stdout}");
}

#[test_case("raise ValueError('bad')", "except ValueError:" ; "constructed_exception")]
#[test_case("raise ValueError", "except:" ; "catch_all")]
#[test_case("raise ValueError", "except (ValueError, TypeError):" ; "tuple_handler")]
#[test_case("raise ValueError", "except Exception:" ; "broad_handler")]
fn single_caller_helper_keeps_caught_raises_outside_try(raised: &str, handler: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"TRY301\"]\n"),
        (
            "module.py",
            &format!(
                "def _helper(value):\n    if value:\n        {raised}\n    return 0\n\n\ndef run(value):\n    try:\
                    \n        return _helper(value)\n    {handler}\n        return 0\n"
            ),
        ),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP505"]);
    assert!(!stdout.contains("`_helper` is called only"), "{stdout}");
}

#[test_case("TRY301", "", "OSError", "if value:\n        raise ValueError\n    return 0" ; "unrelated_handler")]
#[test_case(
    "PLR1702",
    "max-nested-blocks = 2",
    "ValueError",
    "if value:\n        return 1\n    return 0" ;
    "nested_limit_not_crossed"
)]
#[test_case(
    "PLW0717", "max-statements-in-try = 2", "ValueError", "copy = value\n    return copy" ; "try_limit_not_crossed"
)]
fn single_caller_helper_reports_when_ruff_limits_hold(rule: &str, option: &str, handled: &str, helper: &str) {
    let config =
        format!("[tool.ruff.lint]\npreview = true\nselect = [\"{rule}\"]\n[tool.ruff.lint.pylint]\n{option}\n");
    let source = format!(
        "def _helper(value):\n    {helper}\n\n\ndef run(value):\n    try:\n        return _helper(value)\n    except \
            {handled}:\n        return 0\n"
    );
    let root = common::project(&[("pyproject.toml", &config), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP505"]);
    assert!(stdout.contains("`_helper` is called only"), "{stdout}");
}

// A function scoring 18 is reported above a limit of 17 and left alone at 18, since the limit is the highest score
// kept.
#[test_case("", 1 ; "default_limit_of_twenty_reports_twenty_one")]
#[test_case("[tool.simplint.logic]\nmax-cognitive-complexity = 17\n", 2 ; "limit_seventeen_reports_eighteen")]
#[test_case("[tool.simplint.logic]\nmax-cognitive-complexity = 18\n", 1 ; "limit_eighteen_keeps_eighteen")]
fn cognitive_complexity_limit(settings: &str, reports: usize) {
    let source = common::fixture_source("logic/cognitive_complexity/cognitive_complexity.py");
    let root = common::project(&[("pyproject.toml", settings), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP529"]);
    assert_eq!(stdout.matches("SMP529").count(), reports);
}

// Each function in the fixture ends its name with the score SonarSource gives it, which a limit of zero reports.
#[test]
fn cognitive_complexity_scores() {
    let source = common::fixture_source("logic/cognitive_complexity/cognitive_complexity_scores.py");
    let settings = "[tool.simplint.logic]\nmax-cognitive-complexity = 0\n";
    let root = common::project(&[("pyproject.toml", settings), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP529"]);
    let scores: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| {
            let name = line.split('`').nth(1)?;
            let score = line.split("complexity ").nth(1)?.split(',').next()?;
            Some((name.rsplit('_').next()?, score))
        })
        .collect();
    let expected: Vec<(&str, &str)> = source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("def ")?.split('(').next()?.rsplit('_').next())
        .filter(|score| *score != "0")
        .map(|score| (score, score))
        .collect();
    assert_eq!(scores, expected);
}

// Ruff's RET504 reports `return name` after an assignment, and RUF070 reports a `yield from` of one, so SMP504 leaves
// those pairs to them whether or not the project selects them. RET504 skips a generator, so the return in one is kept.
#[test_case("def f() -> int:\n    total = 1 + 2\n    return total\n", false ; "return")]
#[test_case("def f():\n    values = make()\n    yield from values\n", false ; "yield_from")]
#[test_case("def f():\n    yield 0\n    total = 1 + 2\n    return total\n", true ; "return_in_generator")]
#[test_case(
    "def f():\n    get = lambda: (yield)\n    total = 1 + 2\n    return total\n", true ; "return_with_lambda_yield"
)]
#[test_case(
    "def f():\n    def inner():\n        yield 1\n    total = 1 + 2\n    return total\n",
    false ;
    "return_with_nested_generator"
)]
#[test_case(
    "def f():\n    def inner() -> (yield):\n        return 1\n    total = 1 + 2\n    return total\n",
    false ;
    "return_with_nested_annotation_yield"
)]
#[test_case(
    "def f():\n    @(yield)\n    def inner():\n        return 1\n    total = 1 + 2\n    return total\n",
    true ;
    "return_with_nested_decorator_yield"
)]
#[test_case(
    "def f():\n    @(yield)\n    class Inner:\n        pass\n    total = 1 + 2\n    return total\n",
    true ;
    "return_with_class_decorator_yield"
)]
fn single_use_variable_leaves_ruff_returns_and_yield_froms(source: &str, reported: bool) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP504"]).1;
    assert_eq!(
        stdout.contains("`total` is used once") || stdout.contains("`values` is used once"),
        reported,
        "{stdout}"
    );
}

// Ruff's B006 and B008 report a value that becomes a parameter's default, so SMP504 keeps a variable whose value would
// become one while the project's Ruff configuration runs the rule, and still inlines what they leave alone.
#[test_case("B008", "", "make()", "held", true, false ; "call_with_b008")]
#[test_case("B008", "", "make()", "held", false, true ; "call_without_b008")]
#[test_case("B008", "", "tuple()", "held", true, true ; "immutable_call")]
#[test_case("B008", "", "lambda: make()", "held", true, true ; "call_in_lambda")]
#[test_case("B008", "", "3", "held", true, true ; "plain_value")]
#[test_case("B008", "", "make()", "held: tuple[int, ...]", true, true ; "immutable_annotation")]
#[test_case("B008", "", "UserId(3)", "held", true, true ; "newtype_of_immutable")]
#[test_case(
    "B008",
    "[tool.ruff.lint.flake8-bugbear]\nextend-immutable-calls = [\"frozen.freeze\"]\n",
    "freeze()",
    "held",
    true,
    true ;
    "extended_immutable_call"
)]
#[test_case("B008", "", "freeze()", "held", true, false ; "call_not_extended")]
#[test_case("B006", "", "[]", "held", true, false ; "list_literal")]
#[test_case("B006", "", "[]", "held", false, true ; "list_literal_without_b006")]
#[test_case("B006", "", "list()", "held", true, false ; "list_call")]
#[test_case("B006", "", "list()", "held", false, true ; "list_call_without_b006")]
#[test_case("B006", "", "collections.Counter()", "held", true, false ; "counter_call")]
#[test_case("B006", "", "collections.Counter()", "held", false, true ; "counter_call_without_b006")]
#[test_case("B006", "", "{key: 1 for key in range(3)}", "held", true, false ; "dict_comprehension")]
#[test_case("B006", "", "{key: 1 for key in range(3)}", "held", false, true ; "dict_comprehension_without_b006")]
#[test_case("B006", PREVIEW, "([],)", "held", true, false ; "tuple_of_list")]
#[test_case("B006", "", "([],)", "held", true, true ; "tuple_of_list_without_preview")]
#[test_case("B006", PREVIEW, "([],)", "held", false, true ; "tuple_of_list_without_b006")]
#[test_case("B006", PREVIEW, "(key for key in range(3))", "held", true, false ; "generator")]
#[test_case("B006", "", "(key for key in range(3))", "held", true, true ; "generator_without_preview")]
#[test_case("B006", PREVIEW, "(key for key in range(3))", "held", false, true ; "generator_without_b006")]
#[test_case("B006", "", "[]", "held: tuple[int, ...]", true, true ; "immutable_annotation_b006")]
fn single_use_variable_follows_ruff_defaults(
    code: &str,
    extra: &str,
    value: &str,
    parameter: &str,
    selected: bool,
    reported: bool,
) {
    let source = format!(
        "import collections\nfrom frozen import freeze\nfrom typing import NewType\n\nUserId = NewType(\"UserId\", int)\
            \n\n\ndef make() -> list[int]:\n    return []\n\n\ndef f() -> None:\n    lock = {value}\n\n    def g(\
            {parameter}=lock) -> None:\n        print(held)\n"
    );
    let select = if selected { format!("\"{code}\"") } else { String::new() };
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n{extra}")),
        ("module.py", source.as_str()),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP504"]).1;
    assert_eq!(stdout.contains("`lock` is used once"), reported, "{stdout}");
}

// B006 reports a generator and a tuple holding a mutable value only in preview.
const PREVIEW: &str = "[tool.ruff]\npreview = true\n";

// Ruff's G003 reports a `+` that is a logging call's message, and its ISC003 a concatenation of two literals across
// lines, so SMP503 leaves those to Ruff whether or not the project selects them.
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"Hello, \" + name)\n",
    false ;
    "logging_message"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(msg=\"Hello, \" + name)\n",
    false ;
    "logging_keyword_message"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.log(1, \"Hello, \" + name)\n",
    false ;
    "logging_log_message"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"Hello, \" + name + \"!\")\n",
    false ;
    "logging_chain_message"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.log(\"Hello, \" + name)\n",
    true ;
    "logging_log_without_message"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"a\", \"Hello, \" + name)\n",
    true ;
    "logging_second_argument"
)]
#[test_case(
    "",
    "import logging\n\n\ndef f(name: str) -> None:\n    logging.info((\"Hello, \" + name) * 2)\n",
    true ;
    "logging_repeated_message"
)]
#[test_case("", "def f(name: str) -> None:\n    foo.info(\"Hello, \" + name)\n", true ; "not_a_logger")]
#[test_case("", "def f(name: str) -> None:\n    logger.info(\"Hello, \" + name)\n", false ; "logger_named_variable")]
#[test_case(
    "logger-objects = [\"mypkg.logger\"]\n",
    "from mypkg import logger\n\n\ndef f(name: str) -> None:\n    logger.info(\"Hello, \" + name)\n",
    false ;
    "logger_object"
)]
#[test_case(ISC003, "x = (\n    \"Hello, \"\n    + \"world\"\n)\n", false ; "literal_pair_across_lines")]
#[test_case("", "x = (\n    \"Hello, \"\n    + \"world\"\n)\n", true ; "literal_pair_across_lines_without_isc003")]
#[test_case(ISC003, "x = \"Hello, \" + \"world\"\n", true ; "literal_pair_on_one_line")]
#[test_case(ISC003, "x = (\n    \"Hello, \"\n    + \"world\"\n    + name\n)\n", false ; "pair_at_start_of_chain")]
#[test_case(ISC003, "x = (\n    \"Hello, \"\n    + name\n)\n", true ; "name_in_pair")]
#[test_case(ISC003, "x = (\n    (\"Hello, \")\n    + \"world\"\n)\n", true ; "parenthesized_left_of_pair")]
#[test_case(ISC003, "x = (\n    \"Hello, \"\n    + (\"world\")\n)\n", true ; "parenthesized_right_of_pair")]
#[test_case(
    "extend-select = [\"ISC003\"]\n[tool.ruff.lint.flake8-implicit-str-concat]\nallow-multiline = false\n",
    "x = (\n    \"Hello, \"\n    + \"world\"\n)\n",
    true ;
    "multiline_not_allowed"
)]
fn string_concatenation_follows_ruff(extra: &str, source: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = []\n{extra}")),
        ("module.py", source),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
    assert_eq!(stdout.contains("Build text with an f-string"), reported, "{stdout}");
}

// `ruff format` joins a pair ISC003 reports onto one line, where ISC003 no longer reports it, so SMP503 leaves the pair
// to Ruff only while Ruff runs ISC003.
const ISC003: &str = "extend-select = [\"ISC003\"]\n";

// A class of the project is skipped under any name, and a function of the project is reported even when its name is
// CapWords, since the project's own definitions tell the two apart. An import from outside the project is a class only
// when its name is CapWords.
#[test_case("Badge(\"x\")", 0 ; "project_class")]
#[test_case("badge(\"x\")", 0 ; "project_class_lowercase")]
#[test_case("Label(\"x\")", 1 ; "project_function_capwords")]
#[test_case("Markup(\"x\")", 0 ; "external_capwords")]
#[test_case("minify(\"x\")", 1 ; "external_lowercase")]
#[test_case("re.escape(\"x\")", 1 ; "external_module_function")]
#[test_case("_Hidden(\"x\")", 0 ; "external_underscore_capwords")]
#[test_case("shapes.badge(\"x\")", 0 ; "project_module_class_lowercase")]
#[test_case("shapes.Badge(\"x\")", 0 ; "project_module_class")]
#[test_case("shapes.Label(\"x\")", 1 ; "project_module_function_capwords")]
#[test_case("tags.Thing(\"x\")", 0 ; "external_module_capwords")]
#[test_case("tags.thing(\"x\")", 1 ; "external_module_lowercase")]
fn string_concatenation_imported_callee(call: &str, reported: usize) {
    let shapes = "class Badge(str):\n    pass\n\n\nclass badge(str):\n    pass\n\n\ndef Label(text: str) -> str:\n    \
        return text\n";
    let source = format!(
        "import re\nimport shapes\nimport tags\n\nfrom markupsafe import Markup\nfrom shapes import Badge, Label, badge\
            \nfrom tags import _Hidden, minify\n\n\ndef f() -> list[str]:\n    return [{call} + \"!\"]\n"
    );
    let root = common::project(&[("shapes.py", shapes), ("module.py", source.as_str())]);
    let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// A file checked alone is read against the rest of the project only as far as the run goes, so an import of a file
// the run does not read is told apart by its name alone.
#[test_case("Badge(\"x\")", 0 ; "unread_capwords")]
#[test_case("badge(\"x\")", 1 ; "unread_lowercase")]
fn string_concatenation_unread_import(call: &str, reported: usize) {
    let shapes = "class Badge(str):\n    pass\n\n\nclass badge(str):\n    pass\n";
    let source = format!("from shapes import Badge, badge\n\n\ndef f() -> list[str]:\n    return [{call} + \"!\"]\n");
    let root = common::project(&[("shapes.py", shapes), ("module.py", source.as_str())]);
    let stdout = common::check(root.path(), &["--select", "SMP503", "module.py"]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// An import the project re-exports from outside it binds a class by its name alone, while one the project re-exports
// from its own module binds what that module defines: a function is reported even when its name is CapWords.
#[test_case("from markupsafe import Markup\n", "Markup(\"x\")", 0 ; "reexported_from_outside")]
#[test_case("from pkg.funcs import Wrap\n", "Wrap(\"x\")", 1 ; "reexported_function")]
fn string_concatenation_reexported_import(init: &str, call: &str, reported: usize) {
    let funcs = "def Wrap(text: str) -> str:\n    return text\n";
    let source = format!("from pkg import Markup, Wrap\n\n\ndef f() -> list[str]:\n    return [{call} + \"!\"]\n");
    let root = common::project(&[("pkg/__init__.py", init), ("pkg/funcs.py", funcs), ("module.py", source.as_str())]);
    let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// A parameter annotated with an imported class is an instance of it, whether the run defines the class or the name is
// spelled as one, and an annotation naming a class through a module the code imports is spelled as one too.
#[test]
fn string_concatenation_annotated_import() {
    let shapes = "class Badge(str):\n    pass\n";
    let source = "import markupsafe\n\nfrom shapes import Badge\n\n\ndef f(safe: Badge, outside: markupsafe.Markup) -> \
        list[str]:\n    return [safe + \"!\", outside + \"!\"]\n";
    let root = common::project(&[("shapes.py", shapes), ("module.py", source)]);
    let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), 0, "{stdout}");
}

// An import inside a function binds its name as the import at module level does, so the class it names is told apart
// in the same way.
#[test_case("from markupsafe import Markup as Local", "Local(\"x\")", 0 ; "function_import_capwords")]
#[test_case("from tags import thing as Local", "Local(\"x\")", 1 ; "function_import_lowercase")]
fn string_concatenation_function_import(import: &str, call: &str, reported: usize) {
    let source = format!("def f() -> list[str]:\n    {import}\n    return [{call} + \"!\"]\n");
    let root = common::project(&[("module.py", source.as_str())]);
    let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// Before Python 3.12 a replacement field cannot hold a backslash inside a string, so a chain whose expression has one
// has no f-string for that target and is not reported. A line continuation outside any string is not such a backslash,
// and a quote conflict only declines the fix, since a triple-quoted f-string holds both quote characters.
#[test_case(
    "py311",
    "def render(tail: str, names: list[str]) -> str:\n    return \"<p>\" + tail + \"</p>\" + '\", \"'.join(names) + \"'\
        \"\n",
    1 ;
    "both_quotes_before_python312"
)]
#[test_case(
    "py312",
    "def render(tail: str, names: list[str]) -> str:\n    return \"<p>\" + tail + \"</p>\" + '\", \"'.join(names) + \"'\
        \"\n",
    1 ;
    "both_quotes_from_python312"
)]
#[test_case(
    "py311", "def f(name: str) -> str:\n    return \"a\" + name.strip(\"\\n\")\n", 0 ; "backslash_before_python312"
)]
#[test_case(
    "py311",
    "def f(name: str) -> str:\n    return \"it's \\\"x\\\"\" + name\n",
    1 ;
    "string_operand_with_both_quotes_before_python312"
)]
#[test_case(
    "py311",
    "def f(name: str) -> str:\n    return 'say \"hi\"' + \"it's\" + name\n",
    1 ;
    "clashing_string_operands_before_python312"
)]
#[test_case(
    "py311",
    "def f(name: str) -> str:\n    return \"a\" + name \\\n    .strip()\n",
    1 ;
    "line_continuation_before_python312"
)]
#[test_case(
    "py311",
    "def f(name: str) -> str:\n    return \"x\" + name.join('''a\n    b''')\n",
    1 ;
    "multiline_expression_rendered_escape_before_python312"
)]
#[test_case(
    "py311",
    "def f(name: str) -> str:\n    return \"\"\"a\"\"\" + name.strip('\"') + \"!\"\n",
    1 ;
    "triple_quoted_operand_before_python312"
)]
fn string_concatenation_before_python312(target: &str, source: &str, reported: usize) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--select", "SMP503", "--target-version", target]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// FURB113 fixes two or more appends to a list only while they run one after another with no comment between them. The
// run leaves a list that Ruff fixes to Ruff, and keeps the rest. No Ruff rule is selected here, so the skip does not
// depend on the selection.
#[test_case("xs = []\nxs.append(1)\nxs.append(2)\n", 0 ; "repeated_appends_left_to_ruff")]
#[test_case("xs = []\nxs.append(1)\n", 1 ; "single_append_kept")]
#[test_case("ys = []\nxs = []\nxs.append(1)\nys.append(2)\nxs.append(3)\n", 1 ; "append_to_other_list_between")]
#[test_case("xs = []\nxs.append(1)\n# the second\nxs.append(g())\n", 1 ; "comment_between_appends")]
#[test_case(
    "xs = []\nxs.append(1)\nxs.append(2)\nxs = []\nxs.append(3)\nxs.append(4)\n", 1 ; "rebound_second_run_kept"
)]
#[test_case(
    "def build() -> list[int]:\n    xs = []\n    xs.append(1)\n    xs.append(2)\n    return xs\n", 0 ; "function_scope"
)]
#[test_case(
    "def build(xs: list[int]) -> None:\n    xs = []\n    xs.append(1)\n    xs.append(2)\n", 1 ; "parameter_rebound_kept"
)]
#[test_case("xs = []\nxs.append(1)\nprint(xs)\nxs.append(2)\n", 1 ; "call_between")]
#[test_case("xs = []\nxs.append(1)\nxs\nxs.append(2)\n", 1 ; "expression_between")]
#[test_case("xs = []\nxs.append(1)\nxs.copy().append(0)\nxs.append(2)\n", 1 ; "append_on_call_between")]
#[test_case("xs = []\nxs.append(1)\nxs.insert(0, 0)\nxs.append(2)\n", 1 ; "insert_between")]
fn collection_built_by_statements_leaves_repeated_appends_to_ruff(source: &str, reported: usize) {
    let root = common::project(&[("module.py", source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP531", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP531\"").count(), reported, "{stdout}");
}

// RUF055 rewrites a `re.match` call with a plain-text pattern that is only tested for truth into `str.startswith`, so
// SMP522 leaves those calls to Ruff whether or not the project selects it, and keeps the rest.
#[test_case("if re.match(\"ab\", text):\n        pass", 0 ; "if_test")]
#[test_case("while re.match(\"ab\", text):\n        break", 0 ; "while_test")]
#[test_case("if text:\n        pass\n    elif re.match(\"ab\", text):\n        pass", 0 ; "elif_test")]
#[test_case("assert re.match(\"ab\", text)", 0 ; "assert_test")]
#[test_case("assert text, re.match(\"ab\", text)", 1 ; "assert_message")]
#[test_case("return re.match(\"ab\", text)", 1 ; "value_used")]
#[test_case("return not re.match(\"ab\", text)", 0 ; "not_operand")]
#[test_case("return bool(re.match(\"ab\", text))", 0 ; "bool_argument")]
#[test_case("return str(re.match(\"ab\", text))", 1 ; "str_argument")]
#[test_case("return not (re.match(\"ab\", text) or text)", 0 ; "or_under_not")]
#[test_case("return re.match(\"ab\", text) or text", 1 ; "or_value_used")]
#[test_case("return 1 if re.match(\"ab\", text) else 2", 0 ; "ternary_test")]
#[test_case("return re.match(\"ab\", text) if text else 2", 1 ; "ternary_body")]
#[test_case("return [t for t in text if re.match(\"ab\", t)]", 0 ; "list_condition")]
#[test_case("return {t: 1 for t in text if re.match(\"ab\", t)}", 0 ; "dict_condition")]
#[test_case("return {t for t in text if re.match(\"ab\", t)}", 0 ; "set_condition")]
#[test_case("return any(True for t in text if re.match(\"ab\", t))", 0 ; "generator_condition")]
#[test_case("return [re.match(\"ab\", t) for t in text]", 1 ; "comprehension_element")]
#[test_case("return re.match", 1 ; "reference")]
#[test_case("match text:\n        case str() if re.match(\"ab\", text):\n            pass", 0 ; "match_guard")]
#[test_case("return re.match(\"ab\", text) is None", 0 ; "compared_to_none")]
#[test_case("return re.match(\"ab\", text) == None", 1 ; "equal_to_none")]
#[test_case("return re.match(\"a.\", text) is None", 1 ; "metacharacter")]
#[test_case("return re.match(b\"ab\", text) is None", 0 ; "bytes_pattern")]
#[test_case("return re.match(b\"a.\", text) is None", 1 ; "bytes_metacharacter")]
#[test_case("return re.match(PLAIN, text) is None", 0 ; "name_bound_once")]
#[test_case("return re.match(REBOUND, text) is None", 1 ; "name_bound_twice")]
#[test_case("return re.match(\"ab\", text, 0) is None", 1 ; "flags_argument")]
#[test_case("return re.match(\"ab\", *text) is None", 1 ; "starred_string")]
#[test_case("if (found := re.match(\"ab\", text)):\n        pass", 1 ; "walrus")]
fn up_re_prefixmatch_leaves_truth_tests_to_ruf055(body: &str, reported: usize) {
    let source =
        format!("import re\n\nPLAIN = \"ab\"\nREBOUND = \"ab\"\nREBOUND = \"cd\"\n\n\ndef check(text):\n    {body}\n");
    let root = common::project(&[
        ("pyproject.toml", "[project]\nrequires-python = \">=3.15\"\n\n[tool.ruff.lint]\nselect = []\n"),
        ("module.py", &source),
    ]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP522", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP522\"").count(), reported, "{stdout}");
}

// SMP550 turns an end-anchored `re.match` into `re.fullmatch`, so with both rules selected SMP522 leaves it to SMP550
// and renames only the other calls.
#[test_case("return re.match(r\"a.$\", text)", 0, 1 ; "anchored")]
#[test_case("return re.match(r\"a.\", text)", 1, 0 ; "open_end")]
fn up_re_prefixmatch_leaves_anchored_calls_to_smp550(body: &str, renamed: usize, anchored: usize) {
    let source = format!("import re\n\n\ndef check(text):\n    {body}\n");
    let root = common::project(&[
        ("pyproject.toml", "[project]\nrequires-python = \">=3.15\"\n\n[tool.ruff.lint]\nselect = []\n"),
        ("module.py", &source),
    ]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP522,SMP550", "--output-format", "json"]);
    let counts = (stdout.matches("\"code\": \"SMP522\"").count(), stdout.matches("\"code\": \"SMP550\"").count());
    assert_eq!(counts, (renamed, anchored), "{stdout}");
}

// Code outside a library may call the public functions and classes of its public modules without the argument, even
// when every call in the run passes it. A private function there is still judged.
#[test]
fn unused_default_leaves_public_functions_of_library_modules_alone() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("core.py", "def connect(host, timeout=30):\n    return host, timeout\n\n\nconnect(\"a\", 5)\n"),
        ("_core.py", "def _connect(host, timeout=30):\n    return host, timeout\n\n\n_connect(\"a\", 5)\n"),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(
        stdout,
        "_core.py:1:28: SMP506 Every call to `_connect` passes `timeout`; drop its default `30`\nFound 1 error.\n1 \
         unsafe fix available with `--fix --unsafe-fixes`.\n"
    );
}

// A pattern another module imports may be read by number there, so its groups keep their numbers.
#[test]
fn unnamed_regex_group_keeps_a_pattern_another_module_imports() {
    let root = common::project(&[
        ("core.py", "import re\n\nPAIR = re.compile(r\"(a)(b)\")\n"),
        ("main.py", "from core import PAIR\n\n\ndef use(text):\n    return PAIR.match(text).group(2)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP532"]);
    assert_eq!(
        (code, stdout.as_str()),
        (
            1,
            "core.py:3:19: SMP532 regex has 2 unnamed capture groups; name them with `(?P<name>...)` so readers do not \
             count parentheses\nFound 1 error.\n"
        )
    );
}

// Constants come out in source order, each above the statement its call sits in, and a later pass adds below the
// ones an earlier pass added.
#[test]
fn inline_regex_fix_keeps_the_constants_in_source_order() {
    let root = common::project(&[(
        "module.py",
        "import re\n\n\ndef first(text):\n    return re.match(r\"a\\d\", text)\n\n\ndef second(text):\n    return \
         re.search(r\"b\\d\", text) and re.fullmatch(r\"c\\d\", text)\n",
    )]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP534", "--fix", "--unsafe-fixes"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (
            0,
            common::ONE_FIXED,
            "import re\nfrom typing import Final\n\n\
             _FIRST_PATTERN: Final[re.Pattern[str]] = re.compile(r\"a\\d\")\n\n\n\
             def first(text):\n    return _FIRST_PATTERN.match(text)\n\n\n_SECOND_PATTERN: Final[re.Pattern[str]] = \
             re.compile(r\"b\\d\")\n\n_SECOND_PATTERN_2: Final[re.Pattern[str]] = re.compile(r\"c\\d\")\n\n\n\
             def second(text):\n    return _SECOND_PATTERN.search(text) and _SECOND_PATTERN_2.fullmatch(text)\n"
                .to_owned()
        )
    );
}

// The module constant an inline-regex fix writes is one the constant rules leave alone.
#[test]
fn inline_regex_fix_leaves_the_constant_clean_of_the_constant_rules() {
    let root = common::project(&[(
        "module.py",
        "import re\n\n\ndef is_image(name):\n    return re.fullmatch(r\"[a-z]+\", name) is not None\n",
    )]);
    let (fix_code, fix_stdout, _) =
        check(root.path(), &["--no-cache", "--select", "SMP534", "--fix", "--unsafe-fixes"]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP202,SMP534,SMP601"]);
    assert_eq!((fix_code, fix_stdout.as_str(), code, stdout.as_str()), (0, common::ONE_FIXED, 0, common::PASSED));
}

// A module-level name another module imports, or `__all__` lists, keeps its report, and the fix that removes it is
// withheld.
#[test_case(
    "SMP512",
    "py314",
    "import contextvars\n\nrequest_id = contextvars.ContextVar(\"request_id\")\ntoken = request_id.set(\"one\")\ntry:\
        \n    print(request_id.get())\nfinally:\n    request_id.reset(token)\n",
    "from state import token\n",
    "state.py:4:1: SMP512 `token` resets the variable in a `finally`; a `with` block does that on exit\nFound 1 error.\
        \n" ;
    "context_var_imported"
)]
#[test_case(
    "SMP512",
    "py314",
    "__all__ = [\"token\"]\nimport contextvars\n\nrequest_id = contextvars.ContextVar(\"request_id\")\ntoken = \
        request_id.set(\"one\")\ntry:\n    print(request_id.get())\nfinally:\n    request_id.reset(token)\n",
    "",
    "state.py:5:1: SMP512 `token` resets the variable in a `finally`; a `with` block does that on exit\nFound 1 error.\
        \n" ;
    "context_var_listed_in_all"
)]
#[test_case(
    "SMP513",
    "py311",
    "import contextlib\nimport os\n\nold = os.getcwd()\nos.chdir(\"..\")\ntry:\n    print(os.getcwd())\nfinally:\n    \
        os.chdir(old)\n",
    "from state import old\n",
    "state.py:4:1: SMP513 Restoring the directory in a `finally` is what `contextlib.chdir` does\nFound 1 error.\n" ;
    "chdir_imported"
)]
#[test_case(
    "SMP512",
    "py314",
    "import contextvars\n\nrequest_id = contextvars.ContextVar(\"request_id\")\ntoken = request_id.set(\"one\")\ntry:\
        \n    print(request_id.get())\nfinally:\n    request_id.reset(token)\n",
    "from state import *\nprint(token)\n",
    "state.py:4:1: SMP512 `token` resets the variable in a `finally`; a `with` block does that on exit\nFound 1 error.\
        \n" ;
    "context_var_star_imported"
)]
#[test_case(
    "SMP513",
    "py311",
    "import contextlib\nimport os\n\nold = os.getcwd()\nos.chdir(\"..\")\ntry:\n    print(os.getcwd())\nfinally:\n    \
        os.chdir(old)\n",
    "from state import *\nprint(old)\n",
    "state.py:4:1: SMP513 Restoring the directory in a `finally` is what `contextlib.chdir` does\nFound 1 error.\n" ;
    "chdir_star_imported"
)]
fn module_name_another_module_uses_keeps_report_without_fix(
    select: &str,
    target: &str,
    state: &str,
    user: &str,
    report: &str,
) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff]\ntarget-version = \"{target}\"\n")),
        ("state.py", state),
        ("user.py", user),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", select]);
    assert_eq!((code, stdout.as_str()), (1, report));
    common::assert_fix_keeps_report(root.path(), select, &["--fix"], "state.py", report);
}

// A name in a function is not the module's, so another module importing the same name does not withhold its fix.
#[test]
fn context_var_token_local_named_like_an_imported_name_is_still_fixed() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py314\"\n"),
        (
            "state.py",
            "import contextvars\n\nrequest_id = contextvars.ContextVar(\"request_id\")\n\n\ndef handle(value):\n    \
                token = request_id.set(value)\n    try:\n        run()\n    finally:\n        request_id.reset(token)\n",
        ),
        ("user.py", "from state import token\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP512", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "state.py")),
        (
            0,
            common::ONE_FIXED,
            "import contextvars\n\nrequest_id = contextvars.ContextVar(\"request_id\")\n\n\ndef handle(value):\n    \
                with request_id.set(value):\n        run()\n"
                .to_owned()
        )
    );
}

const PY311: &str = "[tool.ruff]\ntarget-version = \"py311\"\n";

// A backslash joins the line before the assignment to it, so the `with` block would keep that line's statement before
// it.
#[test_case(
    "import os\n\nstarted = True; \\\nold = os.getcwd()\nos.chdir(\"..\")\ntry:\n    print(started)\nfinally:\n    os.\
        chdir(old)\n",
    "mod.py:4:1: SMP513 Restoring the directory in a `finally` is what `contextlib.chdir` does\nFound 1 error.\n" ;
    "backslash_joined_line"
)]
// A parameter named like the module hides it in the function, even when the module imports it too.
#[test_case(
    "import contextlib\nimport os\n\n\ndef run(target, contextlib=None):\n    old = os.getcwd()\n    os.chdir(target)\
        \n    try:\n        return os.listdir()\n    finally:\n        os.chdir(old)\n",
    "mod.py:6:5: SMP513 Restoring the directory in a `finally` is what `contextlib.chdir` does\nFound 1 error.\n" ;
    "parameter_takes_module_name"
)]
fn chdir_restore_keeps_report_without_fix(source: &str, report: &str) {
    let root = common::project(&[("pyproject.toml", PY311), ("mod.py", source)]);
    common::assert_fix_keeps_report(root.path(), "SMP513", &["--fix"], "mod.py", report);
}

// A comment ends no line with a backslash, so a backslash in one does not join the lines and the fix stays.
#[test_case(
    "mod.py",
    "import os\n\nstarted = True  # see \\\nold = os.getcwd()\nos.chdir(\"..\")\ntry:\n    print(os.getcwd(), started)\
        \nfinally:\n    os.chdir(old)\n",
    "import contextlib\nimport os\n\nstarted = True  # see \\\nwith contextlib.chdir(\"..\"):\n    print(os.getcwd(), \
        started)\n" ;
    "comment_ending_in_backslash"
)]
// The import goes above the comment that describes the import it precedes, so the comment stays with its statement.
#[test_case(
    "block.py",
    "import abc\n# os: the build runs in the target directory\nimport os\nimport sys\n\n\ndef run(target):\n    old = \
        os.getcwd()\n    os.chdir(target)\n    try:\n        return abc, sys, os.listdir()\n    finally:\n        os.\
        chdir(old)\n",
    "import abc\nimport contextlib\n# os: the build runs in the target directory\nimport os\nimport sys\n\n\ndef run(\
        target):\n    with contextlib.chdir(target):\n        return abc, sys, os.listdir()\n" ;
    "import_above_comment"
)]
// A docstring line that starts with `#` is text, not a comment, so the import goes above the comment after the
// docstring and not into the docstring.
#[test_case(
    "h.py",
    "\"\"\"Doc.\n\n# heading\"\"\"\n# comment\nVALUE = 1\n\n\ndef run(target):\n    import os\n\n    old = os.getcwd()\
        \n    os.chdir(target)\n    try:\n        return os.listdir()\n    finally:\n        os.chdir(old)\n",
    "\"\"\"Doc.\n\n# heading\"\"\"\nimport contextlib\n\n# comment\nVALUE = 1\n\n\ndef run(target):\n    import os\n\
        \n    with contextlib.chdir(target):\n        return os.listdir()\n" ;
    "docstring_line_starting_with_hash"
)]
// A comment on the first line is a header of the file, which the import stays below.
#[test_case(
    "first.py",
    "# Runs the step in the target directory; see docs/build.md.\ndef run(target):\n    import os\n\n    old = os.\
        getcwd()\n    os.chdir(target)\n    try:\n        return os.listdir()\n    finally:\n        os.chdir(old)\n",
    "# Runs the step in the target directory; see docs/build.md.\nimport contextlib\n\n\ndef run(target):\n    import \
        os\n\n    with contextlib.chdir(target):\n        return os.listdir()\n" ;
    "header_comment_on_first_line"
)]
// A class attribute named like the module is not seen by the methods of the class, so their fix stays.
#[test_case(
    "mod.py",
    "import os\n\n\nclass Runner:\n    contextlib = None\n\n    def run(self, target):\n        old = os.getcwd()\
        \n        os.chdir(target)\n        try:\n            return os.listdir()\n        finally:\n            os.\
        chdir(old)\n",
    "import contextlib\nimport os\n\n\nclass Runner:\n    contextlib = None\n\n    def run(self, target):\n        \
        with contextlib.chdir(target):\n            return os.listdir()\n" ;
    "class_attribute_takes_module_name"
)]
fn chdir_restore_fixes(file: &str, source: &str, expected: &str) {
    let root = common::project(&[("pyproject.toml", PY311), (file, source)]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP513", "--fix"]);
    assert_eq!((code, stdout.as_str(), read(root.path(), file)), (0, common::ONE_FIXED, expected.to_owned()));
}

// An import that shares its line with code cannot take the new import below it, as the code runs first, so the fix
// stays off. A comment after the import is no code, so the import takes the fix.
#[test_case(
    "SMP516",
    "py314",
    "import asyncio; print(asyncio.iscoroutinefunction(print))\n",
    "mod.py:1:23: SMP516 `asyncio.iscoroutinefunction()` is deprecated since Python 3.14 and also accepts the legacy \
        coroutine marker\nFound 1 error.\n" ;
    "asyncio_shared_line"
)]
#[test_case(
    "SMP601",
    "py314",
    "import os; LIMIT = 3\n",
    "mod.py:1:12: SMP601 `LIMIT` is never reassigned; annotate it `Final`\nFound 1 error.\n" ;
    "final_shared_line"
)]
fn import_after_a_shared_line_is_not_inserted(select: &str, target: &str, source: &str, report: &str) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff]\ntarget-version = \"{target}\"\n")),
        ("mod.py", source),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", select]);
    assert_eq!((code, stdout.as_str()), (1, report));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", select, "--fix", "--unsafe-fixes"]);
    assert_eq!((code, stdout.as_str(), read(root.path(), "mod.py")), (1, report, source.to_owned()));
}

#[test]
fn asyncio_import_after_a_comment_on_the_import_line_is_inserted_below_it() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff]\ntarget-version = \"py314\"\n"),
        (
            "mod.py",
            "import asyncio  # loop\n\n\ndef check(task):\n    asyncio.sleep(0)\n    return asyncio.iscoroutinefunction\
                (task)\n",
        ),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP516", "--fix", "--unsafe-fixes"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "mod.py")),
        (
            0,
            common::ONE_FIXED,
            "import asyncio  # loop\nimport inspect\n\n\ndef check(task):\n    asyncio.sleep(0)\n    return inspect.\
                iscoroutinefunction(task)\n"
                .to_owned()
        )
    );
}
