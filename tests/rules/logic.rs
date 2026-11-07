use tempfile::TempDir;
use test_case::test_case;

use crate::common::{self, check, read};

#[test_case("duplicate-code", "duplicate_code.py", "py314" ; "duplicate_code")]
#[test_case("duplicate-code", "duplicate_code_copy.py", "py314" ; "duplicate_code_copy")]
#[test_case("duplicate-code", "duplicate_code_many.py", "py314" ; "duplicate_code_many")]
#[test_case("duplicate-code", "duplicate_code_renamed.py", "py314" ; "duplicate_code_renamed")]
#[test_case("duplicate-code", "duplicate_code_twice.py", "py314" ; "duplicate_code_twice")]
#[test_case("duplicate-code", "duplicate_code_short.py", "py314" ; "duplicate_code_short")]
#[test_case("duplicate-code", "duplicate_code_nested.py", "py314" ; "duplicate_code_nested")]
#[test_case("duplicate-code", "duplicate_code_import.py", "py314" ; "duplicate_code_import")]
#[test_case("duplicate-code", "duplicate_code_nested_copy.py", "py314" ; "duplicate_code_nested_copy")]
#[test_case("duplicate-code", "duplicate_code_short_copy.py", "py314" ; "duplicate_code_short_copy")]
#[test_case("duplicate-code", "duplicate_code_import_copy.py", "py314" ; "duplicate_code_import_copy")]
#[test_case("duplicate-code", "duplicate_code_block.py", "py314" ; "duplicate_code_block")]
#[test_case("duplicate-code", "test_duplicate_code_module.py", "py314" ; "duplicate_code_test_module")]
#[test_case("forwarding-function", "forwarding_function.py", "py314" ; "forwarding_function")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py314" ; "walrus_candidate")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py37" ; "walrus_candidate_before_python38")]
#[test_case("walrus-candidate", "walrus_candidate_gate.py", "py314" ; "walrus_candidate_version_gate")]
#[test_case("walrus-candidate", "walrus_candidate_dynamic.py", "py314" ; "walrus_candidate_computed_all")]
#[test_case("walrus-candidate", "walrus_candidate_listed.py", "py314" ; "walrus_candidate_listed_in_all")]
#[test_case("walrus-candidate", "walrus_candidate_width.py", "py314" ; "walrus_candidate_header_width")]
#[test_case("single-caller-helper", "single_caller_helper.py", "py314" ; "single_caller_helper")]
#[test_case("single-caller-helper", "single_caller_helper_dynamic.py", "py314" ; "single_caller_helper_computed_all")]
#[test_case("single-caller-helper", "tests/test_single_caller_helper_from_test.py", "py314" ; "single_caller_helper_from_test")]
#[test_case("single-use-variable", "single_use_variable.py", "py314" ; "single_use_variable")]
#[test_case("single-use-variable", "single_use_variable_width.py", "py314" ; "single_use_variable_width")]
#[test_case("single-use-variable", "single_use_variable_debug.py", "py314" ; "single_use_variable_debug")]
#[test_case("single-use-variable", "single_use_variable_requote.py", "py311" ; "single_use_variable_requote")]
#[test_case("single-use-variable", "single_use_variable_splice.py", "py314" ; "single_use_variable_splice")]
#[test_case("single-use-variable", "single_use_variable_splice.py", "py311" ; "single_use_variable_splice_before_python312")]
#[test_case("single-use-variable", "single_use_variable_parens.py", "py314" ; "single_use_variable_parens")]
#[test_case("single-use-variable", "single_use_variable_field.py", "py314" ; "single_use_variable_field")]
#[test_case("single-use-variable", "single_use_variable_lifetime.py", "py314" ; "single_use_variable_lifetime")]
#[test_case("string-concatenation", "string_concatenation.py", "py314" ; "string_concatenation")]
#[test_case("string-concatenation", "string_concatenation_old.py", "py310" ; "string_concatenation_py310")]
#[test_case("string-concatenation", "string_concatenation_types.py", "py314" ; "string_concatenation_types")]
#[test_case("string-concatenation", "string_concatenation_kinds.py", "py314" ; "string_concatenation_kinds")]
#[test_case("string-concatenation", "string_concatenation_class.py", "py314" ; "string_concatenation_class")]
#[test_case("string-concatenation", "string_concatenation_genexp.py", "py314" ; "string_concatenation_genexp")]
#[test_case("string-concatenation", "string_concatenation_type_params.py", "py314" ; "string_concatenation_type_params")]
#[test_case("string-concatenation", "string_concatenation_ruff_overlap.py", "py314" ; "string_concatenation_ruff_overlap")]
#[test_case("string-concatenation", "string_concatenation_imported.py", "py314" ; "string_concatenation_imported")]
#[test_case("string-concatenation", "string_concatenation_imported_values.py", "py314" ; "string_concatenation_imported_values")]
#[test_case("string-concatenation", "string_concatenation_class_values.py", "py314" ; "string_concatenation_class_values")]
#[test_case("unused-default", "unused_default.py", "py314" ; "unused_default")]
#[test_case("unused-default", "unused_default_public_module.py", "py314" ; "unused_default_public_module")]
#[test_case("error-message-matching", "error_message_matching.py", "py314" ; "error_message_matching")]
#[test_case("up-comprehension-unpacking", "up_comprehension_unpacking.py", "py315" ; "comprehension_unpacking")]
#[test_case("up-comprehension-unpacking", "up_comprehension_unpacking.py", "py314" ; "comprehension_unpacking_before_python315")]
#[test_case("up-traceback-exception-arguments", "up_traceback_exception_arguments.py", "py310" ; "traceback_exception_arguments")]
#[test_case("up-traceback-exception-arguments", "up_traceback_exception_arguments.py", "py39" ; "traceback_exception_arguments_before_python310")]
#[test_case("up-exc-info-exception", "up_exc_info_exception.py", "py311" ; "exc_info_exception")]
#[test_case("up-exc-info-exception", "up_exc_info_exception.py", "py310" ; "exc_info_exception_before_python311")]
#[test_case("up-object-sentinel", "up_object_sentinel.py", "py315" ; "object_sentinel")]
#[test_case("up-object-sentinel", "up_object_sentinel.py", "py314" ; "object_sentinel_before_python315")]
#[test_case("up-context-var-token-reset", "up_context_var_token_reset.py", "py314" ; "context_var_token_reset")]
#[test_case("up-context-var-token-reset", "up_context_var_token_reset.py", "py313" ; "context_var_token_reset_before_python314")]
#[test_case("up-chdir-restore", "up_chdir_restore.py", "py311" ; "chdir_restore")]
#[test_case("up-chdir-restore", "up_chdir_restore_future.py", "py311" ; "chdir_restore_after_future_import")]
#[test_case("up-chdir-restore", "up_chdir_restore_no_import.py", "py311" ; "chdir_restore_without_imports")]
#[test_case("up-chdir-restore", "up_chdir_restore_imported.py", "py311" ; "chdir_restore_contextlib_imported")]
#[test_case("up-chdir-restore", "up_chdir_restore_name.py", "py311" ; "chdir_restore_contextlib_name_taken")]
#[test_case("up-chdir-restore", "up_chdir_restore.py", "py310" ; "chdir_restore_before_python311")]
#[test_case("up-strptime-date", "up_strptime_date.py", "py314" ; "strptime_date")]
#[test_case("up-strptime-date", "up_strptime_date_name.py", "py314" ; "strptime_date_class_name")]
#[test_case("up-strptime-date", "up_strptime_date_spelled.py", "py314" ; "strptime_date_name_taken")]
#[test_case("up-strptime-date", "up_strptime_date_class_import.py", "py314" ; "strptime_date_class_imported_by_function")]
#[test_case("up-strptime-date", "up_strptime_date_imported.py", "py314" ; "strptime_date_name_imported")]
#[test_case("up-strptime-date", "up_strptime_date_no_class_import.py", "py314" ; "strptime_date_without_class_import")]
#[test_case("up-strptime-date", "up_strptime_date.py", "py313" ; "strptime_date_before_python314")]
#[test_case("up-event-loop-in-coroutine", "up_event_loop_in_coroutine.py", "py314" ; "event_loop_in_coroutine")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction.py", "py314" ; "asyncio_iscoroutinefunction")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction_inspect.py", "py314" ; "asyncio_iscoroutinefunction_imported")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction_bound.py", "py314" ; "asyncio_iscoroutinefunction_bound")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction_sorted.py", "py314" ; "asyncio_iscoroutinefunction_sorted")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction_guarded.py", "py314" ; "asyncio_iscoroutinefunction_guarded")]
#[test_case("up-asyncio-iscoroutinefunction", "up_asyncio_iscoroutinefunction_name.py", "py314" ; "asyncio_iscoroutinefunction_name")]
#[test_case("up-codecs-open", "up_codecs_open.py", "py314" ; "codecs_open")]
#[test_case("up-codecs-open", "up_codecs_open_shadowed.py", "py314" ; "codecs_open_renamed")]
#[test_case("up-codecs-open", "up_codecs_open_builtin.py", "py314" ; "codecs_open_builtin")]
#[test_case("up-re-error-alias", "up_re_error_alias.py", "py314" ; "re_error_alias")]
#[test_case("up-re-error-alias", "up_re_error_alias.py", "py312" ; "re_error_alias_before_python313")]
#[test_case("up-argparse-suggest-on-error", "up_argparse_suggest_on_error.py", "py315" ; "argparse_suggest_on_error")]
#[test_case("up-argparse-suggest-on-error", "up_argparse_suggest_on_error.py", "py310" ; "argparse_suggest_on_error_before_python315")]
#[test_case("up-argparse-suggest-on-error", "up_argparse_suggest_on_error_commented.py", "py315" ; "argparse_suggest_on_error_commented")]
#[test_case("up-removed-ast-node", "up_removed_ast_node.py", "py314" ; "removed_ast_node")]
#[test_case("up-removed-ast-node", "up_removed_ast_node_shadowed.py", "py314" ; "removed_ast_node_shadowed")]
#[test_case("early-assignment", "early_assignment.py", "py314" ; "early_assignment")]
#[test_case("early-assignment", "early_assignment_blocked.py", "py314" ; "early_assignment_blocked")]
#[test_case("super-forwarding-method", "super_forwarding_method.py", "py314" ; "super_forwarding_method")]
#[test_case("super-forwarding-method", "super_forwarding_method_decorated.py", "py314" ; "super_forwarding_method_decorated")]
#[test_case("long-tuple-return", "long_tuple_return.py", "py314" ; "long_tuple_return")]
#[test_case("cognitive-complexity", "cognitive_complexity.py", "py314" ; "cognitive_complexity")]
#[test_case("collection-built-by-statements", "collection_built_by_statements.py", "py314" ; "collection_built_by_statements")]
#[test_case("collection-built-by-statements", "collection_built_by_statements_names.py", "py314" ; "collection_built_by_statements_names")]
#[test_case("unnamed-regex-group", "unnamed_regex_group.py", "py314" ; "unnamed_regex_group")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_unknown_flags.py", "py314" ; "unnamed_regex_group_unknown_flags")]
#[test_case("unnamed-regex-group", "unnamed_regex_group_default_scope.py", "py314" ; "unnamed_regex_group_default_scope")]
#[test_case("unexplained-regex", "unexplained_regex.py", "py314" ; "unexplained_regex")]
#[test_case("unexplained-regex", "unexplained_regex_unknown_flags.py", "py314" ; "unexplained_regex_unknown_flags")]
#[test_case("up-utf8-encoding-default", "up_utf8_encoding_default.py", "py315" ; "utf8_encoding_default")]
#[test_case("up-utf8-encoding-default", "up_utf8_encoding_default.py", "py314" ; "utf8_encoding_default_before_python315")]
#[test_case("up-utf8-encoding-default", "up_utf8_encoding_default_shadowed.py", "py315" ; "utf8_encoding_default_shadowed_open")]
#[test_case("up-re-prefixmatch", "up_re_prefixmatch.py", "py315" ; "re_prefixmatch")]
#[test_case("up-re-prefixmatch", "up_re_prefixmatch.py", "py314" ; "re_prefixmatch_before_python315")]
#[test_case("up-compression-modules", "up_compression_modules.py", "py314" ; "compression_modules")]
#[test_case("up-compression-modules", "up_compression_modules.py", "py313" ; "compression_modules_before_python314")]
#[test_case("up-compression-modules", "compression/up_compression_modules_shim.py", "py314" ; "compression_modules_in_compression_package")]
#[test_case("up-warnings-deprecated", "up_warnings_deprecated.py", "py313" ; "warnings_deprecated")]
#[test_case("up-warnings-deprecated", "up_warnings_deprecated.py", "py312" ; "warnings_deprecated_before_python313")]
#[test_case("up-deprecated-stdlib-alias", "up_deprecated_stdlib_alias.py", "py312" ; "deprecated_stdlib_alias")]
#[test_case("up-deprecated-stdlib-alias", "up_deprecated_stdlib_alias.py", "py310" ; "deprecated_stdlib_alias_python310")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// Ruff's cleanup merges the `if` the fix leaves alone in an outer `if`, whatever the layout and line length.
#[test_case("if flag:", "        size = len(text)", "if flag and len(text) > 3:" ; "fits")]
#[test_case("if flag:", "        size = len(text                )", "if flag and len(text                ) > 3:" ; "spaced")]
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
        (
            0,
            "Found 1 error (1 fixed, 0 remaining).\n",
            format!("def run(text, flag):\n    {merged}\n        return text\n")
        )
    );
}

