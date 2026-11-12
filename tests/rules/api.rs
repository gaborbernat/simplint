use test_case::test_case;

use crate::common;

const APPLICATION: &str = "[tool.simplint]\nproject-kind = \"application\"\n";
const LIBRARY: &str = "[tool.simplint]\nproject-kind = \"library\"\n";

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
#[test_case(
    "module-variable-prefix", "_module_variable_prefix_empty_name.py"
    ; "module_variable_prefix_empty_or_keyword_name"
)]
#[test_case("module-variable-prefix", "module_variable_prefix_source_text.py" ; "module_variable_prefix_source_text")]
#[test_case("module-variable-prefix", "module_variable_prefix_type_alias.py" ; "module_variable_prefix_type_alias")]
#[test_case(
    "module-variable-prefix", "module_variable_prefix_function_alias.py"
    ; "module_variable_prefix_function_alias"
)]
#[test_case("module-variable-prefix", "_module_variable_prefix_block_local.py" ; "module_variable_prefix_block_local")]
#[test_case(
    "module-variable-prefix", "_module_variable_prefix_computed_setattr.py"
    ; "module_variable_prefix_computed_setattr"
)]
#[test_case(
    "module-variable-prefix", "module_variable_prefix_type_checking_stubs.py"
    ; "module_variable_prefix_type_checking_stubs"
)]
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
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from package import model\n\n\ndef \
                __getattr__(name: str):\n    return name\n\n\ndef __dir__():\n    return []\n\nVALUE = 1\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP407"]);
    assert_eq!(common::reports(&stdout, "SMP407").len(), 1, "{stdout}");
    assert!(!stdout.contains("tests/__init__.py"), "{stdout}");
    assert!(!stdout.contains("roots/test-example"), "{stdout}");
}

