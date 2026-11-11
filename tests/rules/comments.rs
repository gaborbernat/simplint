use std::fmt::Write as _;

use test_case::test_case;

use crate::common;

#[test_case("section-comment", "section_comment.py", None ; "section_comment")]
#[test_case("long-paragraph", "long_paragraph.py", None ; "long_paragraph")]
#[test_case("suppression-without-reason", "suppression_without_reason.py", None ; "suppression_without_reason")]
#[test_case(
    "suppression-without-reason", "suppression_without_reason_above.py", None ; "suppression_without_reason_above"
)]
#[test_case("blanket-ignore", "blanket_ignore.py", None ; "blanket_ignore")]
#[test_case("restating-comment", "restating_comment.py", None ; "restating_comment")]
#[test_case("string-statement-comment", "string_statement_comment.py", None ; "string_statement_comment")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

// Ruff's E266 and E265 report a plain comment of prose after its hashes, with the text SMP001's fix keeps, so SMP001
// leaves such a comment to them whether or not the project selects them. A decorated comment or a short heading is not
// the same text as Ruff's fix, so SMP001 keeps it.
#[test_case("E266", "## Remove this return to update the cron job\nVALUE = 1\n", [false, false] ; "spaced_hashes")]
#[test_case("E265", "##Remove this return to update the cron job\nVALUE = 1\n", [false, false] ; "unspaced_hashes")]
#[test_case("E265", "##\tRemove this return to update the cron job\nVALUE = 1\n", [false, false] ; "tab_after_hashes")]
#[test_case(
    "E266",
    "VALUES = [\n    1,\n    ## Remove this return to update the cron job\n    2,\n]\n",
    [false, false] ;
    "inside_brackets"
)]
#[test_case(
    "E266", "## --- Remove this return to update the cron job ---\nVALUE = 1\n", [true, true] ; "decorated_prose"
)]
#[test_case(
    "E266", "#### Remove this return to update the cron job ####\nVALUE = 1\n", [true, true] ; "decorated_hashes"
)]
#[test_case("E266", "## Setup\nVALUE = 1\n", [true, true] ; "short_heading")]
fn section_comment_prose_is_left_to_ruff_e265_and_e266(ruff_code: &str, source: &str, reported: [bool; 2]) {
    let case = common::RuffOverlap { settings: "", select: "SMP001", ruff_code, files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, "SMP001"), reported);
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

// Ruff's PGH003, PGH004 and RUF103 report a blanket directive, which SMP004 leaves to them whether or not the project
// selects them. A `type: ignore` followed by a reason is no blanket for PGH003, so SMP004 keeps it.
#[test_case("import os  # noqa\n", "PGH004", "`# noqa` suppresses every error", [false, false] ; "noqa")]
#[test_case(
    "import os  # type: ignore\n", "PGH003", "`# type: ignore` suppresses every error", [false, false] ; "type_ignore"
)]
#[test_case(
    "# ruff: noqa\nimport os\n",
    "PGH004",
    "`# ruff: noqa` suppresses every error in the file",
    [false, false] ;
    "file_noqa"
)]
#[test_case(
    "VALUE = 1  # ruff: ignore\n",
    "RUF103",
    "`# ruff: ignore` suppresses every error on the line",
    [false, false] ;
    "ruff_ignore"
)]
#[test_case(
    "VALUE = 1  # ruff: ignore[]\n",
    "RUF103",
    "`# ruff: ignore` suppresses every error",
    [false, false] ;
    "ruff_ignore_empty"
)]
#[test_case(
    "VALUE = 1  # ruff: ignore[E501\n",
    "RUF103",
    "`# ruff: ignore` suppresses every error",
    [false, false] ;
    "ruff_ignore_unclosed"
)]
#[test_case(
    "# ruff: file-ignore\nimport os\n",
    "RUF103",
    "`# ruff: file-ignore` suppresses every error in the file",
    [false, false] ;
    "ruff_file_ignore"
)]
#[test_case(
    "VALUE = 1  # ruff: ignore [ ]\n",
    "RUF103",
    "`# ruff: ignore` suppresses every error",
    [false, false] ;
    "ruff_ignore_spaced_empty"
)]
#[test_case(
    "import os  # type: ignore  reason\n",
    "PGH003",
    "`# type: ignore` suppresses every error",
    [true, true] ;
    "type_ignore_with_reason"
)]
fn blanket_ignore_is_left_to_ruff_blanket_rules(source: &str, code: &str, fragment: &str, reported: [bool; 2]) {
    let case = common::RuffOverlap { settings: "", select: "SMP004", ruff_code: code, files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, fragment), reported);
}