#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n        else:\n            return \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n        else:\n            return \"\"\n"
    ; "inner_else"
)]
#[test_case(
    "    if flag:\n        size = len(text)\n        if size > 3:\n            return text\n    else:\n        return \"\"\n",
    "    if flag:\n        if len(text) > 3:\n            return text\n    else:\n        return \"\"\n"
    ; "outer_else"
)]
#[test_case(
    "    if flag:  # keep\n        size = len(text)\n        if size > 3:\n            return text\n        else:\n            return \"\"\n",
    "    if flag:  # keep\n        if len(text) > 3:\n            return text\n        else:\n            return \"\"\n"
    ; "inner_else_under_comment"
)]
fn walrus_candidate_keeping_if_branches_is_fixed(body: &str, fixed: &str) {
    let root = collapsible_if_project(&format!("def run(text, flag):\n{body}"));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (0, "Found 1 error (1 fixed, 0 remaining).\n", format!("def run(text, flag):\n{fixed}"))
    );
}

// Ruff does not merge an `if` into an outer `if` whose header holds a comment, so the fix would leave work the fix
// loop withdraws. The finding stays, without its fix, and the other candidates of the file are still fixed.
#[test]
fn walrus_candidate_under_a_comment_in_the_outer_if_is_reported_without_a_fix() {
    let source = "def run(text, flag):\n    if flag:  # keep\n        size = len(text)\n        if size > 3:\n            \
                  return text\n    return \"\"\n";
    let root = collapsible_if_project(source);
    let report = "module.py:3:9: SMP502 Inline single-use `size` into the `if` test\nFound 1 error.\n";
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502"]);
    assert_eq!((code, stdout.as_str()), (1, report));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix", "--unsafe-fixes"]);
    assert_eq!((code, stdout.as_str(), read(root.path(), "module.py")), (1, report, source.to_owned()));
}

