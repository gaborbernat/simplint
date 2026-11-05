use tempfile::TempDir;
use test_case::test_case;

use crate::common::{self, check, read};

#[test_case("forwarding-function", "forwarding_function.py", "py314" ; "forwarding_function")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py314" ; "walrus_candidate")]
#[test_case("walrus-candidate", "walrus_candidate.py", "py37" ; "walrus_candidate_before_python38")]
#[test_case("walrus-candidate", "walrus_candidate_gate.py", "py314" ; "walrus_candidate_version_gate")]
#[test_case("walrus-candidate", "walrus_candidate_dynamic.py", "py314" ; "walrus_candidate_computed_all")]
#[test_case("walrus-candidate", "walrus_candidate_listed.py", "py314" ; "walrus_candidate_listed_in_all")]
#[test_case("walrus-candidate", "walrus_candidate_width.py", "py314" ; "walrus_candidate_header_width")]
#[test_case("single-caller-helper", "single_caller_helper.py", "py314" ; "single_caller_helper")]
#[test_case("single-caller-helper", "single_caller_helper_dynamic.py", "py314" ; "single_caller_helper_computed_all")]
#[test_case("single-use-variable", "single_use_variable.py", "py314" ; "single_use_variable")]
#[test_case("single-use-variable", "single_use_variable_width.py", "py314" ; "single_use_variable_width")]
#[test_case("single-use-variable", "single_use_variable_debug.py", "py314" ; "single_use_variable_debug")]
#[test_case("single-use-variable", "single_use_variable_requote.py", "py311" ; "single_use_variable_requote")]
#[test_case("string-concatenation", "string_concatenation.py", "py314" ; "string_concatenation")]
#[test_case("string-concatenation", "string_concatenation_old.py", "py310" ; "string_concatenation_py310")]
#[test_case("string-concatenation", "string_concatenation_types.py", "py314" ; "string_concatenation_types")]
#[test_case("string-concatenation", "string_concatenation_kinds.py", "py314" ; "string_concatenation_kinds")]
#[test_case("string-concatenation", "string_concatenation_genexp.py", "py314" ; "string_concatenation_genexp")]
#[test_case("string-concatenation", "string_concatenation_type_params.py", "py314" ; "string_concatenation_type_params")]
#[test_case("unused-default", "unused_default.py", "py314" ; "unused_default")]
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
fn walrus_candidate_keeping_if_branches_is_fixed(body: &str, fixed: &str) {
    let root = collapsible_if_project(&format!("def run(text, flag):\n{body}"));
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP502", "--fix"]);
    assert_eq!(
        (code, stdout.as_str(), read(root.path(), "module.py")),
        (0, "Found 1 error (1 fixed, 0 remaining).\n", format!("def run(text, flag):\n{fixed}"))
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
    "def quote(value):\n    quoted = value.replace(\"'\", \"''\")\n    return f\"'{quoted}'\"\n",
    None
    ; "quote_before_python312"
)]
#[test_case(
    "py312",
    "def quote(value):\n    quoted = value.replace(\"'\", \"''\")\n    return f\"'{quoted}'\"\n",
    Some("def quote(value):\n    return f\"'{value.replace(\"'\", \"''\")}'\"\n")
    ; "quote_from_python312"
)]
#[test_case(
    "py311",
    "def join(items):\n    joined = \"\\n\".join(items)\n    return f\"{joined}\"\n",
    None
    ; "backslash_before_python312"
)]
#[test_case(
    "py312",
    "def join(items):\n    joined = \"\\n\".join(items)\n    return f\"{joined}\"\n",
    Some("def join(items):\n    return f\"{\"\\n\".join(items)}\"\n")
    ; "backslash_from_python312"
)]
#[test_case(
    "py311",
    "def size(items):\n    count = len(items)\n    return f\"{count}\"\n",
    Some("def size(items):\n    return f\"{len(items)}\"\n")
    ; "other_value_before_python312"
)]
fn single_use_fstring_quote(target: &str, source: &str, inlined: Option<&str>) {
    let root = single_use_project(target, source);
    check(root.path(), &["--no-cache", "--select", "SMP504", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), inlined.unwrap_or(source));
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
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\napi-modules = [\"core\"]\n"),
        ("core.py", "__all__ = [\"connect\"]\n\n\ndef connect(host, timeout=30):\n    return host, timeout\n"),
        ("main.py", "from core import connect\n\nconnect(\"a\", 1)\n"),
    ]);
    let (code, stdout, _) = check(root.path(), &["--no-cache", "--select", "SMP506"]);
    assert_eq!((code, stdout.as_str()), (0, "All checks passed!\n"));
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
