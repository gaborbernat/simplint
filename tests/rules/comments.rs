use test_case::test_case;

use crate::common;

#[test_case("section-comment", "section_comment.py" ; "section_comment")]
#[test_case("long-paragraph", "long_paragraph.py" ; "long_paragraph")]
#[test_case("suppression-without-reason", "suppression_without_reason.py" ; "suppression_without_reason")]
#[test_case("blanket-ignore", "blanket_ignore.py" ; "blanket_ignore")]
#[test_case("restating-comment", "restating_comment.py" ; "restating_comment")]
#[test_case("string-statement-comment", "string_statement_comment.py" ; "string_statement_comment")]
#[test_case("long-comment-line", "long_comment_line.py" ; "long_comment_line")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

// Ruff's PGH003, PGH004 and RUF103 report a blanket directive, which SMP004 leaves to them while the project's Ruff
// configuration runs them.
#[test_case("import os  # noqa\n", "PGH004", "`# noqa` suppresses every error" ; "noqa")]
#[test_case("import os  # type: ignore\n", "PGH003", "`# type: ignore` suppresses every error" ; "type_ignore")]
#[test_case("# ruff: noqa\nimport os\n", "PGH004", "`# ruff: noqa` suppresses every error in the file" ; "file_noqa")]
#[test_case("VALUE = 1  # ruff: ignore\n", "RUF103", "`# ruff: ignore` suppresses every error on the line" ; "ruff_ignore")]
#[test_case("VALUE = 1  # ruff: ignore[]\n", "RUF103", "`# ruff: ignore` suppresses every error" ; "ruff_ignore_empty")]
#[test_case("VALUE = 1  # ruff: ignore[E501\n", "RUF103", "`# ruff: ignore` suppresses every error" ; "ruff_ignore_unclosed")]
#[test_case("# ruff: file-ignore\nimport os\n", "RUF103", "`# ruff: file-ignore` suppresses every error in the file" ; "ruff_file_ignore")]
#[test_case("VALUE = 1  # ruff: ignore [ ]\n", "RUF103", "`# ruff: ignore` suppresses every error" ; "ruff_ignore_spaced_empty")]
fn blanket_ignore_follows_ruff_blanket_rules(source: &str, code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP004"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}