#[test]
fn walrus_candidate_under_a_comment_leaves_the_other_candidates_fixed() {
    let source = "def run(text, flag):\n    if flag:  # keep\n        size = len(text)\n        if size > 3:\n            \
                  return text\n    return \"\"\n\n\ndef first(text):\n    found = text.strip()\n    if found:\n        \
                  return found\n    return text\n";
    let root = collapsible_if_project(source);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (
            1,
            "module.py:3:9: SMP502 Inline single-use `size` into the `if` test\nFound 2 errors (1 fixed, 1 remaining).\n",
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
             -> list[str] | None:\n    if flag:\n        found = re.findall(_PATTERN, text)\n        if found:\n            \
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
        "def run(text, flag):\n    size = len(text)\n    if size > 3 and flag:\n        if text.isupper():\n            \
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
        "def run(registry):\n    decorate = {value}\n    {decorator}\n    def inner():\n        return 1\n    return inner\n"
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
    "def join(items):\n    joined = (\n        b', '.join(items)\n    )\n    return f\"Items({joined})\"\n\n\ndef size(items):\n    count = len(items)\n    return f\"{count}\"\n",
    Some("def join(items):\n    return f\"Items({b', '.join(items)})\"\n\n\ndef size(items):\n    return f\"{len(items)}\"\n")
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
    "def render(items):\n    prepr = f\"[{','.join(items)}]\"\n    return f\"Items({prepr})\"\n\n\ndef size(items):\n    count = len(items)\n    return f\"{count}\"\n",
    Some("def render(items):\n    return f\"Items([{','.join(items)}])\"\n\n\ndef size(items):\n    return f\"{len(items)}\"\n")
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

// `yield name` is never reported. Ruff's preview rule for a `yield from` of the assigned name covers the same pair when
// its fix is the same, so that pair is left to it with preview on, and reported otherwise. The other next statements are
// inlined in both modes.
#[test_case(false, "def run(items):\n    count = len(items)\n    yield count\n", "def run(items):\n    count = len(items)\n    yield count\n" ; "yield_never_reported")]
#[test_case(true, "def run(items):\n    count = len(items)\n    yield count\n", "def run(items):\n    count = len(items)\n    yield count\n" ; "preview_on_yield_never_reported")]
#[test_case(false, "def run(items):\n    count = len(items)\n    yield from count\n", "def run(items):\n    yield from len(items)\n" ; "preview_off_inlines_yield_from")]
#[test_case(true, "def run(items):\n    count = len(items)\n    yield from count\n", "def run(items):\n    count = len(items)\n    yield from count\n" ; "preview_on_leaves_yield_from_to_ruff")]
#[test_case(true, "def run(first, second):\n    pair = first, second\n    yield from pair\n", "def run(first, second):\n    yield from (first, second)\n" ; "preview_on_inlines_tuple_yield_from")]
#[test_case(true, "def run(first, second):\n    pair = first, second\n    yield pair\n", "def run(first, second):\n    pair = first, second\n    yield pair\n" ; "preview_on_tuple_yield_never_reported")]
#[test_case(true, "def run(items):\n    count = len(items)\n    print(count)\n", "def run(items):\n    print(len(items))\n" ; "preview_on_inlines_into_call")]
#[test_case(true, "def run(items):\n    count = len(items)\n    total = count\n    return total\n", "def run(items):\n    total = len(items)\n    return total\n" ; "preview_on_inlines_into_assignment")]
fn single_use_follows_ruff_preview_for_yields(preview: bool, source: &str, fixed: &str) {
    let ruff = format!(
        "[tool.ruff]\ntarget-version = \"py314\"\n\n[tool.ruff.lint]\npreview = {preview}\nselect = [\"RET504\", \"RUF070\"]\n"
    );
    let root = common::project(&[("pyproject.toml", ruff.as_str()), ("module.py", source)]);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), fixed);
}