#[test]
fn init_module_content_allows_explicit_api_in_applications() {
    let root = common::project(&[
        ("pyproject.toml", APPLICATION),
        ("package/__init__.py", "from ._api import API\nfrom ._api import hidden\n\n__all__ = [\"API\"]\n"),
        ("package/_api.py", "API = 1\nhidden = 2\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP407"]);
    assert!(common::reports(&stdout, "SMP407").is_empty(), "{stdout}");
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

#[test]
fn missing_from_all_drops_the_noqa_it_leaves_unused() {
    let enc = "from os.path import join  # noqa: F401\nfrom os.path import (\n    sep,  # noqa: F401\
               \n    exists,  # noqa: E501\n)\nfrom os.path import isdir  # noqa\
               \nfrom os.path import isabs  # type: ignore\n\n\ndef run() -> int:\n    return 1\n";
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

// A `noqa` for F401 stays while another name its import binds is left out of the listing, since that name still needs
// it.
#[test]
fn missing_from_all_keeps_a_noqa_another_name_on_its_line_needs() {
    let root = common::project(&[
        ("enc.py", "from os.path import join, sep  # noqa: F401\n\n\ndef run() -> int:\n    return 1\n"),
        ("use.py", "from enc import join\n\nprint(join)\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "enc.py"),
        "from os.path import join, sep  # noqa: F401\n\n\ndef run() -> int:\n    return 1\n\n\n__all__ = [\n    \
            \"join\",\n]\n"
    );
}

// Ruff's F401 reports an import of a name that its own file never reads, which another module imports, whether or not
// the project selects F401. simplint keeps that report either way.
#[test]
fn missing_from_all_reports_unused_import_others_use_under_f401() {
    let case = common::RuffOverlap {
        settings: "",
        select: "SMP401",
        ruff_code: "F401",
        files: &[
            ("config.py", "from os import sep\n\n\ndef local() -> int:\n    return 1\n"),
            ("cli.py", "from config import sep, local\n\nprint(sep, local())\n"),
        ],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, "`sep` is imported by other modules"), [true, true]);
}

// With F401 selected, the fix lists the name and keeps the import, so Ruff's fix does not delete it. A `noqa` with its
// reason goes once the fix lists every name on its line.
#[test_case("from os import sep" ; "import_kept")]
#[test_case("from os import sep  # noqa: F401  # imported by cli.py" ; "reasoned_noqa_removed")]
fn missing_from_all_lists_unused_import_others_use_under_f401(import: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"F401\"]\n"),
        ("config.py", &format!("{import}\n\n\ndef local() -> int:\n    return 1\n")),
        ("cli.py", "from config import sep, local\n\nprint(sep, local())\n"),
    ]);
    let (code, _, _) = common::fix_unsafe(root.path(), "SMP401");
    assert_eq!(
        (code, common::read(root.path(), "config.py")),
        (
            0,
            "from os import sep\n\n\ndef local() -> int:\n    return 1\n\n\n__all__ = [\n    \"local\",\n    \"sep\",\n]\n"
                .to_owned()
        )
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
            "__all__ = []\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n\n\ndef beta() -> int:\n    return 2\
                \n",
        ),
        ("use.py", "from api import alpha, beta\n\nprint(alpha(), beta())\n"),
    ]);
    common::check(root.path(), &["--no-cache", "--fix", "--select", "SMP401"]);
    assert_eq!(
        common::read(root.path(), "api.py"),
        "__all__ = [\n    \"alpha\",\n    \"beta\",\n]\n\n\ndef alpha() -> int:  # noqa: SMP401\n    return 1\n\n\ndef \
            beta() -> int:\n    return 2\n"
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
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    class Locked(dict): ...\n\n\nclass Both: ...\n\
                \n\nif TYPE_CHECKING:\n    Both = int\n\n\ndef use(x: \"Locked\") -> None: ...\n\n\n__all__ = [\"use\"]\
                \n",
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
    let core = "class Client:\n    LIMIT = 3\n\n    def send(self) -> None:\n        pass\n\n\ndef helper() -> None:\
        \n    pass\n";
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
    let (core, other) = handler_and_dispatch(name, same_file);
    let root = common::project(&[
        ("app/core.py", core.as_str()),
        ("app/other.py", other.as_str()),
        ("app/main.py", "from app.core import _Handler\n\nprint(_Handler)\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(stdout.contains("`_Handler._handle_text`"), reported, "{stdout}");
}

fn handler_and_dispatch(name: &str, same_file: bool) -> (String, String) {
    let dispatch = format!("def dispatch(obj: object, kind: str) -> object:\n    return getattr(obj, {name})\n");
    let handler = "class _Handler:\n    def _handle_text(self) -> None:\n        pass\n";
    if same_file { (format!("{handler}\n\n{dispatch}"), String::new()) } else { (handler.to_owned(), dispatch) }
}

const RUF067_PREVIEW: &str = "[tool.ruff.lint]\npreview = true\nselect = [\"RUF067\"]\n";
const RUF067_STRICT: &str = "[tool.ruff.lint]\npreview = true\nselect = [\"RUF067\"]\n[tool.ruff.lint.ruff]\
    \nstrictly-empty-init-modules = true\n";
// RUF067 reports the statements of an `__init__.py` only in preview mode, and then the statements it does not allow, so
// the run leaves those to Ruff. The default allows imports, dunder assignments and `if TYPE_CHECKING` blocks, and a
// project that bans all code leaves nothing to the run but attribute docstrings.
#[test_case(RUF067_PREVIEW, common::CLEAN_MODULE, 0 ; "preview_assignment")]
#[test_case(RUF067_PREVIEW, "import os\n", 0 ; "preview_unused_import_left_to_f401")]
#[test_case(RUF067_PREVIEW, "__version__ = \"1\"\n", 1 ; "preview_dunder_allowed")]
#[test_case(RUF067_PREVIEW, "if TYPE_CHECKING:\n    pass\n", 0 ; "preview_type_checking_allowed")]
#[test_case(RUF067_PREVIEW, "def helper() -> None:\n    pass\n", 0 ; "preview_function")]
#[test_case(RUF067_PREVIEW, "X = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_attribute_docstring")]
#[test_case(RUF067_PREVIEW, "def helper() -> None:\n    pass\n\n\"stray\"\n", 0 ; "preview_stray_string")]
#[test_case(RUF067_STRICT, "import os\n", 0 ; "preview_strict_import")]
#[test_case(RUF067_STRICT, "X = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_strict_attribute_docstring")]
#[test_case("[tool.ruff.lint]\nselect = [\"RUF067\"]\n", common::CLEAN_MODULE, 0 ; "preview_off_skipped")]
#[test_case("[tool.ruff.lint]\nselect = []\n", "import os\nVALUE = 1\n", 0 ; "unselected_skipped")]
#[test_case(RUF067_PREVIEW, "__all__: list[str] = []\n", 0 ; "preview_dunder_annotated_allowed")]
#[test_case(RUF067_PREVIEW, "__all__ += [\"x\"]\n", 1 ; "preview_dunder_augmented_allowed")]
#[test_case(RUF067_PREVIEW, "counter += 1\n", 0 ; "preview_augmented_reported")]
#[test_case(
    RUF067_PREVIEW, "import typing\n\nif typing.TYPE_CHECKING:\n    pass\n", 1
    ; "preview_attribute_type_checking_allowed"
)]
#[test_case(RUF067_PREVIEW, "if callable(print):\n    pass\n", 0 ; "preview_other_if_reported")]
#[test_case(RUF067_STRICT, "X: int = 1\n\"\"\"Doc of X.\"\"\"\n", 1 ; "preview_strict_annotated_attribute_docstring")]
fn init_module_content_leaves_ruf067_to_ruff(config: &str, source: &str, reported: usize) {
    let root = common::project(&[("pyproject.toml", config), ("pkg/__init__.py", source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP407", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP407\"").count(), reported, "{stdout}");
}

// F401 reports an import of an `__init__.py` that nothing reads, so the rule leaves that statement to it. A statement
// keeps its report when one of its names is not one F401 reports.
#[test_case("", "from json import dumps\n", &[] ; "unused_import_left_to_f401")]
#[test_case("[tool.ruff.lint]\nselect = []\n", "from json import dumps\n", &[] ; "unused_import_skipped_without_f401")]
#[test_case("", "from json import dumps, loads\nprint(dumps)\n", &[1] ; "partly_used_statement_kept")]
#[test_case("", "from json import dumps\nprint(dumps)\n", &[1] ; "used_import_kept")]
#[test_case("", "from json import dumps\ndel dumps\n", &[1] ; "deleted_import_kept")]
#[test_case(
    "", "from json import dumps\n\n\ndef load(value: \"dumps\") -> None:\n    pass\n", &[1]
    ; "string_annotation_keeps_import"
)]
#[test_case("", "from json import dumps as dumps\n__all__ = [\"loads\"]\n", &[1] ; "explicit_alias_kept")]
#[test_case("", "import json.decoder\nimport json\nprint(json.decoder)\n", &[1] ; "shadowed_import_read_kept")]
#[test_case("", "import json.decoder\nimport json\n", &[] ; "shadowed_unread_imports_left_to_f401")]
#[test_case(
    "", "print(filter)\nfrom json import filter\nfrom json import filter\n", &[2]
    ; "builtin_shadow_ends_copies"
)]
#[test_case(
    "",
    "from json import dumps\nprint(dumps)\ntry:\n    pass\nexcept Exception as dumps:\n    pass\nfrom json import dumps\
        \n",
    &[1]
    ; "unbound_exception_ends_copies"
)]
#[test_case(
    "", "import ab\nimport ab.c\nimport ab.d\n__all__ = [\"a\" \"b\"]\n", &[1]
    ; "concatenated_all_marks_import"
)]
#[test_case("", "type Alias = ab.E\nimport ab\nimport ab.c\nimport ab.d\n", &[2] ; "shadowed_forward_alias_read_kept")]
#[test_case(
    "",
    "from __future__ import annotations\n\n\nclass C:\n    x: JSONDecoder = None\n\n\nfrom json import JSONDecoder\n",
    &[1, 8]
    ; "forward_class_annotation_kept"
)]
#[test_case("", "from json import dumps\n__all__ = [str()]\n", &[2] ; "dynamic_all_unused_import_left_to_f401")]
#[test_case("", "from json import dumps\n__all__ = []\n__all__.append(\"dumps\")\n", &[] ; "all_append_left_to_f401")]
#[test_case("", "from json import dumps\nprint(\"dumps\")\n", &[] ; "call_string_left_to_f401")]
#[test_case(
    "", "from json import dumps\nTABLE = {}\nVALUE = TABLE[\"dumps\"]\n", &[]
    ; "subscript_string_left_to_f401"
)]
#[test_case(
    "", "from typing import Optional\nfrom json import dumps\nx: Optional[\"dumps\"] = None\n", &[1, 2]
    ; "generic_subscript_string_kept"
)]
#[test_case(
    "[tool.ruff.lint.pyflakes]\nextend-generics = [\"mylib.Box\"]\n",
    "from json import JSONDecoder\nfrom mylib import Box\n\nAlias = Box[\"JSONDecoder\"]\n",
    &[1, 2] ;
    "extended_generic_subscript_string_kept"
)]
#[test_case(
    "", "from typing import Literal\nfrom json import dumps\nx: Literal[\"dumps\"] = \"dumps\"\n", &[1]
    ; "literal_string_left_to_f401"
)]
#[test_case(
    "", "from typing import Annotated\nfrom json import dumps\nx: Annotated[int, \"dumps\"] = 1\n", &[1]
    ; "annotated_metadata_left_to_f401"
)]
#[test_case(
    "", "from typing import Annotated\nfrom json import dumps\nx: Annotated[\"dumps\", 1] = 1\n", &[1, 2]
    ; "annotated_type_kept"
)]
#[test_case(
    "", "from typing import Annotated\nfrom json import dumps\nx: Annotated[\"dumps\"] = 1\n", &[1, 2]
    ; "annotated_single_type_kept"
)]
#[test_case("", "from json import dumps\n__all__ = list([\"dumps\"])\n", &[1, 2] ; "list_call_all_listed")]
#[test_case("", "from json import dumps\n__all__ = list(names)\n", &[2] ; "list_call_of_name_left_to_f401")]
#[test_case("", "from json import dumps\n__all__ = build()\n", &[2] ; "call_all_left_to_f401")]
#[test_case("", "from json import dumps\n__all__ = [\"a\"] + [\"dumps\"]\n", &[1, 2] ; "concatenated_list_all_listed")]
#[test_case("", "from json import dumps\n__all__ = [\"a\"] + [\"b\"]\n", &[2] ; "unlisted_concatenation_left_to_f401")]
#[test_case(
    "", "from json import dumps\n__all__ = [\"a\"] + [\"b\"] + [\"dumps\"]\n", &[1, 2]
    ; "chained_concatenation_listed"
)]
#[test_case("", "from json import dumps\n__all__ = list({\"dumps\"})\n", &[1, 2] ; "set_call_all_listed")]
#[test_case("", "from json import dumps\n__all__ = tuple((\"dumps\",))\n", &[1, 2] ; "tuple_call_all_listed")]
#[test_case(
    "", "from json import dumps\n__all__ = []\n__all__ += (value := [\"dumps\"])\n", &[1, 2, 3]
    ; "walrus_all_listed"
)]
#[test_case(
    "",
    "import sys\nif sys.version_info >= (3, 8):\n    import json.decoder\nimport json.encoder\nprint(json.decoder)\n",
    &[1, 4]
    ; "branch_shadow_read_kept"
)]
#[test_case(
    "",
    "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    import json.decoder\nimport json.encoder\
        \nprint(json.decoder)\n",
    &[1, 4]
    ; "type_checking_shadow_read_kept"
)]
#[test_case(
    "", "try:\n    import json.decoder\nexcept ImportError:\n    pass\nimport json.encoder\nprint(json.decoder)\n", &[5]
    ; "try_shadow_read_kept"
)]
#[test_case("", "type Alias = JSONDecoder\nfrom json import JSONDecoder\n", &[2] ; "type_alias_forward_read_kept")]
#[test_case(
    "", "def load[**P = [JSONDecoder]]() -> None:\n    pass\n\n\nfrom json import JSONDecoder\n", &[5]
    ; "param_spec_default_forward_read_kept"
)]
#[test_case(
    "", "def load[T: JSONDecoder]() -> None:\n    pass\n\n\nfrom json import JSONDecoder\n", &[5]
    ; "type_param_bound_forward_read_kept"
)]
#[test_case(
    "", "class Box[T = JSONDecoder]:\n    pass\n\n\nfrom json import JSONDecoder\n", &[5]
    ; "type_param_default_forward_read_kept"
)]
#[test_case("", "import os\nif os.sep:\n    del os\nimport os\n", &[1, 4] ; "conditional_del_reimport_kept")]
#[test_case("", "import os\ndel os\nimport os\n", &[1] ; "top_level_del_reimport_left_to_f401")]
#[test_case("", "from json import *\n", &[1] ; "star_import_kept")]
#[test_case(
    "", "from json import dumps\nfrom typing import cast\n\nvalue = cast(\"dumps\", 1)\n", &[1, 2]
    ; "cast_string_keeps_import"
)]
#[test_case(
    "", "from json import dumps\nfrom typing import TypeVar\n\nT = TypeVar(\"T\", bound=\"dumps\")\n", &[1, 2]
    ; "type_var_bound_keeps_import"
)]
#[test_case("", "from json import dumps\nNAME = \"dumps\"\n", &[] ; "plain_string_left_to_f401")]
#[test_case("", "\"\"\"Wraps dumps.\"\"\"\nfrom json import dumps\n", &[] ; "docstring_mention_left_to_f401")]
#[test_case("", "from json import dumps, loads\n__all__ = [\"dumps\"]\n", &[] ; "partly_listed_statement_left_to_f401")]
#[test_case(
    "[tool.ruff.lint.pyflakes]\nallowed-unused-imports = [\"json.dumps\"]\n",
    "from json import dumps\n",
    &[1] ;
    "allowed_unused_import_kept"
)]
#[test_case(
    "[tool.ruff.lint.isort]\nrequired-imports = [\"from json import dumps\"]\n",
    "from json import dumps\n",
    &[1] ;
    "required_import_kept"
)]
fn init_module_content_leaves_unused_imports_to_f401(config: &str, source: &str, rows: &[u64]) {
    let root = common::project(&[("pyproject.toml", config), ("pkg/__init__.py", source)]);
    let reports: Vec<serde_json::Value> =
        common::json(root.path(), &["check", "--no-cache", "--select", "SMP407", "--output-format", "json"]);
    let found: Vec<u64> = reports.iter().map(|report| report["location"]["row"].as_u64().expect("row")).collect();
    assert_eq!(found, rows);
}

