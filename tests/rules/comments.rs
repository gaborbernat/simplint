use test_case::test_case;

use crate::common;

#[test_case("section-comment", "section_comment.py" ; "section_comment")]
#[test_case("long-paragraph", "long_paragraph.py" ; "long_paragraph")]
#[test_case("suppression-without-reason", "suppression_without_reason.py" ; "suppression_without_reason")]
#[test_case("suppression-without-reason", "suppression_without_reason_above.py" ; "suppression_without_reason_above")]
#[test_case("blanket-ignore", "blanket_ignore.py" ; "blanket_ignore")]
#[test_case("restating-comment", "restating_comment.py" ; "restating_comment")]
#[test_case("string-statement-comment", "string_statement_comment.py" ; "string_statement_comment")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test]
fn numeric_ignore_fixes_known_codes_and_keeps_the_reason() {
    let source = "x = 1  # simplint: ignore[SMP101, imprecise-annotation, SMP999] # external name\n";
    let root = common::project(&[("module.py", source)]);
    let (code, stdout, _) = common::check(root.path(), &["--select", "SMP008"]);
    assert_eq!((code, stdout.contains("Numeric simplint ignore; use rule names")), (1, true));
    common::check(root.path(), &["--select", "SMP008", "--fix"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        "x = 1  # simplint: ignore[single-letter-name, imprecise-annotation, SMP999] # external name\n"
    );
}

#[test]
fn named_ignore_needs_no_migration() {
    let root = common::project(&[("module.py", "x = 1  # simplint: ignore[single-letter-name] # external name\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP008"]).1, common::PASSED);
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
    let case = common::RuffOverlap { settings: "", select: "SMP004", ruff_code: code, files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, fragment), [true, false]);
}