// `ruff format` closes the gap a removed statement leaves, but a `# fmt: off` file keeps it, so the fix takes one of the
// two blank lines with it.
#[test]
fn single_use_variable_keeps_one_blank_line_under_fmt_off() {
    let source = "# fmt: off\ndef make(flags, kwargs):\n    configure(kwargs)\n\n    cli_kwargs = to_args(kwargs)\n\n    return flags + cli_kwargs\n";
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
    let source = "def f(conn):\n    request = (\n        b\"POST / HTTP/1.1\\r\\n\"\n        b\"Host: example.com\\r\\n\"\n    )\n    conn.receive_data(request)\n";
    let root = single_use_project("py314", source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix", "--unsafe-fixes"]);
    assert_eq!(
        read(root.path(), "module.py"),
        "def f(conn):\n    conn.receive_data(b\"POST / HTTP/1.1\\r\\n\"\n        b\"Host: example.com\\r\\n\")\n"
    );
}

// A name a string annotation or a `ForwardRef` reads is read there too, so inlining its value would leave the string
// naming an undefined name.
#[test_case("class Box(Generic[T]):\n        item: \"list[T]\"" ; "string_annotation")]
#[test_case("box = Generic[T]\n    ref = ForwardRef(\"list[T]\")\n    return box, ref" ; "forward_ref")]
fn single_use_string_annotation_is_a_read(body: &str) {
    let source = format!(
        "from typing import ForwardRef, Generic, TypeVar\n\n\ndef build() -> type:\n    T = TypeVar(\"T\")\n\n    {body}\n"
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
        "from typing import Generic, TypeVar\n\n\ndef build() -> type:\n    T = TypeVar(\"T\")\n\n    class Box(Generic[T]):\n        item: \"list[\"\n\n    return Box\n",
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

// A library's public functions may be called by outside code that omits the argument.
#[test]
fn unused_default_leaves_the_public_api_alone() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("core.py", "__all__ = [\"connect\"]\n\n\ndef connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import connect\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.as_str()), (0, "All checks passed!\n"));
}

// Outside code calls the public methods of a public class, not its private ones, so a private method keeps its judgment.
#[test]
fn unused_default_judges_private_methods_of_public_classes() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "core.py",
            "__all__ = [\"Client\"]\n\n\nclass Client:\n    def run(self) -> None:\n        self._verify(\"a\", 1)\n\n    def _verify(self, host, timeout=30):\n        return host, timeout\n",
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
            "def _connect(host, timeout=30):\n    return host, timeout\n\n\ndef _helper(host, timeout=30):\n    return host, timeout\n\n\n_connect(\"a\", 5)\n_helper(\"a\", 5)\n",
        ),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(stdout.matches("Every call to").count(), 1, "{stdout}");
    assert!(stdout.contains("`_helper`") && !stdout.contains("`_connect`"), "{stdout}");
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
            "def make(x: int = 0) -> int:\n    return x\n\n\ndef _unused(x: int = 1) -> int:\n    return x\n\n\n_unused(2)\n",
        ),
        ("pkg/cli.py", "from pkg import make\n\nprint(make(2))\n"),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(stdout.matches("Every call to").count(), 1, "{stdout}");
    assert!(stdout.contains("`_unused`") && !stdout.contains("`make`"), "{stdout}");
}

// The `__all__` of a library's public module declares its interface, so a public function it leaves out is judged like a
// private one, as the other API rules judge it.
#[test]
fn unused_default_judges_public_functions_all_omits() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "core.py",
            "__all__ = [\"connect\"]\n\n\ndef connect(host, timeout=30):\n    return host, timeout\n\n\ndef helper(host, timeout=30):\n    return host, timeout\n\n\nconnect(\"a\", 1)\nhelper(\"a\", 1)\n",
        ),
    ]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!(stdout.matches("Every call to").count(), 1, "{stdout}");
    assert!(stdout.contains("`helper`") && !stdout.contains("`connect`"), "{stdout}");
}