#[test_case("A = 1  # type: ignore  [attr-defined]\n" ; "type_ignore_spaced_codes")]
#[test_case("A = 1  # pyright: ignore  [reportAttributeAccessIssue]\n" ; "pyright_ignore_spaced_codes")]
#[test_case("A = 1  # type: ignore [attr-defined]\n" ; "type_ignore_codes")]
fn named_type_ignore_after_spaces_is_not_blanket(source: &str) {
    let root = common::project(&[("module.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP004"]).1, common::PASSED);
}

#[test]
fn unused_ignore_reports_and_fixes_each_selector() {
    let root = common::project(&[(
        "module.py",
        "x = 1  # simplint: ignore[SMP101, SMP602]  # needed for x\nvalue = 2  # simplint: ignore[SMP602]  # stale\n",
    )]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP101,SMP009"]);
    assert_eq!(status, 1);
    assert_eq!(output.matches(" SMP009 ").count(), 1);
    assert!(output.contains("2 unused simplint ignore selectors"), "{output}");
    assert_eq!(common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(
        common::read(root.path(), "module.py"),
        "x = 1  # simplint: ignore[SMP101]  # needed for x\nvalue = 2  # stale\n"
    );
}

#[test_case(
    "[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 2\n",
    true ;
    "local_limit_blocks_report"
)]
#[test_case("[tool.ruff.lint]\nselect = []\n", true ; "comment_keeps_report_silent")]
fn unused_ignore_rechecks_comment_sensitive_rules(config: &str, removed: bool) {
    let source = "_MESSAGE = '''hello\nworld'''  # simplint: ignore[SMP202]  # moving adds a local\n\n\ndef read() -> s\
        tr:\n    first = 1\n    second = 2\n    return _MESSAGE + str(first + second)\n";
    let root = common::project(&[("pyproject.toml", config), ("module.py", source)]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP202,SMP009"]);
    assert_eq!(output.contains("unused simplint ignore selector"), removed, "{output}");
    assert_eq!(status == 1, removed);
    common::check(root.path(), &["--select", "SMP202,SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py").contains("simplint: ignore"), !removed);
}

#[test]
fn unused_ignore_removes_stale_selectors_beside_one_a_comment_needs() {
    let needed = "_MESSAGE = '''hello\nworld'''  # simplint: ignore[SMP202]\n\n\ndef read() -> str:\n    first = 1\
        \n    second = 2\n    return _MESSAGE + str(first + second)\n";
    let stale = "\n\nvalue = 2  # simplint: ignore[SMP602]\nother = 3  # simplint: ignore[SMP603]\n";
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = []\n"),
        ("module.py", &format!("{needed}{stale}")),
    ]);
    common::check(root.path(), &["--select", "SMP202,SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py"), format!("{needed}\n\nvalue = 2\nother = 3\n"));
}

#[test]
fn unused_ignore_keeps_required_selectors() {
    let source = "x = 1  # simplint: ignore[SMP101]  # short fixture name\n";
    let root = common::project(&[("module.py", source)]);
    assert_eq!(
        common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]),
        (0, common::PASSED.to_owned(), String::new())
    );
    assert_eq!(common::read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_preserves_crlf() {
    let root = common::project(&[("module.py", "value = 1  # simplint: ignore[SMP101]\r\nnext_value = 2\r\n")]);
    common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py"), "value = 1\r\nnext_value = 2\r\n");
}

#[test]
fn unused_ignore_drops_standalone_directive_without_hiding_later_reports() {
    let root = common::project(&[("module.py", "# simplint: ignore[SMP602]  # stale\nx = 1\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 1);
    assert_eq!(common::read(root.path(), "module.py"), "# stale\nx = 1\n");
    assert_eq!(common::check(root.path(), &["--select", "SMP101"]).0, 1);
}

#[test]
fn unused_ignore_removes_a_selector_already_covered_by_noqa() {
    let root = common::project(&[("module.py", "x = 1  # noqa: SMP101  # simplint: ignore[SMP101]\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(common::read(root.path(), "module.py"), "x = 1  # noqa: SMP101\n");
}

#[test_case(
    "value = 1  # simplint: ignore[SMP101]  # wire order\n",
    "value = 1  # wire order\n" ;
    "trailing_directive_keeps_later_comment"
)]
#[test_case(
    "def f():\n    # simplint: ignore[SMP101]  # wire order\n    return 1\n",
    "def f():\n    # wire order\n    return 1\n" ;
    "own_line_directive_keeps_later_comment"
)]
#[test_case(
    "value = 1  # simplint: ignore[SMP101]# noqa: E501\n",
    "value = 1  # noqa: E501\n" ;
    "directive_joined_to_later_comment"
)]
#[test_case(
    "x = 1  # simplint: ignore[SMP602] # noqa: SMP101\n",
    "x = 1  # noqa: SMP101\n" ;
    "stale_directive_keeps_noqa_of_its_line"
)]
#[test_case(
    "def f():\n    # simplint: ignore[SMP101]\n    return 1\n",
    "def f():\n    return 1\n" ;
    "standalone_directive_leaves_no_line"
)]
fn unused_ignore_removes_only_the_directive_and_its_reason(source: &str, expected: &str) {
    let root = common::project(&[("module.py", source)]);
    common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py"), expected);
}

