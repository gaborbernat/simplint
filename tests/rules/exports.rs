use test_case::test_case;

use crate::common;

#[test_case("all-not-last", "all_not_last.py" ; "all_not_last")]
#[test_case("all-not-last", "all_not_last_eof.py" ; "all_not_last_at_eof")]
#[test_case("all-not-last", "all_not_last_reversed.py" ; "all_not_last_reversed_main_guard")]
#[test_case("all-not-last", "all_not_last_nested.py" ; "all_not_last_extended_in_block")]
#[test_case("all-not-last", "all_not_last_negated.py" ; "all_not_last_negated_guard")]
#[test_case("all-not-last", "all_not_last_nonl.py" ; "all_not_last_without_trailing_newline")]
#[test_case("all-not-last", "all_not_last_ann.py" ; "all_not_last_annotated_and_augmented")]
#[test_case("all-not-last", "all_not_last_fenced.py" ; "all_not_last_keeps_its_fmt_off_fence")]
#[test_case("all-not-last", "multiline_module_docstring_ok.py" ; "all_not_last_without_all")]
#[test_case("all-format", "all_format.py" ; "all_format")]
#[test_case("all-format", "all_format_tuple.py" ; "all_format_tuple")]
#[test_case("all-format", "all_format_bare.py" ; "all_format_bare_tuple")]
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
#[test_case("missing-from-all", "missing_from_all_scripts.py" ; "missing_from_all_entry_point")]
#[test_case("missing-from-all", "tests/test_module_docstring.py" ; "missing_from_all_test_module")]
#[test_case("missing-from-all", "package/__init__.py" ; "missing_from_all_init_module")]
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
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP115"]);
    assert_eq!(
        (common::read(root.path(), "listed.py"), common::read(root.path(), "paired.py")),
        (
            "def load() -> None: ...\n\n\n__all__ = [\"load\"]\n".to_owned(),
            "def load() -> None: ...\n\n\ndef save() -> None: ...\n\n\n__all__ = [\"load\", \"save\"]\n".to_owned()
        )
    );
}