// A star import hands the module's names to code that the calls of the run do not show.
#[test]
fn unused_default_leaves_star_imported_modules_alone() {
    let root = common::project(&[
        ("core.py", "def connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import *\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.as_str()), (0, "All checks passed!\n"));
}

// Test modules may assert on the messages their code raises.
#[test]
fn error_message_matching_leaves_test_modules_alone() {
    let root = common::project(&[(
        "test_errors.py",
        "def test_load():\n    try:\n        load()\n    except OSError as exc:\n        assert \"gone\" in str(exc)\n",
    )]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!((code, stdout.as_str()), (0, "All checks passed!\n"));
}

// Ruff's PT017 reports an `assert` naming the exception, which SMP507 leaves to it while the project's Ruff
// configuration selects it.
#[test_case("assert \"gone\" in str(exc)", "PT017", 0 ; "assert_left_to_ruff")]
#[test_case("assert \"gone\" in str(exc)", "E501", 1 ; "assert_reported_without_pt017")]
#[test_case("if \"gone\" in str(exc):\n            pass", "PT017", 1 ; "if_reported_with_pt017")]
fn error_message_matching_leaves_asserts_to_pt017(body: &str, code: &str, reports: usize) {
    let source = format!("def run(f):\n    try:\n        f()\n    except OSError as exc:\n        {body}\n");
    let settings = format!("[tool.ruff.lint]\nselect = [\"{code}\"]\n");
    let root = common::project(&[("pyproject.toml", &settings), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// The text of two exceptions tells them apart by their wording alone, so only the text of one against a literal or a
// plain variable is reported.
#[test_case("return exc.args == other.args", 0 ; "args_of_two_exceptions")]
#[test_case("return str(exc) == str(other)", 0 ; "text_of_two_exceptions")]
#[test_case("return str(exc) == \"boom\"", 1 ; "text_against_literal")]
#[test_case("return exc.args == expected", 1 ; "args_against_variable")]
#[test_case("try:\n            f()\n        except OSError as inner:\n            return str(exc) == str(inner)", 0 ; "text_of_nested_handlers")]
#[test_case("return str(exc) == str(other) == \"boom\"", 1 ; "text_chained_with_literal")]
#[test_case("return \"boom\" == str(exc) == str(other)", 1 ; "literal_chained_before_text")]
#[test_case("other = \"timed out\"\n        return str(exc) == str(other)", 1 ; "parameter_rebound_to_text")]
#[test_case("def inner(other):\n            return str(exc) == str(other)\n        return inner(expected)", 1 ; "shadowed_by_unannotated_parameter")]
#[test_case("return (lambda other: str(exc) == str(other))(expected)", 1 ; "shadowed_by_lambda_parameter")]
#[test_case("return (lambda: str(exc) == str(other))()", 0 ; "lambda_reads_outer_parameter")]
fn error_message_matching_leaves_text_pairs_alone(body: &str, reports: usize) {
    let source = format!(
        "def run(f, other: OSError, expected):\n    try:\n        f()\n    except OSError as exc:\n        {body}\n"
    );
    let root = common::project(&[("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// The parameters of a method, reached through its class, are read the same way as those of a function.
#[test_case("return replay_error.args == error.args", 0 ; "args_of_a_parameter")]
#[test_case("return replay_error.args == expected", 1 ; "args_against_variable")]
fn error_message_matching_reads_parameters_of_methods(body: &str, reports: usize) {
    let source = format!(
        "class Replay:\n    def run(self, target, error: OSError, expected):\n        try:\n            target()\n        except OSError as replay_error:\n            {body}\n"
    );
    let root = common::project(&[("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
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
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// A string annotation names the class the file binds under that name, which shadows the builtin of the same name.
#[test_case("\"ValueError\"", 1 ; "string_names_shadowed_builtin_without_exception")]
#[test_case("\"OSError\"", 0 ; "string_names_builtin_exception")]
#[test_case("\"int\"", 1 ; "string_names_builtin_without_exception")]
fn error_message_matching_reads_string_annotations_in_the_file(annotation: &str, reports: usize) {
    let source = format!(
        "class ValueError:\n    pass\n\n\ndef run(f, other: {annotation}):\n    try:\n        f()\n    except OSError as exc:\n        return str(exc) == str(other)\n"
    );
    let root = common::project(&[("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
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
        "from typing import Optional\n\n\nclass Local(Exception):\n    pass\n\n\nclass Deep(Local):\n    pass\n\n\ndef run(f, other: {annotation}):\n    try:\n        f()\n    except OSError as exc:\n        return str(exc) == str(other)\n"
    );
    let root = common::project(&[("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// An exception class the parameter's annotation imports from another file of the run.
#[test_case("class MyError(Exception):\n    pass\n", 0 ; "imported_exception_class")]
#[test_case("class MyError:\n    pass\n", 1 ; "imported_class_is_not_an_exception")]
fn error_message_matching_reads_imported_exceptions(definition: &str, reports: usize) {
    let module = "from errors import MyError\n\n\ndef run(f, other: MyError):\n    try:\n        f()\n    except OSError as exc:\n        return str(exc) == str(other)\n";
    let root = common::project(&[("errors.py", definition), ("module.py", module)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// A class nested in a function is an exception class of the file, whether the annotation names it or a string does.
#[test_case("Local", 0 ; "nested_class_name")]
#[test_case("\"Local\"", 0 ; "nested_class_string")]
#[test_case("int", 1 ; "nested_control_without_exception")]
fn error_message_matching_reads_nested_exception_classes(annotation: &str, reports: usize) {
    let source = format!(
        "def outer():\n    class Local(Exception):\n        pass\n\n    def inner(f, other: {annotation}):\n        try:\n            f()\n        except OSError as exc:\n            return str(exc) == str(other)\n    return inner\n"
    );
    let root = common::project(&[("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// A class whose base name is rebound to a class deriving from it is followed through both bindings, and stops at the
// cycle, so the first binding still shows the exception.
#[test]
fn error_message_matching_stops_at_rebound_class_cycles() {
    let source = "def outer():\n    class B(Exception):\n        pass\n\n    class A(B):\n        pass\n\n    class B(A):\n        pass\n\n    def inner(f, other: A):\n        try:\n            f()\n        except OSError as exc:\n            return str(exc) == str(other)\n    return inner\n";
    let root = common::project(&[("module.py", source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.as_str(), "All checks passed!\n");
}

// An exception class of the file counts as an exception type for a parameter annotation.
#[test]
fn error_message_matching_reads_exception_classes_of_the_file() {
    let root = common::project(&[(
        "module.py",
        "class Offset(Exception):\n    pass\n\n\ndef replay(target, error: Offset):\n    try:\n        target()\n    except Offset as replay_error:\n        return replay_error.args == error.args\n",
    )]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!((code, stdout.as_str()), (0, "All checks passed!\n"));
}

// A class the run defines that a star import brings into the module, which the annotation then names.
#[test_case("class AppError(Exception):\n    pass\n", 0 ; "star_imported_exception_class")]
#[test_case("class AppError:\n    pass\n", 1 ; "star_imported_class_is_not_an_exception")]
fn error_message_matching_reads_star_imported_exceptions(definition: &str, reports: usize) {
    let module = "from mod_a import *\n\n\ndef run(f, other: AppError):\n    try:\n        f()\n    except OSError as exc:\n        return str(exc) == str(other)\n";
    let root = common::project(&[("mod_a.py", definition), ("module.py", module)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP507"]);
    assert_eq!(stdout.matches("SMP507").count(), reports, "{stdout}");
}

// The tuple length and the complexity a project allows before the rules report, read from `[tool.simplint.logic]`.
#[test_case("", 2 ; "default_limit_reports_triples")]
#[test_case("[tool.simplint.logic]\nmax-tuple-elements = 3\n", 1 ; "limit_three_reports_quadruples")]
#[test_case("[tool.simplint.logic]\nmax-tuple-elements = 4\n", 0 ; "limit_four_reports_nothing")]
fn long_tuple_return_limit(settings: &str, reports: usize) {
    let source = "def triple() -> tuple[int, int, int]:\n    return 1, 2, 3\n\n\ndef quad() -> tuple[int, int, int, int]:\n    return 1, 2, 3, 4\n";
    let root = common::project(&[("pyproject.toml", settings), ("module.py", source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP528"]);
    assert_eq!(stdout.matches("SMP528").count(), reports);
}

// A function scoring 18 is reported above a limit of 17 and left alone at 18, since the limit is the highest score kept.
#[test_case("", 1 ; "default_limit_of_twenty_reports_twenty_one")]
#[test_case("[tool.simplint.logic]\nmax-cognitive-complexity = 17\n", 2 ; "limit_seventeen_reports_eighteen")]
#[test_case("[tool.simplint.logic]\nmax-cognitive-complexity = 18\n", 1 ; "limit_eighteen_keeps_eighteen")]
fn cognitive_complexity_limit(settings: &str, reports: usize) {
    let source = fixture_source("cognitive_complexity/cognitive_complexity.py");
    let root = common::project(&[("pyproject.toml", settings), ("module.py", &source)]);
    let (_, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP529"]);
    assert_eq!(stdout.matches("SMP529").count(), reports);
}

// Each function in the fixture ends its name with the score SonarSource gives it, which a limit of zero reports.
#[test]
fn cognitive_complexity_scores() {
    let source = fixture_source("cognitive_complexity/cognitive_complexity_scores.py");
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

fn fixture_source(path: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/logic").join(path))
        .expect("fixture")
}

// Ruff's RET504 reports `return name` after an assignment, and RUF070 reports a `yield from` of one, which SMP504 leaves
// to them while the project's Ruff configuration runs them.
#[test_case("def f() -> int:\n    total = 1 + 2\n    return total\n", "RET504", false, "`total` is used once" ; "return")]
#[test_case("def f():\n    values = make()\n    yield from values\n", "RUF070", true, "`values` is used once" ; "yield_from")]
fn single_use_variable_follows_ruff_returns_and_yield_froms(source: &str, code: &str, preview: bool, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let preview = if preview { "[tool.ruff]\npreview = true\n" } else { "" };
        let root = common::project(&[
            ("pyproject.toml", &format!("{preview}[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP504"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
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
#[test_case("B008", "[tool.ruff.lint.flake8-bugbear]\nextend-immutable-calls = [\"frozen.freeze\"]\n", "freeze()", "held", true, true ; "extended_immutable_call")]
#[test_case("B008", "", "freeze()", "held", true, false ; "call_not_extended")]
#[test_case("B006", "", "[]", "held", true, false ; "list_literal")]
#[test_case("B006", "", "[]", "held", false, true ; "list_literal_without_b006")]
#[test_case("B006", "", "list()", "held", true, false ; "list_call")]
#[test_case("B006", "", "list()", "held", false, true ; "list_call_without_b006")]
#[test_case("B006", "", "collections.Counter()", "held", true, false ; "counter_call")]
#[test_case("B006", "", "collections.Counter()", "held", false, true ; "counter_call_without_b006")]
#[test_case("B006", "", "{key: 1 for key in range(3)}", "held", true, false ; "dict_comprehension")]
#[test_case("B006", "", "{key: 1 for key in range(3)}", "held", false, true ; "dict_comprehension_without_b006")]
#[test_case("B006", "", "([],)", "held", true, true ; "tuple_of_list_without_preview")]
#[test_case("B006", "[tool.ruff]\npreview = true\n", "([],)", "held", true, false ; "tuple_of_list_in_preview")]
#[test_case("B006", "", "(key for key in range(3))", "held", true, true ; "generator_without_preview")]
#[test_case("B006", "[tool.ruff]\npreview = true\n", "(key for key in range(3))", "held", true, false ; "generator_in_preview")]
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
        "import collections\nfrom frozen import freeze\nfrom typing import NewType\n\nUserId = NewType(\"UserId\", int)\n\n\ndef make() -> list[int]:\n    return []\n\n\ndef f() -> None:\n    lock = {value}\n\n    def g({parameter}=lock) -> None:\n        print(held)\n"
    );
    let select = if selected { format!("\"{code}\"") } else { String::new() };
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n{extra}")),
        ("module.py", source.as_str()),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP504"]).1;
    assert_eq!(stdout.contains("`lock` is used once"), reported, "{stdout}");
}

// Ruff's G003 reports a `+` that is a logging call's message, and its ISC003 a concatenation of two literals across
// lines, so SMP503 leaves those to Ruff while the project's Ruff configuration selects them.
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"Hello, \" + name)\n", false ; "logging_message")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(msg=\"Hello, \" + name)\n", false ; "logging_keyword_message")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.log(1, \"Hello, \" + name)\n", false ; "logging_log_message")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"Hello, \" + name + \"!\")\n", false ; "logging_chain_message")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.log(\"Hello, \" + name)\n", true ; "logging_log_without_message")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.info(\"a\", \"Hello, \" + name)\n", true ; "logging_second_argument")]
#[test_case("G003", "", "import logging\n\n\ndef f(name: str) -> None:\n    logging.info((\"Hello, \" + name) * 2)\n", true ; "logging_repeated_message")]
#[test_case("G003", "", "def f(name: str) -> None:\n    foo.info(\"Hello, \" + name)\n", true ; "not_a_logger")]
#[test_case("G003", "", "def f(name: str) -> None:\n    logger.info(\"Hello, \" + name)\n", false ; "logger_named_variable")]
#[test_case("G003", "logger-objects = [\"mypkg.logger\"]\n", "from mypkg import logger\n\n\ndef f(name: str) -> None:\n    logger.info(\"Hello, \" + name)\n", false ; "logger_object")]
#[test_case("ISC003", "", "x = (\n    \"Hello, \"\n    + \"world\"\n)\n", false ; "literal_pair_across_lines")]
#[test_case("ISC003", "", "x = \"Hello, \" + \"world\"\n", true ; "literal_pair_on_one_line")]
#[test_case("ISC003", "", "x = (\n    \"Hello, \"\n    + \"world\"\n    + name\n)\n", false ; "pair_at_start_of_chain")]
#[test_case("ISC003", "", "x = (\n    \"Hello, \"\n    + name\n)\n", true ; "name_in_pair")]
#[test_case("ISC003", "[tool.ruff.lint.flake8-implicit-str-concat]\nallow-multiline = false\n", "x = (\n    \"Hello, \"\n    + \"world\"\n)\n", true ; "multiline_not_allowed")]
fn string_concatenation_follows_ruff(code: &str, extra: &str, source: &str, reported_when_selected: bool) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n{extra}")),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP503"]).1;
        assert_eq!(stdout.contains("Build text with an f-string"), !selected || reported_when_selected, "{stdout}");
    }
}

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
    let shapes = "class Badge(str):\n    pass\n\n\nclass badge(str):\n    pass\n\n\ndef Label(text: str) -> str:\n    return text\n";
    let source = format!(
        "import re\nimport shapes\nimport tags\n\nfrom markupsafe import Markup\nfrom shapes import Badge, Label, badge\nfrom tags import _Hidden, minify\n\n\ndef f() -> list[str]:\n    return [{call} + \"!\"]\n"
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
    let source = "import markupsafe\n\nfrom shapes import Badge\n\n\ndef f(safe: Badge, outside: markupsafe.Markup) -> list[str]:\n    return [safe + \"!\", outside + \"!\"]\n";
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

// Before Python 3.12 a replacement field cannot hold a backslash inside a string, so a chain whose expression has one has
// no f-string for that target and is not reported. A line continuation outside any string is not such a backslash, and
// a quote conflict only declines the fix, since a triple-quoted f-string holds both quote characters.
#[test_case("py311", "def render(tail: str, names: list[str]) -> str:\n    return \"<p>\" + tail + \"</p>\" + '\", \"'.join(names) + \"'\"\n", 1 ; "both_quotes_before_python312")]
#[test_case("py312", "def render(tail: str, names: list[str]) -> str:\n    return \"<p>\" + tail + \"</p>\" + '\", \"'.join(names) + \"'\"\n", 1 ; "both_quotes_from_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return \"a\" + name.strip(\"\\n\")\n", 0 ; "backslash_before_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return \"it's \\\"x\\\"\" + name\n", 1 ; "string_operand_with_both_quotes_before_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return 'say \"hi\"' + \"it's\" + name\n", 1 ; "clashing_string_operands_before_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return \"a\" + name \\\n    .strip()\n", 1 ; "line_continuation_before_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return \"x\" + name.join('''a\n    b''')\n", 1 ; "multiline_expression_rendered_escape_before_python312")]
#[test_case("py311", "def f(name: str) -> str:\n    return \"\"\"a\"\"\" + name.strip('\"') + \"!\"\n", 1 ; "triple_quoted_operand_before_python312")]
fn string_concatenation_before_python312(target: &str, source: &str, reported: usize) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--select", "SMP503", "--target-version", target]).1;
    assert_eq!(stdout.matches("Build text with an f-string").count(), reported, "{stdout}");
}

// FURB113 reports two or more appends to a list, with appends to other lists between them, when preview mode is on and
// the project selects it. The run leaves a list that Ruff reports to Ruff, and keeps the rest.
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "xs = []\nxs.append(1)\nxs.append(2)\n", 0 ; "preview_selected")]
#[test_case("select = [\"FURB113\"]\n", "xs = []\nxs.append(1)\nxs.append(2)\n", 1 ; "preview_off")]
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "xs = []\nxs.append(1)\n", 1 ; "single_append_kept")]
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "ys = []\nxs = []\nxs.append(1)\nys.append(2)\nxs.append(3)\n", 0 ; "append_to_other_list_between")]
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "xs = []\nxs.append(1)\nxs.append(2)\nxs = []\nxs.append(3)\nxs.append(4)\n", 1 ; "rebound_second_run_kept")]
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "def build() -> list[int]:\n    xs = []\n    xs.append(1)\n    xs.append(2)\n    return xs\n", 0 ; "function_scope")]
#[test_case("preview = true\nselect = [\"FURB113\"]\n", "def build(xs: list[int]) -> None:\n    xs = []\n    xs.append(1)\n    xs.append(2)\n", 1 ; "parameter_rebound_kept")]
fn collection_built_by_statements_leaves_repeated_appends_to_ruff(config: &str, source: &str, reported: usize) {
    let ruff = format!("[tool.ruff.lint]\n{config}");
    let root = common::project(&[("pyproject.toml", ruff.as_str()), ("module.py", source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP531", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP531\"").count(), reported, "{stdout}");
}

// RUF055 rewrites a `re.match` call with a plain-text pattern that is only tested for truth into `str.startswith`, when
// preview mode is on and the project selects it. SMP522 leaves those calls to Ruff and keeps the rest.
#[test_case("preview = true\n", "if re.match(\"ab\", text):\n        pass", 0 ; "if_test")]
#[test_case("", "if re.match(\"ab\", text):\n        pass", 1 ; "preview_off")]
#[test_case("preview = true\n", "while re.match(\"ab\", text):\n        break", 0 ; "while_test")]
#[test_case("preview = true\n", "if text:\n        pass\n    elif re.match(\"ab\", text):\n        pass", 0 ; "elif_test")]
#[test_case("preview = true\n", "assert re.match(\"ab\", text)", 0 ; "assert_test")]
#[test_case("preview = true\n", "assert text, re.match(\"ab\", text)", 1 ; "assert_message")]
#[test_case("preview = true\n", "return re.match(\"ab\", text)", 1 ; "value_used")]
#[test_case("preview = true\n", "return not re.match(\"ab\", text)", 0 ; "not_operand")]
#[test_case("preview = true\n", "return bool(re.match(\"ab\", text))", 0 ; "bool_argument")]
#[test_case("preview = true\n", "return str(re.match(\"ab\", text))", 1 ; "str_argument")]
#[test_case("preview = true\n", "return not (re.match(\"ab\", text) or text)", 0 ; "or_under_not")]
#[test_case("preview = true\n", "return re.match(\"ab\", text) or text", 1 ; "or_value_used")]
#[test_case("preview = true\n", "return 1 if re.match(\"ab\", text) else 2", 0 ; "ternary_test")]
#[test_case("preview = true\n", "return re.match(\"ab\", text) if text else 2", 1 ; "ternary_body")]
#[test_case("preview = true\n", "return [t for t in text if re.match(\"ab\", t)]", 0 ; "list_condition")]
#[test_case("preview = true\n", "return {t: 1 for t in text if re.match(\"ab\", t)}", 0 ; "dict_condition")]
#[test_case("preview = true\n", "return {t for t in text if re.match(\"ab\", t)}", 0 ; "set_condition")]
#[test_case("preview = true\n", "return any(True for t in text if re.match(\"ab\", t))", 0 ; "generator_condition")]
#[test_case("preview = true\n", "return [re.match(\"ab\", t) for t in text]", 1 ; "comprehension_element")]
#[test_case("preview = true\n", "return re.match", 1 ; "reference")]
#[test_case("preview = true\n", "match text:\n        case str() if re.match(\"ab\", text):\n            pass", 0 ; "match_guard")]
#[test_case("preview = true\n", "return re.match(\"ab\", text) is None", 0 ; "compared_to_none")]
#[test_case("preview = true\n", "return re.match(\"ab\", text) == None", 1 ; "equal_to_none")]
#[test_case("preview = true\n", "return re.match(\"a.\", text) is None", 1 ; "metacharacter")]
#[test_case("preview = true\n", "return re.match(b\"ab\", text) is None", 0 ; "bytes_pattern")]
#[test_case("preview = true\n", "return re.match(b\"a.\", text) is None", 1 ; "bytes_metacharacter")]
#[test_case("preview = true\n", "return re.match(PLAIN, text) is None", 0 ; "name_bound_once")]
#[test_case("preview = true\n", "return re.match(REBOUND, text) is None", 1 ; "name_bound_twice")]
#[test_case("preview = true\n", "return re.match(\"ab\", text, 0) is None", 1 ; "flags_argument")]
#[test_case("preview = true\n", "return re.match(\"ab\", *text) is None", 1 ; "starred_string")]
#[test_case("preview = true\n", "if (found := re.match(\"ab\", text)):\n        pass", 1 ; "walrus")]
fn up_re_prefixmatch_leaves_truth_tests_to_ruf055(preview: &str, body: &str, reported: usize) {
    let settings =
        format!("[project]\nrequires-python = \">=3.15\"\n\n[tool.ruff.lint]\n{preview}select = [\"RUF055\"]\n");
    let source =
        format!("import re\n\nPLAIN = \"ab\"\nREBOUND = \"ab\"\nREBOUND = \"cd\"\n\n\ndef check(text):\n    {body}\n");
    let root = common::project(&[("pyproject.toml", &settings), ("module.py", &source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP522", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP522\"").count(), reported, "{stdout}");
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
        "_core.py:1:28: SMP506 Every call to `_connect` passes `timeout`; drop its default `30`\nFound 1 error.\n1 unsafe \
         fix available with `--fix --unsafe-fixes`.\n"
    );
}
