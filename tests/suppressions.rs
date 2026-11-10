mod common;

use std::fmt::Write as _;

use test_case::test_case;

use common::{check, project, read};

#[test]
fn check_noqa_suppresses_listed_code() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP101\ny = 2  # noqa\nz = 3  # noqa: E501\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101"]).1,
        "module.py:3:1: SMP101 Single-letter name `z`; only comprehensions may use one\nFound 1 \
         error.\n"
    );
}

#[test]
fn check_ignore_noqa_reports_suppressed_code() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP101\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101", "--ignore-noqa"]).0, 1);
}

#[test_case("x = 1  # simplint: ignore[SMP101]\n" ; "ignore")]
#[test_case("x = 1  # simplint: ignore[single-letter-name]\n" ; "long_name")]
#[test_case("x = 1  # simplint : ignore[SMP101]\n" ; "ignore_space_before_colon")]
#[test_case("x = 1  # simplint:  ignore[SMP101]\n" ; "ignore_spaces_after_colon")]
fn simplint_ignore_suppresses_with_any_space_around_its_colon(source: &str) {
    let root = project(&[("module.py", source)]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

#[test_case("x = 1  # NoQA: SMP101\n", 0 ; "keyword_in_any_case")]
#[test_case("x = 1  # noqa : SMP101\n", 0 ; "space_before_colon")]
#[test_case("x = 1  # noqa:SMP601,SMP101\n", 0 ; "codes_after_a_comma")]
#[test_case("x = 1  # noqa: SMP601 SMP101 kept for plugins\n", 0 ; "codes_then_reason")]
#[test_case("x = 1  # noqa: SMP601 kept for SMP101\n", 1 ; "code_after_reason")]
#[test_case("x = 1  # noqa:SMP601SMP101\n", 0 ; "joined_codes")]
#[test_case("x = 1  # noqa:SMP101# why\n", 0 ; "code_then_comment")]
#[test_case("x = 1  # noqa: SMP101x\n", 1 ; "code_running_into_a_letter")]
#[test_case("x = 1  # noqa: SMP601SMPx\n", 1 ; "joined_code_without_digits")]
#[test_case("x = 1  # noqa:\n", 1 ; "colon_without_codes")]
#[test_case("x = 1  # noqa # noqa: SMP601\n", 0 ; "first_directive_blanket")]
#[test_case("x = 1  # noqax # noqa: SMP101\n", 1 ; "first_directive_rejected")]
#[test_case("x = 1  # why # noqa: SMP101\n", 0 ; "after_another_comment")]
fn noqa_reads_codes_as_ruff_does(source: &str, errors: i32) {
    let root = project(&[("module.py", source)]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, errors);
}

#[test]
fn noqa_after_a_multiline_string_suppresses_its_start() {
    let root = project(&[("module.py", "x = \"\"\"a\nb\"\"\"  # noqa: SMP101\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 0);
}

#[test_case("# ruff: noqa: SMP101\nx = 1\n", 0 ; "ruff_noqa_codes")]
#[test_case("# ruff: noqa: SMP601\nx = 1\n", 1 ; "ruff_noqa_other_codes")]
#[test_case("#ruff:NOQA:SMP601,SMP101\nx = 1\n", 0 ; "ruff_noqa_without_spaces")]
#[test_case("# Ruff: noqa: SMP101\nx = 1\n", 1 ; "ruff_noqa_capitalized")]
#[test_case("# ruff: noqa\nx = 1\n", 0 ; "ruff_noqa_blanket")]
#[test_case("# ruff: noqa# why\nx = 1\n", 0 ; "ruff_noqa_blanket_then_comment")]
#[test_case("# ruff: noqa because\nx = 1\n", 0 ; "ruff_noqa_blanket_then_text")]
#[test_case("# ruff: noqa : SMP101\nx = 1\n", 0 ; "ruff_noqa_space_before_colon")]
#[test_case("# ruff: noqa:\nx = 1\n", 1 ; "ruff_noqa_colon_without_codes")]
#[test_case("# ruff: noqaSMP101\nx = 1\n", 1 ; "ruff_noqa_joined_code")]
#[test_case("# flake8: noqa\nx = 1\n", 0 ; "flake8_noqa_blanket")]
#[test_case("# flake8: noqa: SMP101\nx = 1\n", 0 ; "flake8_noqa_codes")]
#[test_case("x = 1  # ruff: noqa\n", 1 ; "ruff_noqa_trailing")]
#[test_case("def solve():\n    # ruff: noqa\n    x = 1\n    return x\n", 0 ; "ruff_noqa_indented")]
#[test_case("# ruff: file-ignore[SMP101] paper\nx = 1\n", 0 ; "file_ignore")]
#[test_case("#ruff:file-ignore[SMP601,SMP101]\nx = 1\n", 0 ; "file_ignore_without_spaces")]
#[test_case("# ruff :  file-ignore[SMP101]\nx = 1\n", 0 ; "file_ignore_spaced_colon")]
#[test_case("# ruff: file-ignore[SMP601]\nx = 1\n", 1 ; "file_ignore_other_codes")]
#[test_case("# ruff: file-ignore[SMP101\nx = 1\n", 1 ; "file_ignore_unclosed")]
#[test_case("def solve():\n    # ruff: file-ignore[SMP101]\n    x = 1\n    return x\n", 1 ; "file_ignore_indented")]
#[test_case("x = 1  # ruff: file-ignore[SMP101]\n", 1 ; "file_ignore_trailing")]
fn file_level_suppression(source: &str, reports: usize) {
    let root = project(&[("module.py", source)]);
    let stdout = check(root.path(), &["--select", "SMP101"]).1;
    assert_eq!(stdout.matches(" SMP101 ").count(), reports);
}

#[test]
fn blanket_file_level_noqa_keeps_its_reason_report() {
    let root = project(&[
        ("pyproject.toml", "[tool.ruff.lint]\nselect = [\"PGH004\"]\n"),
        ("module.py", "# ruff: noqa\nx = 1\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101,SMP003,SMP004"]).1,
        "module.py:1:1: SMP003 Suppression comment gives no reason; add one after the codes\nFound 1 error.\n"
    );
}

#[test]
fn ignore_noqa_reports_file_level_suppressed_code() {
    let root = project(&[("module.py", "# ruff: noqa: SMP101\nx = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101", "--ignore-noqa"]).0, 1);
}

#[test]
fn simplint_ignore_comment_suppresses_its_codes() {
    let root =
        project(&[("module.py", "x = 1  # simplint: ignore[SMP101, SMP601]\ny = 2  # simplint:ignore[SMP601]\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101"]).1,
        "module.py:2:1: SMP101 Single-letter name `y`; only comprehensions may use one\nFound 1 error.\n"
    );
}

#[test_case(&["--add-noqa"], "x = 1  # noqa: E501, SMP101 long\ny = \"\"\"a\nb\"\"\"  # noqa: SMP101\nz = 1  # noqa\n# --- helpers ---  # noqa: SMP001\n" ; "codes")]
#[test_case(&["--add-noqa=why"], "x = 1  # noqa: E501, SMP101 why\ny = \"\"\"a\nb\"\"\"  # noqa: SMP101 why\nz = 1  # noqa\n# --- helpers ---  # noqa: SMP001 why\n" ; "reason")]
fn add_noqa_adds_directives(flags: &[&str], expected: &str) {
    let root = project(&[("module.py", NOQA_SOURCE)]);
    let mut args = vec!["--select", "SMP001,SMP101,SMP003"];
    args.extend_from_slice(flags);
    let result = check(root.path(), &args);
    assert_eq!(
        (result, read(root.path(), "module.py")),
        ((0, String::new(), "Added 3 noqa directives.\n".to_owned()), expected.to_owned())
    );
}

const NOQA_SOURCE: &str = "x = 1  # noqa: E501 long\ny = \"\"\"a\nb\"\"\"\nz = 1  # noqa\n# --- helpers ---\n";

#[test]
fn add_ignore_merges_into_ignore_comments() {
    let root = project(&[(
        "module.py",
        "x = 1  # simplint: ignore[SMP601] kept\n# simplint: ignore[SMP002]\ny = 1  # noqa: SMP601\n",
    )]);
    let result = check(root.path(), &["--select", "SMP101,SMP003", "--add-ignore"]);
    assert_eq!(
        (result, read(root.path(), "module.py")),
        (
            (0, String::new(), "Added 3 ignore comments.\n".to_owned()),
            "x = 1  # simplint: ignore[missing-final, single-letter-name] kept\n# simplint: ignore[long-paragraph, suppression-without-reason]\ny = 1  # noqa: SMP601  # simplint: ignore[single-letter-name, suppression-without-reason]\n".to_owned()
        )
    );
}

// The reason after the codes and any comment after it stay, unless a new reason replaces the old one.
#[test_case("--add-noqa", "x = 1  # noqa: E501 long  # type: ignore[misc]\n", "x = 1  # noqa: E501, SMP101 long  # type: ignore[misc]\n" ; "noqa_keeps_reason_and_comment")]
#[test_case("--add-noqa=why", "x = 1  # noqa: E501 long  # type: ignore[misc]\n", "x = 1  # noqa: E501, SMP101 why  # type: ignore[misc]\n" ; "noqa_replaces_reason")]
#[test_case("--add-noqa=why", "x = 1  # noqa: E501# type: ignore[misc]\n", "x = 1  # noqa: E501, SMP101 why  # type: ignore[misc]\n" ; "noqa_reason_before_joined_comment")]
#[test_case("--add-noqa=why", "x = 1  # noqa: E501 long\n", "x = 1  # noqa: E501, SMP101 why\n" ; "noqa_replaces_reason_without_comment")]
#[test_case("--add-ignore", "x = 1  # simplint: ignore[SMP601] kept  # pragma: no cover\n", "x = 1  # simplint: ignore[missing-final, single-letter-name] kept  # pragma: no cover\n" ; "ignore_keeps_reason_and_comment")]
#[test_case("--add-ignore=paper", "x = 1  # simplint: ignore[SMP601] kept  # pragma: no cover\n", "x = 1  # simplint: ignore[missing-final, single-letter-name] paper  # pragma: no cover\n" ; "ignore_replaces_reason")]
fn add_suppression_keeps_what_follows_the_codes(flag: &str, source: &str, expected: &str) {
    let root = project(&[("module.py", source)]);
    check(root.path(), &["--select", "SMP101", flag]);
    assert_eq!(read(root.path(), "module.py"), expected);
}

// The merge renames the comment's numeric codes, which leaves nothing for a `numeric-ignore` selector to silence there.
#[test]
fn add_ignore_does_not_name_the_numeric_codes_it_renames() {
    let root = project(&[(
        "module.py",
        "x = 1  # simplint: ignore[SMP601] kept\ny = 2  # simplint: ignore[SMP101] kept\n# simplint: ignore[SMP101]\n",
    )]);
    let result = check(root.path(), &["--select", "SMP101,SMP008", "--add-ignore"]);
    assert_eq!(
        (result, read(root.path(), "module.py")),
        (
            (0, String::new(), "Added 3 ignore comments.\n".to_owned()),
            "x = 1  # simplint: ignore[missing-final, single-letter-name] kept\ny = 2  # simplint: ignore[single-letter-name] kept\n# simplint: ignore[single-letter-name]\n".to_owned()
        )
    );
}

#[test]
fn add_ignore_counts_one_comment() {
    let root = project(&[("module.py", "x = 1")]);
    let result = check(root.path(), &["--select", "SMP101", "--add-ignore=paper"]);
    // A file without a line break gets the platform's, as in Ruff.
    let newline = if cfg!(windows) { "\r\n" } else { "\n" };
    assert_eq!(
        (result, read(root.path(), "module.py")),
        (
            (0, String::new(), "Added 1 ignore comment.\n".to_owned()),
            format!("x = 1  # simplint: ignore[single-letter-name] paper{newline}")
        )
    );
}

#[test]
fn add_noqa_quiet_prints_nothing() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(check(root.path(), &["--quiet", "--select", "SMP101", "--add-noqa"]), (0, String::new(), String::new()));
}

#[test]
fn add_noqa_rejects_a_reason_over_lines() {
    let root = project(&[("module.py", "x = 1\n")]);
    assert_eq!(
        check(root.path(), &["--add-noqa=a\nb"]),
        (
            2,
            String::new(),
            "simplint failed\n  Cause: --add-noqa <reason> cannot contain newline characters\n".to_owned()
        )
    );
}

#[test]
fn add_noqa_warns_when_fixes_are_configured() {
    let root = project(&[("pyproject.toml", "[tool.simplint]\nfix = true\n"), ("module.py", "x = 1\n")]);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101", "--add-noqa"]);
    assert_eq!(
        (code, stderr),
        (0, "warning: --fix is incompatible with --add-noqa.\nAdded 1 noqa directive.\n".to_owned())
    );
}

#[test]
fn add_noqa_skips_a_file_that_does_not_parse() {
    let root = project(&[("module.py", "x = (\n")]);
    assert_eq!(check(root.path(), &["--add-noqa"]), (0, String::new(), String::new()));
}

#[test]
fn add_noqa_logs_a_read_only_file() {
    let root = project(&[("module.py", "x = 1\n")]);
    common::make_read_only(&root.path().join("module.py"));
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101", "--add-noqa"]);
    assert_eq!((code, stderr.contains("Failed to add suppression to")), (0, true));
}

#[cfg(unix)]
#[test]
fn add_noqa_logs_an_unreadable_file() {
    let root = project(&[("module.py", "x = 1\n")]);
    common::set_mode(&root.path().join("module.py"), 0o000);
    let (code, _, stderr) = check(root.path(), &["--select", "SMP101", "--add-noqa"]);
    assert_eq!((code, stderr.contains("Failed to extract source from")), (0, true));
}

#[test]
fn unused_ignore_reports_and_fixes_each_selector() {
    let root = project(&[(
        "module.py",
        "x = 1  # simplint: ignore[SMP101, SMP602]  # needed for x\nvalue = 2  # simplint: ignore[SMP602]  # stale\n",
    )]);
    let (status, output, _) = check(root.path(), &["--select", "SMP101,SMP009"]);
    assert_eq!(status, 1);
    assert_eq!(output.matches(" SMP009 ").count(), 1);
    assert!(output.contains("2 unused simplint ignore selectors"), "{output}");
    assert_eq!(check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), "x = 1  # simplint: ignore[SMP101]  # needed for x\nvalue = 2\n");
}

#[test_case("[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 2\n", true ; "local_limit_blocks_report")]
#[test_case("[tool.ruff.lint]\nselect = []\n", false ; "comment_exposes_report")]
fn unused_ignore_rechecks_comment_sensitive_rules(config: &str, removed: bool) {
    let source = "_MESSAGE = '''hello\nworld'''  # simplint: ignore[SMP202]  # moving adds a local\n\n\ndef read() -> str:\n    first = 1\n    second = 2\n    return _MESSAGE + str(first + second)\n";
    let root = project(&[("pyproject.toml", config), ("module.py", source)]);
    let (status, output, _) = check(root.path(), &["--select", "SMP202,SMP009"]);
    assert_eq!(output.contains("unused-ignore"), removed, "{output}");
    assert_eq!(status == 1, removed);
    check(root.path(), &["--select", "SMP202,SMP009", "--fix"]);
    assert_eq!(read(root.path(), "module.py").contains("simplint: ignore"), !removed);
}

#[test]
fn unused_ignore_keeps_required_selectors() {
    let source = "x = 1  # simplint: ignore[SMP101]  # short fixture name\n";
    let root = project(&[("module.py", source)]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101,SMP009", "--fix"]),
        (0, "All checks passed!\n".to_owned(), String::new())
    );
    assert_eq!(read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_preserves_crlf() {
    let root = project(&[("module.py", "value = 1  # simplint: ignore[SMP101]\r\nnext_value = 2\r\n")]);
    check(root.path(), &["--select", "SMP101,SMP009", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), "value = 1\r\nnext_value = 2\r\n");
}

#[test]
fn unused_ignore_drops_standalone_comment_without_hiding_later_reports() {
    let root = project(&[("module.py", "# simplint: ignore[SMP602]  # stale\nx = 1\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 1);
    assert_eq!(read(root.path(), "module.py"), "x = 1\n");
    assert_eq!(check(root.path(), &["--select", "SMP101"]).0, 1);
}

#[test]
fn unused_ignore_removes_a_selector_already_covered_by_noqa() {
    let root = project(&[("module.py", "x = 1  # noqa: SMP101  # simplint: ignore[SMP101]\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), "x = 1  # noqa: SMP101\n");
}

#[test]
fn unused_ignore_honors_unfixable() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = project(&[("module.py", source)]);
    assert_eq!(check(root.path(), &["--select", "SMP009", "--unfixable", "SMP009", "--fix"]).0, 1);
    assert_eq!(read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_is_opt_in() {
    let root = project(&[("module.py", "value = 1  # simplint: ignore[SMP101]\n")]);
    assert!(!check(root.path(), &[]).1.contains("SMP009"));
    assert!(check(root.path(), &["--extend-select", "SMP009"]).1.contains("SMP009"));
    check(root.path(), &["--extend-select", "SMP009", "--fix"]);
    assert_eq!(read(root.path(), "module.py"), "value = 1\n");
}

#[test]
fn unused_ignore_fix_appears_in_diff() {
    let source = "value = 1  # simplint: ignore[SMP101]\n";
    let root = project(&[("module.py", source)]);
    let (status, output, _) = check(root.path(), &["--select", "SMP009", "--diff"]);
    assert_eq!(status, 1);
    assert!(output.contains("+value = 1\n"), "{output}");
    assert_eq!(read(root.path(), "module.py"), source);
}

#[test]
fn unused_ignore_follows_numeric_ignore_fix() {
    let root = project(&[("module.py", "value = 1  # simplint: ignore[SMP602]\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP008,SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), "value = 1\n");
}

#[test]
fn unused_ignore_fixes_many_comments_in_one_pass() {
    let source = (0..120).fold(String::new(), |mut source, index| {
        writeln!(source, "value_{index} = {index}  # simplint: ignore[SMP101]").expect("write to string");
        source
    });
    let root = project(&[("module.py", &source)]);
    assert_eq!(check(root.path(), &["--select", "SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), source.replace("  # simplint: ignore[SMP101]", ""));
}

#[test]
fn unused_ignore_removes_multiple_selectors_after_unicode() {
    let root = project(&[("module.py", "name = 'é'  # simplint: ignore[SMP101, SMP602]\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), "name = 'é'\n");
}

#[test]
fn unused_ignore_rewrites_separators_around_a_stale_selector() {
    let root = project(&[(
        "module.py",
        "from typing import Any\n\n\ndef x(value: Any) -> None:  # simplint: ignore[SMP101,SMP603,SMP602]\n    pass\n",
    )]);
    assert_eq!(check(root.path(), &["--select", "SMP101,SMP602,SMP009", "--fix"]).0, 0);
    assert_eq!(
        read(root.path(), "module.py"),
        "from typing import Any\n\n\ndef x(value: Any) -> None:  # simplint: ignore[SMP101, SMP602]\n    pass\n"
    );
}

#[test]
fn unused_ignore_handles_file_level_suppression() {
    let root = project(&[("module.py", "# ruff: noqa: SMP101\nx = 1  # simplint: ignore[SMP101]\n")]);
    assert_eq!(check(root.path(), &["--select", "SMP101,SMP009", "--fix"]).0, 0);
    assert_eq!(read(root.path(), "module.py"), "# ruff: noqa: SMP101\nx = 1\n");
}

// SMP008 reports on the codes each removal edits, which leaves the stale numeric selectors no less unused.
#[test]
fn unused_ignore_reports_numeric_selectors_next_to_numeric_ignore() {
    let root = project(&[("module.py", "x = 1  # simplint: ignore[SMP602,SMP101]\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP008,SMP009"]),
        (
            1,
            "module.py:1:8: SMP009 [*] 2 unused simplint ignore selectors\nmodule.py:1:27: SMP008 [*] Numeric simplint \
             ignore; use rule names\nFound 2 errors.\n[*] 2 fixable with `--fix`.\n"
                .to_owned(),
            String::new()
        )
    );
}