// PYI046 and PYI049 report a private protocol or TypedDict that its file never reads, so the run leaves those to Ruff
// whether or not the project selects them. A class Ruff reads, or one that is not its last binding, stays with the run.
#[test_case(
    "from typing import Protocol\n\n\nclass _Proto(Protocol):\n    def run(self) -> None: ...\n", 0
    ; "protocol"
)]
#[test_case(
    "import typing_extensions\n\n\nclass _Proto(typing_extensions.Protocol[int]):\n    pass\n", 0
    ; "protocol_typing_extensions_subscript"
)]
#[test_case("from typing import TypedDict\n\n\nclass _Row(TypedDict):\n    a: int\n", 0 ; "typed_dict")]
#[test_case("from mylib import Protocol\n\n\nclass _Proto(Protocol):\n    pass\n", 1 ; "protocol_foreign")]
#[test_case(
    "from typing import Protocol\n\n\nclass _P(Protocol):\n    pass\n\n\nclass _P(Protocol):\n    pass\n", 1
    ; "protocol_rebound"
)]
#[test_case(
    "from __future__ import annotations\n\nfrom typing import Protocol\n\n\nclass _P(Protocol):\n    def run(self) -> \
        _P: ...\n",
    1
    ; "protocol_read_by_own_annotation"
)]
#[test_case(
    "from typing import Protocol\n\n\nclass _P(Protocol):\n    def run(self) -> '_P': ...\n", 1
    ; "protocol_string_annotation"
)]
#[test_case(
    "from typing import Protocol\n\n\nclass _P(Protocol):\n    \"\"\"Protocol of _P.\"\"\"\n", 0
    ; "protocol_docstring_mention"
)]
fn unused_definition_leaves_private_protocols_and_typed_dicts_to_ruff(source: &str, reported: usize) {
    let root = common::project(&[("module.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP409"]);
    assert_eq!(common::reports(&stdout, "SMP409").len(), reported, "{stdout}");
}

// A method a version switch defines inside an `if` in the class body is the class's method, so its calls count.
#[test]
fn unused_definition_counts_calls_of_a_method_under_an_if_block() {
    let root = common::project(&[
        (
            "app/core.py",
            "class Client:\n    if True:\n        def _send(self) -> None:\n            pass\n\n    def run(self) -> \
                None:\n        self._send()\n",
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
        "def setup(app: object) -> None:\n    print(app)\n\n\ndef pytest_configure(config: object) -> None:\n    \
            print(config)\n",
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
#[test_case(
    "docs/index.rst", ".. module:: app.core\n.. autofunction:: helper\n", false, true
    ; "directive_after_directive"
)]
#[test_case("docs/index.rst", ".. autoclass:: app.core.Client\n   :members:\n", true, false ; "autoclass_members")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n   :members:\n", false, false ; "automodule_members")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n   :members: helper\n", false, true ; "automodule_listed")]
#[test_case("docs/index.rst", ".. automodule:: app.core\n", true, true ; "automodule_alone")]
#[test_case(
    "docs/cli.rst", ".. sphinx_argparse_cli::\n   :module: app.core\n   :func: helper\n", false, true
    ; "argparse_cli"
)]
#[test_case("docs/cli.rst", ".. argparse::\n   :ref: app.core.helper\n   :prog: app\n", false, true ; "argparse_ref")]
#[test_case("docs/api.md", "Call {py:meth}`app.core.Client.send`.", true, false ; "myst_role")]
#[test_case("docs/api.md", "```{autofunction} app.core.helper\n```\n", false, true ; "myst_directive")]
#[test_case(
    "docs/api.md", ":::{autoclass} app.core.Client\n:members: send\n:::\n", true, false
    ; "myst_colon_directive"
)]
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