#[test]
fn unused_ignore_honors_unfixable() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = common::project(&[("module.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP009", "--unfixable", "SMP009", "--fix"]).0, 1);
    assert_eq!(common::read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_is_opt_in() {
    let root = common::project(&[("module.py", "value = 1  # simplint: ignore[SMP101]\n")]);
    assert!(!common::check(root.path(), &[]).1.contains("SMP009"));
    assert!(common::check(root.path(), &["--extend-select", "SMP009"]).1.contains("SMP009"));
    common::check(root.path(), &["--extend-select", "SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py"), "value = 1\n");
}

#[test]
fn unused_ignore_fix_appears_in_diff() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = common::project(&[("module.py", source)]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP009", "--diff"]);
    assert_eq!(status, 1);
    assert!(output.contains("+value = 1\n"), "{output}");
    assert_eq!(common::read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_follows_numeric_ignore_fix() {
    let root = common::project(&[("module.py", "value = 1  # simplint: ignore[SMP602]\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP008,SMP009", "--fix"]).0, 0);
    assert_eq!(common::read(root.path(), "module.py"), "value = 1\n");
}

#[test]
fn unused_ignore_fixes_many_comments_in_one_pass() {
    let source = (0..120).fold(String::new(), |mut source, index| {
        writeln!(source, "value_{index} = {index}  # simplint: ignore[SMP101]").expect("write to string");
        source
    });
    let root = common::project(&[("module.py", &source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP009", "--fix"]).0, 0);
    assert_eq!(common::read(root.path(), "module.py"), source.replace("  # simplint: ignore[SMP101]", ""));
}

#[test]
fn unused_ignore_removes_multiple_selectors_after_unicode() {
    let root = common::project(&[("module.py", "name = 'é'  # simplint: ignore[SMP101, SMP602]\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP009", "--fix"]).0, 0);
    assert_eq!(common::read(root.path(), "module.py"), "name = 'é'\n");
}

#[test]
fn unused_ignore_rewrites_separators_around_a_stale_selector() {
    let root = common::project(&[(
        "module.py",
        "from typing import Any\n\n\ndef x(value: list[Any]) -> None:  \
         # simplint: ignore[SMP101,SMP603,SMP602]\n    pass\n",
    )]);
    assert_eq!(common::check(root.path(), &["--select", "SMP101,SMP602,SMP009", "--fix"]).0, 0);
    assert_eq!(
        common::read(root.path(), "module.py"),
        "from typing import Any\n\n\ndef x(value: list[Any]) -> None:  # simplint: ignore[SMP101, SMP602]\n    pass\n"
    );
}

#[test]
fn unused_ignore_handles_file_level_suppression() {
    let root = common::project(&[("module.py", "# ruff: noqa: SMP101\nx = 1  # simplint: ignore[SMP101]\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(common::read(root.path(), "module.py"), "# ruff: noqa: SMP101\nx = 1\n");
}

// SMP008 reports on the codes each removal edits, which leaves the stale numeric selectors no less unused.
#[test]
fn unused_ignore_reports_numeric_selectors_next_to_numeric_ignore() {
    let root = common::project(&[("module.py", "x = 1  # simplint: ignore[SMP602,SMP101]\n")]);
    assert_eq!(
        common::check(root.path(), &["--select", "SMP008,SMP009"]),
        (
            1,
            "module.py:1:8: SMP009 [*] 2 unused simplint ignore selectors\nmodule.py:1:27: SMP008 [*] Numeric simplint \
             ignore; use rule names\nFound 2 errors.\n[*] 2 fixable with `--fix`.\n"
                .to_owned(),
            String::new()
        )
    );
}
