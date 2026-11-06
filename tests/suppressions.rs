mod common;

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
fn file_level_noqa_with_space_before_colon_silences_long_comment() {
    let root = project(&[("module.py", &format!("# ruff : noqa: E501\n# {}\nx = 1\n", "word ".repeat(40)))]);
    assert_eq!(check(root.path(), &["--select", "SMP007"]).0, 0);
}

#[test]
fn blanket_file_level_noqa_keeps_its_own_report() {
    let root = project(&[("module.py", "# ruff: noqa\nx = 1\n")]);
    assert_eq!(
        check(root.path(), &["--select", "SMP101,SMP003,SMP004"]).1,
        "module.py:1:1: SMP003 Suppression comment gives no reason; add one after the codes\nmodule.py:1:1: SMP004 `# ruff: noqa` suppresses every error in the file; name the codes, as in `# ruff: noqa: CODE`\nFound 2 errors.\n"
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

#[test_case(&["--add-noqa"], "x = 1  # noqa: E501, SMP101\ny = \"\"\"a\nb\"\"\"  # noqa: SMP101\nz = 1  # noqa\n# --- helpers ---  # noqa: SMP001\n" ; "codes")]
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
            "x = 1  # simplint: ignore[SMP101, SMP601]\n# simplint: ignore[SMP002, SMP003]\ny = 1  # noqa: SMP601  # simplint: ignore[SMP003, SMP101]\n".to_owned()
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
            format!("x = 1  # simplint: ignore[SMP101] paper{newline}")
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