// Only a project with a `pyproject.toml` has documentation to read, and only a run with `unused-definition` on reads
// it.
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

// Renaming would change the name a reflective read returns, so the name stays.
#[test_case(
    "__all__ = []\n\n\nclass Item:\n    def label(self) -> str:\n        return type(self).__name__\n", "", "Item"
    ; "class_read_through_type"
)]
#[test_case(
    "__all__ = []\n\n\nclass Item:\n    def label(self) -> str:\n        return self.__class__.__qualname__\n",
    "",
    "Item"
    ; "class_read_through_class_attribute"
)]
#[test_case(
    "__all__ = []\n\n\nclass Item:\n    @classmethod\n    def label(cls) -> str:\n        return cls.__name__\n",
    "",
    "Item"
    ; "classmethod_reads_its_class"
)]
#[test_case(
    "__all__ = []\n\n\nclass Item:\n    pass\n",
    "def use(module: object) -> str:\n    return module.Item.__qualname__\n",
    "Item"
    ; "class_read_through_attribute_elsewhere"
)]
#[test_case(
    "__all__ = []\n\n\ndef helper() -> str:\n    return helper.__name__\n", "", "helper"
    ; "function_reads_its_own_name"
)]
#[test_case(
    "__all__ = []\n\n\ndef helper() -> str:\n    return helper.__qualname__\n", "", "helper"
    ; "function_reads_its_own_qualname"
)]
#[test_case(
    "__all__ = []\n\n\ndef helper() -> None:\n    pass\n",
    "def use(obj: object) -> str:\n    return obj.helper.__name__\n",
    "helper"
    ; "function_read_through_attribute_elsewhere"
)]
fn unexported_public_name_keeps_a_name_read_by_reflection(module: &str, consumer: &str, name: &str) {
    let root = common::project(&[("mod.py", module), ("consumer.py", consumer)]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert_eq!(
        (stdout.contains(&format!("SMP403 `{name}` is not in `__all__`")), stdout.contains("SMP403 [*]")),
        (true, false),
        "{stdout}"
    );
}

// A name read off a value that is neither a variable, an attribute nor a call names no class or function, and a
// function nobody reads by name is still renamed: only reflective reads withhold the rename.
#[test_case(
    "__all__ = []\n\n\nclass Item:\n    def label(self) -> str:\n        return self.items[0].__name__\n"
    ; "class_read_off_a_subscript"
)]
#[test_case("__all__ = []\n\n\ndef helper() -> None:\n    pass\n" ; "function_nobody_reads_by_name")]
fn unexported_public_name_renames_when_no_reflective_read_names_it(module: &str) {
    let root = common::project(&[("mod.py", module)]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert!(stdout.contains("SMP403 [*]"), "{stdout}");
}

// The name a script's `main` is looked up by in a test is a string of another file, which keeps `main` in place.
#[test]
fn unexported_public_name_checks_strings_of_other_files() {
    let root = common::project(&[
        ("tools.py", "__all__ = []\n\n\ndef main() -> None:\n    pass\n"),
        ("tests/test_tools.py", "ENTRY = \"main\"\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP403"]).1;
    assert_eq!(
        (stdout.contains("SMP403 `main` is not in `__all__`"), stdout.contains("SMP403 [*]")),
        (true, false),
        "{stdout}"
    );
}

// Ruff's COM812 reports a list whose closing bracket follows a line break with no trailing comma, so SMP405 leaves that
// list to it while Ruff runs the rule: `ruff format` joins a list that fits on one line, where COM812 no longer reports
// it. A list that closes on its last element's line is not COM812's.
#[test_case("__all__ = [\n    \"a\",\n    \"b\"\n]\n", COM812, false ; "closing_bracket_on_own_line")]
#[test_case("__all__ = [\n    \"a\",\n    \"b\"\n]\n", "", true ; "closing_bracket_on_own_line_without_com812")]
#[test_case("__all__ = [\n    \"a\"  # note\n]\n", COM812, false ; "comment_before_closing_bracket")]
#[test_case("__all__ = [\n    (\"a\")\n]\n", COM812, false ; "parenthesized_last_element")]
#[test_case("__all__ = [\n    \"a\",\n    \"b\"]\n", COM812, true ; "closing_bracket_after_last_element")]
#[test_case("__all__ = [\"a\", \"b\"]\n", COM812, true ; "single_line")]
#[test_case("__all__ = [\n    \"a\" \\\n]\n", COM812, true ; "line_continuation")]
fn all_format_leaves_com812_lists_to_ruff(source: &str, selected: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{selected}]\n")),
        ("module.py", source),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP405"]).1;
    assert_eq!(stdout.contains("has no trailing comma"), reported, "{stdout}");
}

const COM812: &str = "\"COM812\"";

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

const FUNCTION: &str = "def helper() -> None: ...\n\n\n__all__ = []\n";
const PRIVATE_FUNCTION: &str = "def _helper() -> None: ...\n\n\n__all__ = []\n";
const VARIABLE: &str = "limit = 3\n\n\n__all__ = []\n";
const PRIVATE_VARIABLE: &str = "_limit = 3\n\n\n__all__ = []\n";

// A public module's names are the API its users import, so the rename to private stays with the author, though the
// report stays. A private module and an application keep the rename.
#[test_case("unexported-public-name", FUNCTION, FUNCTION, "library", "core.py" ; "function_public_module_of_library")]
#[test_case(
    "unexported-public-name", FUNCTION, PRIVATE_FUNCTION, "library", "_core.py"
    ; "function_private_module_of_library"
)]
#[test_case(
    "unexported-public-name", FUNCTION, PRIVATE_FUNCTION, "application", "core.py"
    ; "function_public_module_of_application"
)]
#[test_case("module-variable-prefix", VARIABLE, VARIABLE, "library", "core.py" ; "variable_public_module_of_library")]
#[test_case(
    "module-variable-prefix", VARIABLE, PRIVATE_VARIABLE, "library", "_core.py"
    ; "variable_private_module_of_library"
)]
#[test_case(
    "module-variable-prefix", VARIABLE, PRIVATE_VARIABLE, "application", "core.py"
    ; "variable_public_module_of_application"
)]
fn private_rename_follows_the_module_role(rule: &str, source: &str, expected: &str, kind: &str, module: &str) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.simplint]\nproject-kind = \"{kind}\"\n")),
        (module, source),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", rule, "--fix", "--unsafe-fixes"]);
    assert_eq!(common::read(root.path(), module), expected, "{rule}: {stdout}");
}

