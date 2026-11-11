use test_case::test_case;

use crate::common;

#[test_case("missing-from-all", "_missing_from_all.py" ; "missing_from_all")]
#[test_case("missing-from-all", "_missing_from_all_multiline.py" ; "missing_from_all_multiline")]
#[test_case("missing-from-all", "_missing_from_all_noall.py" ; "missing_from_all_creates_all")]
#[test_case("missing-from-all", "_missing_from_all_guard.py" ; "missing_from_all_above_main_guard")]
#[test_case("missing-from-all", "_missing_from_all_bare.py" ; "missing_from_all_bare_tuple")]
#[test_case("missing-from-all", "_missing_from_all_sorted.py" ; "missing_from_all_sorted_insertion")]
#[test_case("missing-from-all", "_missing_from_all_reexport.py" ; "missing_from_all_reexported_all")]
#[test_case("missing-from-all", "_missing_from_all_hanging.py" ; "missing_from_all_hanging_indent")]
#[test_case("missing-from-all", "_missing_from_all_hanging_comment.py" ; "missing_from_all_hanging_indent_comment")]
#[test_case("missing-from-all", "_missing_from_all_aug.py" ; "missing_from_all_augmented")]
#[test_case("missing-from-all", "_missing_from_all_call.py" ; "missing_from_all_dynamic")]
#[test_case("missing-from-all", "_missing_from_all_nested.py" ; "missing_from_all_nested_all")]
#[test_case("missing-from-all", "_missing_from_all_empty.py" ; "missing_from_all_empty")]
#[test_case("missing-from-all", "_missing_from_all_empty_comment.py" ; "missing_from_all_empty_comment")]
#[test_case("missing-from-all", "_missing_from_all_last_comment.py" ; "missing_from_all_last_comment")]
#[test_case("missing-from-all", "_missing_from_all_last_no_comma.py" ; "missing_from_all_last_no_comma")]
#[test_case("missing-from-all", "_missing_from_all_last_bracket.py" ; "missing_from_all_last_bracket")]
#[test_case("missing-from-all", "_missing_from_all_last_comma_line.py" ; "missing_from_all_last_comma_line")]
#[test_case("missing-from-all", "_missing_from_all_imported.py" ; "missing_from_all_imported_name")]
#[test_case("missing-from-all", "_missing_from_all_parenthesized_tail.py" ; "missing_from_all_parenthesized_tail")]
#[test_case("missing-from-all", "_missing_from_all_parenthesized_next.py" ; "missing_from_all_parenthesized_next")]
#[test_case("missing-from-all", "_missing_from_all_reached.py" ; "missing_from_all_reached_types")]
#[test_case("missing-from-all", "missing_from_all_scripts.py" ; "missing_from_all_entry_point")]
#[test_case("missing-from-all", "tests/test_module_docstring.py" ; "missing_from_all_test_module")]
#[test_case("missing-from-all", "package/__init__.py" ; "missing_from_all_init_module")]
#[test_case("unused-export", "_exporting/unused_export.py" ; "unused_export")]
#[test_case("unused-export", "_exporting/unused_export_inline.py" ; "unused_export_inline")]
#[test_case("unused-export", "_exporting/unused_export_tuple.py" ; "unused_export_tuple")]
#[test_case("unused-export", "_exporting/unused_export_all.py" ; "unused_export_all_unused")]
#[test_case("unused-export", "_exporting/unused_export_comment.py" ; "unused_export_comment")]
#[test_case("unused-export", "_exporting/unused_export_extend.py" ; "unused_export_extended")]
#[test_case("unused-export", "_exporting/unused_export_call.py" ; "unused_export_extend_call")]
#[test_case("unused-export", "_exporting/sub/__init__.py" ; "unused_export_init_module")]
#[test_case("unused-export", "_exporting/unused_export_double.py" ; "unused_export_double_import")]
#[test_case("unused-export", "_exporting/unused_export_comment_all.py" ; "unused_export_comment_all")]
#[test_case("unused-export", "_exporting/unused_export_lazy.py" ; "unused_export_lazy_module")]
#[test_case("unused-export", "_exporting/unused_export_packed.py" ; "unused_export_packed")]
#[test_case("unused-export", "_exporting/unused_export_tuple_end.py" ; "unused_export_tuple_end")]
#[test_case("unused-export", "_exporting/unused_export_parenthesized.py" ; "unused_export_parenthesized")]
#[test_case("unused-export", "_exporting/unused_export_trailing_comment.py" ; "unused_export_trailing_comment")]
#[test_case("unused-export", "_exporting/unused_export_kept_comment.py" ; "unused_export_kept_comment")]
#[test_case("unused-export", "_exporting/unused_export_kept_entry_comment.py" ; "unused_export_kept_entry_comment")]
#[test_case("unused-export", "_exporting/unused_export_statement_comment.py" ; "unused_export_statement_comment")]
#[test_case("unused-export", "_exporting/unused_export_noted.py" ; "unused_export_noted")]
#[test_case("unused-export", "_exporting/unused_export_star.py" ; "unused_export_star_imported")]
#[test_case("unused-export", "unused_export_root.py" ; "unused_export_api_root")]
#[test_case("unused-export", "unused_export_public_module.py" ; "unused_export_public_module")]
#[test_case("unused-export", "_exporting/unused_export_reached.py" ; "unused_export_reached_types")]
#[test_case("unused-export", "tests/test_module_docstring.py" ; "unused_export_test_module")]
#[test_case("unexported-public-name", "unexported_public_name.py" ; "unexported_public_name")]
#[test_case("unexported-public-name", "unexported_public_name_extend.py" ; "unexported_public_name_extended_all")]
#[test_case("unexported-public-name", "unexported_public_name_remove.py" ; "unexported_public_name_dynamic_all")]
#[test_case("unexported-public-name", "tests/test_module_docstring.py" ; "unexported_public_name_test_module")]
#[test_case("unexported-public-name", "multiline_module_docstring_ok.py" ; "unexported_public_name_without_all")]
#[test_case("unexported-public-name", "unexported_public_name_ply.py" ; "unexported_public_name_ply_grammar")]
#[test_case("unexported-public-name", "missing_from_all_reached.py" ; "unexported_public_name_reached_types")]
#[test_case("module-variable-prefix", "_module_variable_prefix.py" ; "module_variable_prefix")]
#[test_case("module-variable-prefix", "_module_variable_prefix_empty_name.py" ; "module_variable_prefix_empty_or_keyword_name")]
#[test_case("module-variable-prefix", "module_variable_prefix_source_text.py" ; "module_variable_prefix_source_text")]
#[test_case("module-variable-prefix", "module_variable_prefix_type_alias.py" ; "module_variable_prefix_type_alias")]
#[test_case("module-variable-prefix", "module_variable_prefix_function_alias.py" ; "module_variable_prefix_function_alias")]
#[test_case("module-variable-prefix", "_module_variable_prefix_block_local.py" ; "module_variable_prefix_block_local")]
#[test_case("module-variable-prefix", "_module_variable_prefix_computed_setattr.py" ; "module_variable_prefix_computed_setattr")]
#[test_case("module-variable-prefix", "module_variable_prefix_type_checking_stubs.py" ; "module_variable_prefix_type_checking_stubs")]
#[test_case("unexported-public-name", "unexported_public_name_source_text.py" ; "unexported_public_name_source_text")]
#[test_case("all-format", "all_format.py" ; "all_format")]
#[test_case("all-format", "all_format_tuple.py" ; "all_format_tuple")]
#[test_case("all-format", "all_format_bare.py" ; "all_format_bare_tuple")]
#[test_case("all-format", "all_format_ruff_overlap.py" ; "all_format_ruff_overlap")]
#[test_case("all-format", "all_format_parenthesized_last.py" ; "all_format_parenthesized_last")]
#[test_case("all-format", "all_format_parenthesized_tuple.py" ; "all_format_parenthesized_tuple")]
#[test_case("all-format", "all_format_augmented_tuple.py" ; "all_format_augmented_tuple")]
#[test_case("all-format", "all_format_augmented_literal.py" ; "all_format_augmented_literal")]
#[test_case("all-format", "all_format_augmented_nested.py" ; "all_format_augmented_nested")]
#[test_case("all-format", "all_format_augmented_noqa.py" ; "all_format_augmented_noqa")]
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
#[test_case("all-not-last", "all_not_last_dynamic_attribute.py" ; "all_not_last_dynamic_attribute")]
#[test_case("all-not-last", "all_not_last_dynamic_string.py" ; "all_not_last_dynamic_string")]
#[test_case("all-not-last", "all_not_last_dynamic_globals.py" ; "all_not_last_dynamic_globals")]
#[test_case("all-not-last", "all_not_last_dynamic_vars.py" ; "all_not_last_dynamic_vars")]
#[test_case("all-not-last", "all_not_last_dynamic_dir.py" ; "all_not_last_dynamic_dir")]
#[test_case("all-not-last", "all_not_last_function_reader.py" ; "all_not_last_read_by_function")]
#[test_case("all-not-last", "all_not_last_function_other.py" ; "all_not_last_function_without_read")]
#[test_case("all-not-last", "all_not_last_computed_value.py" ; "all_not_last_computed_value")]
#[test_case("all-not-last", "all_not_last_comment_above.py" ; "all_not_last_comment_above")]
#[test_case("all-not-last", "all_not_last_function_globals_string.py" ; "all_not_last_function_globals_string")]
#[test_case("all-not-last", "all_not_last_function_sysmod.py" ; "all_not_last_function_sysmod")]
#[test_case("all-not-last", "all_not_last_lambda_dir.py" ; "all_not_last_lambda_dir")]
#[test_case("all-not-last", "all_not_last_fence_comment.py" ; "all_not_last_fence_comment")]
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
#[test_case("unused-definition", "_unused_definition.py" ; "unused_definition")]
#[test_case("unused-definition", "_unused_definition_used.py" ; "unused_definition_imported")]
#[test_case("unused-definition", "tests/test_udef_helpers.py" ; "unused_definition_test_module")]
#[test_case("test-only-parameter", "_test_only_parameter.py" ; "test_only_parameter")]
#[test_case("unused-return-value", "_unused_return_value.py" ; "unused_return_value")]
#[test_case("unused-return-value", "_unused_return_value_listed.py" ; "unused_return_value_listed")]
#[test_case("unused-definition", "_unused_definition_frozen_core.py" ; "unused_definition_unresolved_import")]
#[test_case("unused-definition", "_unused_definition_doctest.py" ; "unused_definition_doctest")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test_case("", "", 0 ; "paired_test_only_without_all")]
#[test_case("__all__ = []\n", "", 0 ; "paired_test_only_with_all")]
#[test_case("", "from pkg._render.intersphinx import Hint\n", 1 ; "production_import_without_all")]
#[test_case("__all__ = []\n", "from pkg._render.intersphinx import Hint\n", 1 ; "production_import_with_all")]
fn missing_from_all_ignores_test_only_imports(exports: &str, production: &str, expected: usize) {
    let root = common::project(&[
        ("src/pkg/__init__.py", ""),
        ("src/pkg/_render/__init__.py", ""),
        ("src/pkg/_render/intersphinx.py", &format!("class Hint:\n    pass\n\n{exports}")),
        ("src/pkg/use.py", production),
        ("tests/_render/test_intersphinx.py", "from pkg._render.intersphinx import Hint\n"),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP401"]).1;
    assert_eq!(common::reports(&stdout, "SMP401").len(), expected, "{stdout}");
}

#[test]
fn init_module_content_allows_test_packages_and_pep_562_hooks() {
    let root = common::project(&[
        ("tests/__init__.py", common::CLEAN_MODULE),
        ("roots/test-example/pyproject.toml", "[project]\nname = \"example\"\n"),
        ("roots/test-example/package/__init__.py", common::CLEAN_MODULE),
        (
            "package/__init__.py",
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from package import model\n\n\ndef __getattr__(name: str):\n    return name\n\n\ndef __dir__():\n    return []\n\nVALUE = 1\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP407"]);
    assert_eq!(common::reports(&stdout, "SMP407").len(), 2, "{stdout}");
    assert!(!stdout.contains("tests/__init__.py"), "{stdout}");
    assert!(!stdout.contains("roots/test-example"), "{stdout}");
}

#[test]
fn init_module_content_allows_explicit_api_in_applications() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("package/__init__.py", "from ._api import API\nfrom ._api import hidden\n\n__all__ = [\"API\"]\n"),
        ("package/_api.py", "API = 1\nhidden = 2\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP407"]);
    assert_eq!(common::reports(&stdout, "SMP407").len(), 1, "{stdout}");
    assert!(stdout.contains("package/__init__.py:2:1"), "{stdout}");
}

#[test]
fn missing_from_all_allows_main_modules() {
    let root = common::project(&[
        ("package/__main__.py", common::CLEAN_MODULE),
        ("package/module.py", common::CLEAN_MODULE),
        ("package/user.py", "from package.module import VALUE\n\nprint(VALUE)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP401"]);
    assert_eq!(common::reports(&stdout, "SMP401").len(), 1, "{stdout}");
    assert!(stdout.contains("module.py"), "{stdout}");
}

// Listing a name drops the `noqa` that named only F401 for its import, wherever the import is split, and keeps the
// comments that name other codes.
#[test]
fn missing_from_all_drops_the_noqa_it_leaves_unused() {
    let enc = "from os.path import join  # noqa: F401\nfrom os.path import (\n    sep,  # noqa: F401\n    exists,  # noqa: \
               E501\n)\nfrom os.path import isdir  # noqa\nfrom os.path import isabs  # type: ignore\n\n\ndef run() -> \
               int:\n    return 1\n";
    let root = common::project(&[
        ("enc.py", enc),
        ("use.py", "from enc import join, sep, exists, isdir, isabs\n\nprint(join, sep, exists, isdir, isabs)\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "enc.py"),
        "from os.path import join\nfrom os.path import (\n    sep,\n    exists,  # noqa: E501\n)\nfrom os.path import \
         isdir\nfrom os.path import isabs  # type: ignore\n\n\ndef run() -> int:\n    return 1\n\n\n__all__ = [\n    \
         \"exists\",\n    \"isabs\",\n    \"isdir\",\n    \"join\",\n    \"sep\",\n]\n"
    );
}

// Two names imported from one line share its `noqa`. The fix removes it once and in order, so the full report of the
// unapplied unsafe fix renders instead of panicking.
#[test]
fn missing_from_all_drops_a_noqa_shared_by_names_on_one_line() {
    let root = common::project(&[
        ("enc.py", "from os.path import join, sep  # noqa: F401\n\n\ndef run() -> int:\n    return 1\n"),
        ("use.py", "from enc import join, sep\n\nprint(join, sep)\n"),
    ]);
    let (code, _, stderr) = common::simplint(root.path(), &["check", "--select", "SMP401"]);
    assert_eq!((code, stderr.as_str()), (1, ""));
}

// A `noqa` for F401 stays while another name its import binds is left out of the listing, since that name still needs it.
#[test]
fn missing_from_all_keeps_a_noqa_another_name_on_its_line_needs() {
    let root = common::project(&[
        ("enc.py", "from os.path import join, sep  # noqa: F401\n\n\ndef run() -> int:\n    return 1\n"),
        ("use.py", "from enc import join\n\nprint(join)\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "enc.py"),
        "from os.path import join, sep  # noqa: F401\n\n\ndef run() -> int:\n    return 1\n\n\n__all__ = [\n    \"join\",\n]\n"
    );
}

// Each report of a listing shows the whole fix in the full output, so a module with many missing names shows it once.
#[test]
fn missing_from_all_shows_the_fix_once_for_many_names() {
    let names: Vec<String> = (0..200).map(|index| format!("f{index}")).collect();
    let module =
        (0..200).map(|index| format!("def f{index}() -> int:\n    return {index}\n\n\n")).collect::<Vec<_>>().concat();
    let root = common::project(&[("enc.py", &module), ("use.py", &format!("from enc import {}\n", names.join(", ")))]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP401", "--output-format", "full"]);
    assert_eq!(stdout.matches("help: Add ").count(), 1, "{stdout}");
}

#[test]
fn missing_from_all_fixes_after_first_report_is_suppressed() {
    let root = common::project(&[
        (
            "api.py",
            "__all__ = []\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n\n\ndef beta() -> int:\n    return 2\n",
        ),
        ("use.py", "from api import alpha, beta\n\nprint(alpha(), beta())\n"),
    ]);
    common::check(root.path(), &["--no-cache", "--fix", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "api.py"),
        "__all__ = [\n    \"alpha\",\n    \"beta\",\n]\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n\n\ndef beta() -> int:\n    return 2\n"
    );
}

#[test]
fn missing_from_all_fixes_with_all_suppressions_ignored() {
    let root = common::project(&[
        ("api.py", "__all__ = []\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n"),
        ("use.py", "from api import alpha\n\nprint(alpha())\n"),
    ]);
    common::check(root.path(), &["--no-cache", "--ignore-noqa", "--fix", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "api.py"),
        "__all__ = [\n    \"alpha\",\n]\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n"
    );
}

// A name bound only under `if TYPE_CHECKING:` exists for type checkers alone, so listing it would make `import *` fail,
// while a name the module also binds at run time is still reported.
#[test]
fn missing_from_all_leaves_out_names_bound_only_for_type_checkers() {
    let root = common::project(&[
        (
            "mod.py",
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    class Locked(dict): ...\n\n\nclass Both: ...\n\n\nif TYPE_CHECKING:\n    Both = int\n\n\ndef use(x: \"Locked\") -> None: ...\n\n\n__all__ = [\"use\"]\n",
        ),
        ("use.py", "from mod import Locked, Both, use\n\nprint(Locked, Both, use)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP401"]);
    assert!(stdout.contains("`Both` is imported by other modules"), "{stdout}");
    assert!(!stdout.contains("`Locked`"), "{stdout}");
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

// A public name the run never reads is reported, unless a computed read may fetch it, which keeps every public name.
#[test]
fn unused_definition_reports_public_names_nothing_reads() {
    let reported = unused_definitions("return None");
    assert!(reported.contains("`helper` is never used"), "{reported}");
    assert!(reported.contains("`Client.send` is never called"), "{reported}");
    assert!(reported.contains("`Client.LIMIT` is never read"), "{reported}");
}

#[test_case("getattr(obj, name)" ; "computed_getattr")]
#[test_case("vars(obj)" ; "vars")]
#[test_case("dir(obj)" ; "dir")]
#[test_case("obj.__dict__" ; "dict")]
fn unused_definition_keeps_public_names_under_computed_reads(read: &str) {
    let reported = unused_definitions(&format!("return {read}"));
    assert!(!reported.contains("SMP409"), "{read} may read any public name: {reported}");
}

#[test]
fn unused_definition_keeps_only_the_member_a_literal_getattr_names() {
    let reported = unused_definitions("return getattr(obj, \"send\")");
    assert!(!reported.contains("`Client.send`"), "{reported}");
    assert!(reported.contains("`helper` is never used"), "{reported}");
    assert!(reported.contains("`Client.LIMIT` is never read"), "{reported}");
}

// The diagnostics of a project whose `read` returns `body`, with `main` reading the class and calling `read`, so
// only the definitions of `core` can be unused.
fn unused_definitions(body: &str) -> String {
    let core = "class Client:\n    LIMIT = 3\n\n    def send(self) -> None:\n        pass\n\n\ndef helper() -> None:\n    pass\n";
    let reader = format!("def read(obj: object, name: str = \"send\") -> object:\n    {body}\n");
    let main = "from app.core import Client\nfrom app.reader import read\n\nprint(read(Client()))\n";
    let root = common::project(&[("app/core.py", core), ("app/reader.py", reader.as_str()), ("app/main.py", main)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    stdout
}

// A private method is judged by its own file, so a computed read there keeps it. A read in another file keeps it only
// when the name it computes starts with a literal prefix of the method's, as a dispatch table's does.
#[test_case("'_' + kind", true, false ; "same_file")]
#[test_case("'_' + kind", false, true ; "other_file_underscore_prefix")]
#[test_case("kind", false, true ; "other_file_no_prefix")]
#[test_case("'_handle_' + kind", false, false ; "other_file_matching_prefix")]
#[test_case("f'_handle_{kind}'", false, false ; "other_file_matching_fstring_prefix")]
#[test_case("f'{kind}_handle'", false, true ; "other_file_fstring_without_prefix")]
#[test_case("'_visit_' + kind", false, true ; "other_file_other_prefix")]
fn unused_definition_judges_a_private_method_by_its_own_file(name: &str, same_file: bool, reported: bool) {
    let dispatch = format!("def dispatch(obj: object, kind: str) -> object:\n    return getattr(obj, {name})\n");
    let handler = "class _Handler:\n    def _handle_text(self) -> None:\n        pass\n";
    let core = if same_file { format!("{handler}\n\n{dispatch}") } else { handler.to_owned() };
    let reader = if same_file { String::new() } else { dispatch };
    let root = common::project(&[
        ("app/core.py", core.as_str()),
        ("app/other.py", reader.as_str()),
        ("app/main.py", "from app.core import _Handler\n\nprint(_Handler)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(stdout.contains("`_Handler._handle_text`"), reported, "{stdout}");
}

const RUF067_PREVIEW: &str = "[tool.ruff.lint]\npreview = true\nselect = [\"RUF067\"]\n";
const RUF067_STRICT: &str = "[tool.ruff.lint]\npreview = true\nselect = [\"RUF067\"]\n[tool.ruff.lint.ruff]\nstrictly-empty-init-modules = true\n";
// RUF067 reports the statements of an `__init__.py` only in preview mode, and then the statements it does not allow, so
// the run leaves those to Ruff. The default allows imports, dunder assignments and `if TYPE_CHECKING` blocks, and a
// project that bans all code leaves nothing to the run but attribute docstrings.
#[test_case(RUF067_PREVIEW, common::CLEAN_MODULE, 0 ; "preview_assignment")]
#[test_case(RUF067_PREVIEW, "import os\n", 1 ; "preview_import_allowed")]
#[test_case(RUF067_PREVIEW, "__version__ = \"1\"\n", 1 ; "preview_dunder_allowed")]
#[test_case(RUF067_PREVIEW, "if TYPE_CHECKING:\n    pass\n", 0 ; "preview_type_checking_allowed")]
#[test_case(RUF067_PREVIEW, "def helper() -> None:\n    pass\n", 0 ; "preview_function")]
#[test_case(RUF067_PREVIEW, "X = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_attribute_docstring")]
#[test_case(RUF067_PREVIEW, "def helper() -> None:\n    pass\n\n\"stray\"\n", 0 ; "preview_stray_string")]
#[test_case(RUF067_STRICT, "import os\n", 0 ; "preview_strict_import")]
#[test_case(RUF067_STRICT, "X = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_strict_attribute_docstring")]
#[test_case("[tool.ruff.lint]\nselect = [\"RUF067\"]\n", common::CLEAN_MODULE, 1 ; "preview_off_leaves_it_to_the_run")]
#[test_case(RUF067_PREVIEW, "__all__: list[str] = []\n", 0 ; "preview_dunder_annotated_allowed")]
#[test_case(RUF067_PREVIEW, "__all__ += [\"x\"]\n", 1 ; "preview_dunder_augmented_allowed")]
#[test_case(RUF067_PREVIEW, "counter += 1\n", 0 ; "preview_augmented_reported")]
#[test_case(RUF067_PREVIEW, "import typing\n\nif typing.TYPE_CHECKING:\n    pass\n", 1 ; "preview_attribute_type_checking_allowed")]
#[test_case(RUF067_PREVIEW, "if callable(print):\n    pass\n", 0 ; "preview_other_if_reported")]
#[test_case(RUF067_STRICT, "X: int = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_strict_annotated_attribute_docstring")]
fn init_module_content_leaves_ruf067_to_ruff(config: &str, source: &str, reported: usize) {
    let root = common::project(&[("pyproject.toml", config), ("pkg/__init__.py", source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP407", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP407\"").count(), reported, "{stdout}");
}

// PYI046 and PYI049 report a private protocol or TypedDict that its file never reads, so the run leaves those to Ruff
// when the project selects the rule. A class Ruff reads, or one that is not its last binding, stays with the run.
#[test_case("from typing import Protocol\n\n\nclass _Proto(Protocol):\n    def run(self) -> None: ...\n", "\"PYI046\"", 0 ; "protocol_selected")]
#[test_case("from typing import Protocol\n\n\nclass _Proto(Protocol):\n    def run(self) -> None: ...\n", "\"PYI049\"", 1 ; "protocol_other_rule")]
#[test_case("from typing import Protocol\n\n\nclass _Proto(Protocol):\n    def run(self) -> None: ...\n", "", 1 ; "protocol_unselected")]
#[test_case("import typing_extensions\n\n\nclass _Proto(typing_extensions.Protocol[int]):\n    pass\n", "\"PYI046\"", 0 ; "protocol_typing_extensions_subscript")]
#[test_case("from mylib import Protocol\n\n\nclass _Proto(Protocol):\n    pass\n", "\"PYI046\"", 1 ; "protocol_foreign")]
#[test_case("from typing import TypedDict\n\n\nclass _Row(TypedDict):\n    a: int\n", "\"PYI049\"", 0 ; "typed_dict_selected")]
#[test_case("from typing import TypedDict\n\n\nclass _Row(TypedDict):\n    a: int\n", "\"PYI046\"", 1 ; "typed_dict_other_rule")]
#[test_case("from typing import Protocol\n\n\nclass _P(Protocol):\n    pass\n\n\nclass _P(Protocol):\n    pass\n", "\"PYI046\"", 1 ; "protocol_rebound")]
#[test_case("from __future__ import annotations\n\nfrom typing import Protocol\n\n\nclass _P(Protocol):\n    def run(self) -> _P: ...\n", "\"PYI046\"", 1 ; "protocol_read_by_own_annotation")]
#[test_case("from typing import Protocol\n\n\nclass _P(Protocol):\n    def run(self) -> '_P': ...\n", "\"PYI046\"", 1 ; "protocol_string_annotation")]
#[test_case("from typing import Protocol\n\n\nclass _P(Protocol):\n    \"\"\"Protocol of _P.\"\"\"\n", "\"PYI046\"", 0 ; "protocol_docstring_mention")]
fn unused_definition_leaves_private_protocols_and_typed_dicts_to_ruff(source: &str, select: &str, reported: usize) {
    let ruff = format!("[tool.ruff.lint]\nselect = [{select}]\n");
    let root = common::project(&[("pyproject.toml", ruff.as_str()), ("module.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(common::reports(&stdout, "SMP409").len(), reported, "{stdout}");
}

// A method a version switch defines inside an `if` in the class body is the class's method, so its calls count.
#[test]
fn unused_definition_counts_calls_of_a_method_under_an_if_block() {
    let root = common::project(&[
        (
            "app/core.py",
            "class Client:\n    if True:\n        def _send(self) -> None:\n            pass\n\n    def run(self) -> None:\n        self._send()\n",
        ),
        ("app/main.py", "from app.core import Client\n\nprint(Client().run())\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert!(!stdout.contains("`Client._send`"), "{stdout}");
}

#[test]
fn unused_definition_keeps_framework_hooks_called_by_name() {
    let root = common::project(&[(
        "app/hooks.py",
        "def setup(app: object) -> None:\n    print(app)\n\n\ndef pytest_configure(config: object) -> None:\n    print(config)\n",
    )]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(stdout, common::PASSED);
}

// A definition a page of the project's documentation names is used: by a Sphinx role, an autodoc directive, or the
// options of a directive that renders a module's parser. Hidden and excluded directories, and files that are neither
// reStructuredText nor Markdown, are not read.
#[test_case("docs/index.rst", ":func:`app.core.helper`", false, true ; "role")]
#[test_case("docs/index.rst", ":py:func:`~app.core.helper`", false, true ; "domain_role_short")]
#[test_case("docs/index.rst", "See :meth:`sending <app.core.Client.send>`.", true, false ; "role_with_title")]
#[test_case("docs/index.rst", ".. currentmodule:: app.core\n\nCall :func:`helper`.", false, true ; "current_module")]
#[test_case("docs/index.rst", ".. autofunction:: app.core.helper", false, true ; "autofunction")]
#[test_case("docs/index.rst", ".. module:: app.core\n.. autofunction:: helper\n", false, true ; "directive_after_directive")]
#[test_case("docs/index.rst", ".. autoclass:: app.core.Client\n   :members:\n", true, false ; "autoclass_members")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n   :members:\n", false, false ; "automodule_members")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n   :members: helper\n", false, true ; "automodule_listed")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n", true, true ; "automodule_alone")]
#[test_case("docs/cli.rst", ".. sphinx_argparse_cli::\n   :module: app.core\n   :func: helper\n", false, true ; "argparse_cli")]
#[test_case("docs/cli.rst", ".. argparse::\n   :ref: app.core.helper\n   :prog: app\n", false, true ; "argparse_ref")]
#[test_case("docs/api.md", "Call {py:meth}`app.core.Client.send`.", true, false ; "myst_role")]
#[test_case("docs/api.md", "```{autofunction} app.core.helper\n```\n", false, true ; "myst_directive")]
#[test_case("docs/api.md", ":::{autoclass} app.core.Client\n:members: send\n:::\n", true, false ; "myst_colon_directive")]
#[test_case("docs/index.rst", ":func:`app.core.other`", true, true ; "other_name")]
#[test_case("docs/index.rst", "``helper`` and :unknown:`app.core.helper`", true, true ; "not_a_python_role")]
#[test_case(".venv/lib/page.rst", ":func:`app.core.helper`", true, true ; "excluded_directory")]
#[test_case(".docs/index.rst", ":func:`app.core.helper`", true, true ; "hidden_directory")]
#[test_case("docs/notes.txt", ":func:`app.core.helper`", true, true ; "plain_text")]
fn unused_definition_reads_the_documentation(page: &str, text: &str, helper: bool, send: bool) {
    let core = "def helper() -> None:\n    pass\n\n\nclass Client:\n    def send(self) -> None:\n        pass\n";
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\n"),
        ("app/__init__.py", ""),
        ("app/core.py", core),
        ("app/main.py", "from app.core import Client\n\nprint(Client())\n"),
        (page, text),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(
        (stdout.contains("`helper` is never used"), stdout.contains("`Client.send` is never called")),
        (helper, send),
        "{stdout}"
    );
}

#[test_case(".gitignore", "build/", "build/api.rst" ; "gitignore")]
#[test_case(".ignore", "private/", "private/api.rst" ; "ignore_file")]
fn unused_definition_skips_documentation_an_ignore_file_hides(ignore_file: &str, pattern: &str, page: &str) {
    let root = common::project(&[
        (".git/HEAD", "ref: refs/heads/main\n"),
        ("pyproject.toml", "[project]\nname = \"app\"\n"),
        ("app/__init__.py", ""),
        ("app/core.py", "def helper() -> None:\n    pass\n"),
        (ignore_file, &format!("{pattern}\n")),
        (page, ":func:`app.core.helper`\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert!(stdout.contains("`helper` is never used"), "{stdout}");
}

// Only a project with a `pyproject.toml` has documentation to read, and only a run with `unused-definition` on reads it.
#[test]
fn unused_definition_reads_no_documentation_without_a_project() {
    let root = common::project(&[
        ("app/core.py", "def helper() -> None:\n    pass\n"),
        ("docs/index.rst", ":func:`app.core.helper`\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert!(stdout.contains("`helper` is never used"), "{stdout}");
}

// The documentation is part of what each file's cached result depends on, so a page that starts naming a definition
// clears its report on the next run.
#[test]
fn unused_definition_cache_follows_the_documentation() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"app\"\n"),
        ("app/__init__.py", ""),
        ("app/core.py", "def helper() -> None:\n    pass\n"),
        ("docs/index.rst", "Nothing yet.\n"),
    ]);
    let check = ["check", "--select", "SMP409", "--output-format", "concise"];
    let (_, before, _) = common::simplint(root.path(), &check);
    std::fs::write(root.path().join("docs/index.rst"), ".. autofunction:: app.core.helper\n").expect("write page");
    let (_, after, _) = common::simplint(root.path(), &check);
    assert_eq!((before.contains("`helper`"), after.contains("`helper`")), (true, false), "{before}\n{after}");
}

// A class read by its name, as `type(self).__name__` reads it, must keep the name it has.
#[test_case("type(self).__name__" ; "through_type")]
#[test_case("self.__class__.__qualname__" ; "through_class_attribute")]
fn unexported_public_name_keeps_a_class_read_by_its_name(read: &str) {
    let source = format!("__all__ = []\n\n\nclass Item:\n    def label(self) -> str:\n        return {read}\n");
    let root = common::project(&[("mod.py", &source)]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 `Item` is not in `__all__`"));
    assert!(!stdout.contains("SMP403 [*]"));
}

// Code that reads a function's or class's name off an object, in its own module or elsewhere, keeps the name it has.
#[test_case("mod.py", "def helper() -> str:\n    return helper.__name__\n" ; "own_module")]
#[test_case("mod.py", "def helper() -> str:\n    return helper.__qualname__\n" ; "own_qualname")]
#[test_case("consumer.py", "def use(obj: object) -> str:\n    return obj.helper.__name__\n" ; "attribute_elsewhere")]
fn unexported_public_name_keeps_a_function_read_by_its_name(reader: &str, source: &str) {
    let module = if reader == "mod.py" {
        format!("__all__ = []\n\n\n{source}")
    } else {
        "__all__ = []\n\n\ndef helper() -> None:\n    pass\n".to_owned()
    };
    let root =
        common::project(&[("mod.py", &module), ("consumer.py", if reader == "consumer.py" { source } else { "" })]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 `helper` is not in `__all__`"));
    assert!(!stdout.contains("SMP403 [*]"));
}

// A classmethod reads its class's name off `cls`, so the class keeps its name.
#[test]
fn unexported_public_name_keeps_a_class_its_classmethod_reads() {
    let root = common::project(&[(
        "mod.py",
        "__all__ = []\n\n\nclass Item:\n    @classmethod\n    def label(cls) -> str:\n        return cls.__name__\n",
    )]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 `Item` is not in `__all__`"));
    assert!(!stdout.contains("SMP403 [*]"));
}

// A name read off a value that is neither a variable, an attribute nor a call names no class or function.
#[test]
fn unexported_public_name_renames_a_class_read_off_a_subscript() {
    let root = common::project(&[(
        "mod.py",
        "__all__ = []\n\n\nclass Item:\n    def label(self) -> str:\n        return self.items[0].__name__\n",
    )]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 [*]"));
}

// A class read through an attribute of an object elsewhere keeps its name too.
#[test]
fn unexported_public_name_keeps_a_class_read_through_an_attribute() {
    let root = common::project(&[
        ("mod.py", "__all__ = []\n\n\nclass Item:\n    pass\n"),
        ("consumer.py", "def use(module: object) -> str:\n    return module.Item.__qualname__\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 `Item` is not in `__all__`"));
    assert!(!stdout.contains("SMP403 [*]"));
}

// A function nobody reads by name is still renamed, so the withheld fixes above are the reflective reads alone.
#[test]
fn unexported_public_name_renames_a_function_nobody_reads_by_name() {
    let root = common::project(&[("mod.py", "__all__ = []\n\n\ndef helper() -> None:\n    pass\n")]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 [*]"));
}

// The name a script's `main` is looked up by in a test is a string of another file, which keeps `main` in place.
#[test]
fn unexported_public_name_checks_strings_of_other_files() {
    let root = common::project(&[
        ("tools.py", "__all__ = []\n\n\ndef main() -> None:\n    pass\n"),
        ("tests/test_tools.py", "ENTRY = \"main\"\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 `main` is not in `__all__`"));
    assert!(!stdout.contains("SMP403 [*]"));
}

// Ruff's COM812 reports a list whose closing bracket follows a line break with no trailing comma, which SMP405 leaves to
// it while the project's Ruff configuration selects it. A list that closes on its last element's line is not COM812's.
#[test_case("__all__ = [\n    \"a\",\n    \"b\"\n]\n", false ; "closing_bracket_on_own_line")]
#[test_case("__all__ = [\n    \"a\"  # note\n]\n", false ; "comment_before_closing_bracket")]
#[test_case("__all__ = [\n    (\"a\")\n]\n", false ; "parenthesized_last_element")]
#[test_case("__all__ = [\n    \"a\",\n    \"b\"]\n", true ; "closing_bracket_after_last_element")]
#[test_case("__all__ = [\"a\", \"b\"]\n", true ; "single_line")]
#[test_case("__all__ = [\n    \"a\" \\\n]\n", true ; "line_continuation")]
fn all_format_follows_ruff_com812(source: &str, reported_when_selected: bool) {
    let case =
        common::RuffOverlap { settings: "", select: "SMP405", ruff_code: "COM812", files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, "has no trailing comma"), [true, reported_when_selected]);
}

// A parenthesized last element keeps the comma after it, so the list has one and SMP405 has nothing to add.
#[test]
fn all_format_keeps_parenthesized_trailing_comma() {
    let root = common::project(&[("module.py", "__all__ = [\n    (\"a\"),\n]\n")]);
    let stdout = common::check(root.path(), &["--select", "SMP405"]).1;
    assert!(!stdout.contains("has no trailing comma"), "{stdout}");
}

// The `__all__` of a public module of a library is the interface its author declares, so no importer in the run is
// needed. A private module and an application keep reporting the names nothing imports.
#[test_case("project-kind = \"library\"", "core.py", 0 ; "public_module_of_library")]
#[test_case("project-kind = \"library\"", "_core.py", 1 ; "private_module_of_library")]
#[test_case("project-kind = \"application\"", "core.py", 1 ; "public_module_of_application")]
fn unused_export_keeps_the_declared_api_of_libraries(settings: &str, module: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.simplint]\n{settings}\n")),
        (module, "def run() -> None: ...\n\n\n__all__ = [\"run\"]\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP402"]);
    assert_eq!(common::reports(&stdout, "SMP402").len(), reports, "{stdout}");
}

// A public module's names are the API its users import, so the rename to private stays with the author, though the
// report stays. A private module and an application keep the rename.
#[test_case("unexported-public-name", "def helper() -> None: ...\n\n\n__all__ = []\n", "def _helper() -> None: ...\n\n\n__all__ = []\n", "library", "core.py", false ; "function_public_module_of_library")]
#[test_case("unexported-public-name", "def helper() -> None: ...\n\n\n__all__ = []\n", "def _helper() -> None: ...\n\n\n__all__ = []\n", "library", "_core.py", true ; "function_private_module_of_library")]
#[test_case("unexported-public-name", "def helper() -> None: ...\n\n\n__all__ = []\n", "def _helper() -> None: ...\n\n\n__all__ = []\n", "application", "core.py", true ; "function_public_module_of_application")]
#[test_case("module-variable-prefix", "limit = 3\n\n\n__all__ = []\n", "_limit = 3\n\n\n__all__ = []\n", "library", "core.py", false ; "variable_public_module_of_library")]
#[test_case("module-variable-prefix", "limit = 3\n\n\n__all__ = []\n", "_limit = 3\n\n\n__all__ = []\n", "library", "_core.py", true ; "variable_private_module_of_library")]
#[test_case("module-variable-prefix", "limit = 3\n\n\n__all__ = []\n", "_limit = 3\n\n\n__all__ = []\n", "application", "core.py", true ; "variable_public_module_of_application")]
fn private_rename_follows_the_module_role(
    rule: &str,
    source: &str,
    private: &str,
    kind: &str,
    module: &str,
    renamed: bool,
) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.simplint]\nproject-kind = \"{kind}\"\n")),
        (module, source),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", rule, "--fix", "--unsafe-fixes"]);
    let expected = if renamed { private } else { source };
    assert_eq!(common::read(root.path(), module), expected, "{rule}: {stdout}");
}

// Listing a name in a public module of a library widens its API and what `import *` binds, so the report stays without a
// fix. A private module keeps the safe fix.
#[test_case("core.py", "def run() -> None: ...\n" ; "public_module_of_library_no_fix")]
#[test_case("_core.py", "def run() -> None: ...\n\n\n__all__ = [\n    \"run\",\n]\n" ; "private_module_of_library_fixed")]
fn missing_from_all_offers_no_fix_in_public_library_modules(module: &str, expected: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (module, "def run() -> None: ...\n"),
        ("use.py", &format!("from {} import run\n\nrun()\n", module.trim_end_matches(".py"))),
    ]);
    common::check(root.path(), &["--no-cache", "--select", "SMP401", "--fix", "--unsafe-fixes"]);
    assert_eq!(common::read(root.path(), module), expected);
}

// A name a doctest imports is read by code the index cannot follow, so `unused-export` keeps it in `__all__`, and its
// fix leaves the import the doctest needs.
#[test]
fn unused_export_keeps_a_name_a_doctest_imports() {
    let source = "from app.impl import helper\n\n__all__ = [\"helper\"]\n";
    let root = common::project(&[
        ("app/__init__.py", ""),
        ("app/impl.py", "def helper() -> int:\n    return 1\n"),
        ("app/mod.py", source),
        (
            "app/cli.py",
            "\"\"\"Command line.\n\n    >>> from app.mod import helper\n    >>> helper()\n    1\n    \"\"\"\n",
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP402"]);
    assert_eq!(common::read(root.path(), "app/mod.py"), source);
}

// A public module of a library keeps the names it offers, and so does a name `__all__` lists there, so their renames
// stay with the author. The rules still report them.
#[test]
fn public_module_names_are_not_renamed_by_fixes() {
    let source =
        "__all__ = [\"_limit\", \"level\"]\n\n_limit = 3\n\"\"\"Limit.\"\"\"\n\nlevel = 4\n\"\"\"Level.\"\"\"\n";
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("lib/__init__.py", ""),
        ("lib/api.py", source),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP404,SMP102"]);
    assert_eq!(common::read(root.path(), "lib/api.py"), source);
    assert!(stdout.contains("SMP102") && stdout.contains("SMP404"), "{stdout}");
}

// An import of a name the run cannot resolve, such as a stdlib one, says nothing about the names of the project, so a
// block-local name of another module stays block-local and a module variable of the project stays reported.
#[test]
fn unresolved_import_does_not_change_the_module_variables_of_other_modules() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("app/__init__.py", ""),
        (
            "app/mod.py",
            "import sys\n\nwith open(__file__) as handle:\n    count = len(handle.read())\n    print(count)\n",
        ),
        ("app/other.py", "from itertools import count\n\nprint(count)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP102,SMP404"]);
    assert_eq!(stdout, common::PASSED);
}

// A name an unresolved import may read keeps its report, and only the rename is withheld.
#[test]
fn unresolved_import_keeps_the_report_but_withholds_the_rename() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("app/__init__.py", ""),
        ("app/mod.py", "def loads() -> int:\n    return 1\n\n\n__all__ = []\n"),
        ("app/other.py", "from json import loads\n\nprint(loads)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP403"]);
    assert!(stdout.contains("`loads` is not in `__all__`"), "{stdout}");
    assert!(!stdout.contains("[fix"), "{stdout}");
}

// A doctest that imports a name from its module reads it, so the name is neither reported as private nor as unused.
#[test]
fn doctest_import_counts_as_a_read_of_the_name() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("app/__init__.py", ""),
        ("app/mod.py", "LIMIT = 3\n\n\ndef helper() -> int:\n    return LIMIT\n\n\n__all__ = []\n"),
        (
            "app/cli.py",
            "def run() -> None:\n    \"\"\"Run.\n\n    >>> from app.mod import LIMIT, helper\n    >>> helper() == LIMIT\n    True\n    \"\"\"\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP404,SMP409"]);
    assert!(!stdout.contains("`LIMIT`"), "{stdout}");
    assert!(!stdout.contains("`helper`"), "{stdout}");
}

// A documented function is public to SMP408 too, so a test module that calls it does not make it test-only code.
#[test]
fn documented_function_is_not_test_only_code() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("app/__init__.py", ""),
        ("app/mod.py", "def helper(value: int = 1) -> int:\n    return value\n"),
        ("docs/conf.py", ""),
        ("docs/index.rst", "API\n===\n\n.. autofunction:: app.mod.helper\n"),
        (
            "tests/test_mod.py",
            "from app.mod import helper\n\n\ndef test_helper() -> None:\n    assert helper(2) == 2\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP408"]);
    assert_eq!(stdout, common::PASSED);
}

// A listed name with a leading underscore keeps its name in a public module of a library, so its report asks for the
// rename by hand rather than offering it.
#[test]
fn listed_private_name_is_renamed_by_hand_in_public_library_modules() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("lib/__init__.py", ""),
        ("lib/api.py", "__all__ = [\"_LIMIT\"]\n\n_LIMIT = 3\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP404"]);
    assert!(stdout.contains("`_LIMIT` is in `__all__`; rename it to `LIMIT` by hand"), "{stdout}");
    assert!(!stdout.contains("[fix"), "{stdout}");
}

// A doctest's relative import reads the name as a relative import of the code does, so the name stays used.
#[test]
fn doctest_relative_import_counts_as_a_read_of_the_name() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        ("app/__init__.py", ""),
        ("app/mod.py", "LIMIT = 3\n\n\n__all__ = []\n"),
        ("app/cli.py", "def run() -> None:\n    \"\"\"Run.\n\n    >>> from .mod import LIMIT\n    \"\"\"\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP404"]);
    assert_eq!(stdout, common::PASSED);
}
