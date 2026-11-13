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
    common::check(root.path(), &["--select", "SMP008", "--fix"]);
    assert_eq!(
        (code, stdout.contains("Numeric simplint ignore; use rule names"), common::read(root.path(), "module.py")),
        (
            1,
            true,
            "x = 1  # simplint: ignore[single-letter-name, imprecise-annotation, SMP999] # external name\n".to_owned()
        )
    );
}

#[test_case("x = 1  # simplint: ignore[single-letter-name] # external name\n" ; "named")]
#[test_case("x = 1  # simplint: disable[SMP101]\n" ; "other_action")]
#[test_case("x = 1  # simplint: ignore[SMP101\n" ; "unclosed_selector_list")]
fn ignore_without_numeric_selectors_needs_no_migration(source: &str) {
    let root = common::project(&[("module.py", source)]);
    common::assert_clean(root.path(), "SMP008");
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
    common::assert_clean(root.path(), "SMP004");
}

#[test]
fn unused_ignore_reports_and_fixes_each_selector() {
    let root = common::project(&[(
        "module.py",
        "x = 1  # simplint: ignore[SMP101, SMP602]  # needed for x\nvalue = 2  # simplint: ignore[SMP602]  # stale\n",
    )]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP101,SMP009"]);
    let fixed = common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0;
    assert_eq!(
        (
            status,
            output.matches(" SMP009 ").count(),
            output.contains("2 unused simplint ignore selectors"),
            fixed,
            common::read(root.path(), "module.py"),
        ),
        (1, 1, true, 0, "x = 1  # simplint: ignore[SMP101]  # needed for x\nvalue = 2  # stale\n".to_owned())
    );
}

#[test_case(
    "[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 2\n" ;
    "local_limit_blocks_report"
)]
#[test_case("[tool.ruff.lint]\nselect = []\n" ; "comment_keeps_report_silent")]
fn unused_ignore_rechecks_comment_sensitive_rules(config: &str) {
    let source = "_MESSAGE = '''hello\nworld'''  # simplint: ignore[SMP202]  # moving adds a local\n\n\ndef read() -> \
        str:\n    first = 1\n    second = 2\n    return _MESSAGE + str(first + second)\n";
    let root = common::project(&[("pyproject.toml", config), ("module.py", source)]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP202,SMP009"]);
    common::check(root.path(), &["--select", "SMP202,SMP009", "--fix"]);
    assert_eq!(
        (
            status,
            output.contains("unused simplint ignore selector"),
            common::read(root.path(), "module.py").contains("simplint: ignore"),
        ),
        (1, true, false),
        "{output}"
    );
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

// An indentation as wide as the comment limit leaves no room for the text, so the string is reported without a fix.
#[test]
fn string_statement_comment_without_room_has_no_fix() {
    let source = "def run() -> None:\n    start()\n    \"\"\"Wait.\"\"\"\n    wait()\n";
    let root = common::project(&[
        ("pyproject.toml", "[tool.ruff.lint.pycodestyle]\nmax-doc-length = 4\n"),
        ("module.py", source),
    ]);
    let (status, output, _) = common::check(root.path(), &["--no-cache", "--select", "SMP006", "--fix"]);
    assert_eq!(
        (status, output.contains("SMP006"), common::read(root.path(), "module.py")),
        (1, true, source.to_owned()),
        "{output}"
    );
}

#[test]
fn unused_ignore_keeps_required_selectors() {
    let source = "x = 1  # simplint: ignore[SMP101]  # short fixture name\n";
    let root = common::project(&[("module.py", source)]);
    assert_eq!(
        (common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]), common::read(root.path(), "module.py")),
        ((0, common::PASSED.to_owned(), String::new()), source.to_owned())
    );
}

#[test]
fn unused_ignore_drops_standalone_directive_without_hiding_later_reports() {
    let root = common::project(&[("module.py", "# simplint: ignore[SMP602]  # stale\nx = 1\n")]);
    let fixed = common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0;
    assert_eq!(
        (fixed, common::read(root.path(), "module.py"), common::check(root.path(), &["--select", "SMP101"]).0),
        (1, "# stale\nx = 1\n".to_owned(), 1)
    );
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
#[test_case("value = 1  # simplint: ignore[SMP101]\r\nnext_value = 2\r\n", "value = 1\r\nnext_value = 2\r\n" ; "crlf")]
#[test_case(
    "x = 1  # noqa: E501\nvalue = 2  # simplint: ignore[SMP101]\n",
    "x = 1  # noqa: E501\nvalue = 2\n" ;
    "report_under_another_noqa"
)]
fn unused_ignore_removes_only_the_directive_and_its_reason(source: &str, expected: &str) {
    let root = common::project(&[("module.py", source)]);
    common::check(root.path(), &["--select", "SMP101,SMP009", "--fix"]);
    assert_eq!(common::read(root.path(), "module.py"), expected);
}