// Listing a name in a public module of a library widens its API and what `import *` binds, so the report stays without
// a fix. A private module keeps the safe fix.
#[test_case("core.py", "def run() -> None: ...\n" ; "public_module_of_library_no_fix")]
#[test_case(
    "_core.py", "def run() -> None: ...\n\n\n__all__ = [\n    \"run\",\n]\n"
    ; "private_module_of_library_fixed"
)]
fn missing_from_all_offers_no_fix_in_public_library_modules(module: &str, expected: &str) {
    let root = common::project(&[
        ("pyproject.toml", LIBRARY),
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
    let root = common::project(&[("pyproject.toml", LIBRARY), ("lib/__init__.py", ""), ("lib/api.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP404,SMP102"]);
    assert_eq!(common::read(root.path(), "lib/api.py"), source);
    assert!(stdout.contains("SMP102") && stdout.contains("SMP404"), "{stdout}");
}

// An import of a name the run cannot resolve, such as a stdlib one, says nothing about the names of the project, so a
// block-local name of another module stays block-local and a module variable of the project stays reported.
#[test]
fn unresolved_import_does_not_change_the_module_variables_of_other_modules() {
    let root = common::project(&[
        ("pyproject.toml", APPLICATION),
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
        ("pyproject.toml", APPLICATION),
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
        ("pyproject.toml", APPLICATION),
        ("app/__init__.py", ""),
        ("app/mod.py", "LIMIT = 3\n\n\ndef helper() -> int:\n    return LIMIT\n\n\n__all__ = []\n"),
        (
            "app/cli.py",
            "def run() -> None:\n    \"\"\"Run.\n\n    >>> from app.mod import LIMIT, helper\n    >>> helper() == LIMIT\
                \n    True\n    \"\"\"\n",
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
        ("pyproject.toml", APPLICATION),
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
        ("pyproject.toml", LIBRARY),
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
        ("pyproject.toml", APPLICATION),
        ("app/__init__.py", ""),
        ("app/mod.py", "LIMIT = 3\n\n\n__all__ = []\n"),
        ("app/cli.py", "def run() -> None:\n    \"\"\"Run.\n\n    >>> from .mod import LIMIT\n    \"\"\"\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP404"]);
    assert_eq!(stdout, common::PASSED);
}