#[test_case(
    "SMP101,SMP009",
    "x = 1  # noqa: SMP101  # simplint: ignore[SMP101]\n",
    "x = 1  # noqa: SMP101\n" ;
    "selector_already_covered_by_noqa"
)]
#[test_case("SMP008,SMP009", "value = 1  # simplint: ignore[SMP602]\n", "value = 1\n" ; "after_numeric_ignore_fix")]
#[test_case(
    "SMP009",
    "name = 'é'  # simplint: ignore[SMP101, SMP602]\n",
    "name = 'é'\n" ;
    "multiple_selectors_after_unicode"
)]
#[test_case(
    "SMP101,SMP602,SMP009",
    "from typing import Any\n\n\ndef x(value: list[Any]) -> None:  \
     # simplint: ignore[SMP101,SMP603,SMP602]\n    pass\n",
    "from typing import Any\n\n\ndef x(value: list[Any]) -> None:  # simplint: ignore[SMP101, SMP602]\n    pass\n" ;
    "separators_around_a_stale_selector"
)]
#[test_case(
    "SMP101,SMP009",
    "# ruff: noqa: SMP101\nx = 1  # simplint: ignore[SMP101]\n",
    "# ruff: noqa: SMP101\nx = 1\n" ;
    "under_file_level_suppression"
)]
fn unused_ignore_fixes_every_stale_selector(select: &str, source: &str, expected: &str) {
    let root = common::project(&[("module.py", source)]);
    assert_eq!(
        (common::check(root.path(), &["--select", select, "--fix"]).0, common::read(root.path(), "module.py")),
        (0, expected.to_owned())
    );
}

// The numeric selector the removal shifts is reported where it was, so the report is no new one.
#[test]
fn unused_ignore_keeps_the_numeric_report_of_a_selector_it_shifts() {
    let root = common::project(&[("module.py", "x = 1  # simplint: ignore[SMP602, SMP101]\n")]);
    assert_eq!(
        (
            common::check(root.path(), &["--select", "SMP008,SMP009,SMP101", "--unfixable", "SMP008", "--fix"]).0,
            common::read(root.path(), "module.py"),
        ),
        (1, "x = 1  # simplint: ignore[SMP101]\n".to_owned())
    );
}

#[test]
fn unused_ignore_honors_unfixable() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = common::project(&[("module.py", source)]);
    assert_eq!(
        (
            common::check(root.path(), &["--select", "SMP009", "--unfixable", "SMP009", "--fix"]).0,
            common::read(root.path(), "module.py"),
        ),
        (1, source.to_owned())
    );
}

#[test]
fn unused_ignore_is_opt_in() {
    let root = common::project(&[("module.py", "value = 1  # simplint: ignore[SMP101]\n")]);
    let default = common::check(root.path(), &[]).1.contains("SMP009");
    let selected = common::check(root.path(), &["--extend-select", "SMP009"]).1.contains("SMP009");
    common::check(root.path(), &["--extend-select", "SMP009", "--fix"]);
    assert_eq!((default, selected, common::read(root.path(), "module.py")), (false, true, "value = 1\n".to_owned()));
}

#[test]
fn unused_ignore_fix_appears_in_diff() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = common::project(&[("module.py", source)]);
    let (status, output, _) = common::check(root.path(), &["--select", "SMP009", "--diff"]);
    assert_eq!(
        (status, output.contains("+value = 1\n"), common::read(root.path(), "module.py")),
        (1, true, source.to_owned()),
        "{output}"
    );
}

#[test]
fn unused_ignore_fixes_many_comments_in_one_pass() {
    let source = (0..120).fold(String::new(), |mut source, index| {
        writeln!(source, "value_{index} = {index}  # simplint: ignore[SMP101]").expect("write to string");
        source
    });
    let root = common::project(&[("module.py", &source)]);
    assert_eq!(
        (common::check(root.path(), &["--select", "SMP009", "--fix"]).0, common::read(root.path(), "module.py")),
        (0, source.replace("  # simplint: ignore[SMP101]", ""))
    );
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
