use super::*;
use crate::common::{check, project};
use test_case::test_case;

const REPEATED: &str = "def build(items):\n    total = 0\n    for item in items:\n        total += item\n    count = \
                        len(items)\n    log(total)\n    log(count)\n    return total\n";

#[test]
fn duplicate_block_repeated_in_pieces_names_each_repeating_file_once() {
    let whole = "def run(log):\n    log(1)\n    log(2)\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n    \
    log(7)\n    log(8)\n";
    let head = "def run(log):\n    log(1)\n    log(2)\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n";
    let tail = "def run(log):\n    log(3)\n    log(4)\n    log(5)\n    log(6)\n    log(7)\n    log(8)\n";
    let root = project(&[("a.py", whole), ("b.py", whole), ("c.py", head), ("d.py", tail)]);
    assert!(check(root.path(), &["--select", "SMP530"]).1.contains(
        "a.py:2:5: SMP530 These statements repeat in `b.py:2`, `c.py:2`, `d.py:2`; move them into one definition the \
             modules share"
    ));
}

const DUP_BLOCK: &str = "def run(items):\n    total = 0\n    for item in items:\n        total += item\n    count = \
    len(items)\n    mean = total / count\n    spread = max(items) - min(items)\n    label = f\"{mean}:{spread}\"\n    \
    print(label)\n";

const DUP_EXTRA: &str = "    extra = label.upper()\n    more = extra.lower()\n    again = more.title()\n    final = \
    again.strip()\n    return final\n";

// Each file names the first three other files in sorted order, one place each, and counts the rest. A later statement
// that only two files share does not make a file a partner of the whole block, nor name it at that later line.
#[test]
fn repeated_block_names_three_files_one_place_each() {
    let repeated = (1..=4)
        .map(|index| format!("{}\n\n", DUP_BLOCK.replace("def run(", &format!("def run{index}("))))
        .collect::<Vec<_>>()
        .concat();
    let longer = format!("{DUP_BLOCK}{DUP_EXTRA}");
    let root = project(&[
        ("a.py", repeated.as_str()),
        ("b.py", DUP_BLOCK),
        ("c.py", DUP_BLOCK),
        ("d.py", DUP_BLOCK),
        ("e.py", DUP_BLOCK),
        ("f.py", DUP_BLOCK),
        ("g.py", longer.as_str()),
        ("h.py", longer.as_str()),
    ]);
    let stdout = check(root.path(), &["--select", "SMP530"]).1;
    assert!(stdout.contains(
        "b.py:2:5: SMP530 These statements repeat in `a.py:2`, `c.py:2`, `d.py:2`, and 4 more; move them into one \
        definition \
         the modules share\n"
    ));
    assert!(stdout.contains(
        "g.py:2:5: SMP530 These statements repeat in `a.py:2`, `b.py:2`, `c.py:2`, and 4 more; move them into one \
        definition \
         the modules share\n"
    ));
    assert!(!stdout.contains("h.py:5"), "{stdout}");
}

#[test]
fn package_outside_the_source_roots_resolves_from_its_parent() {
    let root = project(&[
        ("lib/pkg/__init__.py", ""),
        ("lib/pkg/helpers.py", "def helper() -> None:\n    pass\n"),
        ("lib/pkg/use.py", "from pkg.helpers import helper\n"),
    ]);
    assert_eq!(
        check(root.path(), &["--select", "SMP401"]).1.lines().next(),
        Some(
            common::native(
                "lib/pkg/helpers.py:1:5: SMP401 `helper` is imported by other modules; list it in `__all__`"
            )
            .as_str()
        )
    );
}

// Test directories of a project are projects of their own, each repeating the code of the others by design, so their
// copies are not compared with one another, nor with the main project.
#[test_case(
    &[
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/parser.py", REPEATED),
        ("roots/test-b/conf.py", ""),
        ("roots/test-b/parser.py", REPEATED),
    ]
    ; "nested_test_projects"
)]
#[test_case(
    &[
        ("pkg/parser.py", REPEATED),
        ("roots/test-a/conf.py", ""),
        ("roots/test-a/parser.py", REPEATED),
    ]
    ; "main_project"
)]
fn nested_test_projects_are_not_compared(files: &[(&str, &str)]) {
    assert_eq!(duplicate_code_reports(files), Vec::<String>::new());
}

fn duplicate_code_reports(files: &[(&str, &str)]) -> Vec<String> {
    let root = project(files);
    let stdout = check(root.path(), &["--select", "SMP530"]).1;
    let mut reported: Vec<String> = common::reports(&stdout, "SMP530")
        .into_iter()
        .filter_map(|line| line.split(':').next())
        .map(|path| path.replace(std::path::MAIN_SEPARATOR, "/"))
        .collect();
    reported.sort_unstable();
    reported
}

#[test]
fn repeats_within_one_nested_test_project_are_reported() {
    assert_eq!(
        duplicate_code_reports(&[
            ("roots/test-a/conf.py", ""),
            ("roots/test-a/parser.py", REPEATED),
            ("roots/test-a/other.py", REPEATED),
            ("roots/test-b/conf.py", ""),
            ("roots/test-b/parser.py", REPEATED),
        ]),
        ["roots/test-a/other.py", "roots/test-a/parser.py"]
    );
}

#[test_case("conf.py", "" ; "sphinx_conf")]
#[test_case("pyproject.toml", "[project]\nname = \"nested\"\n" ; "pyproject")]
#[test_case("setup.py", "" ; "setup_py")]
#[test_case("setup.cfg", "[metadata]\nname = nested\n" ; "setup_cfg")]
fn each_project_marker_makes_a_nested_test_project(marker: &str, content: &str) {
    assert_eq!(
        duplicate_code_reports(&[
            ("pkg/parser.py", REPEATED),
            ("roots/test-a/parser.py", REPEATED),
            ("roots/test-a/other.py", REPEATED),
            (&format!("roots/test-a/{marker}"), content),
        ]),
        ["roots/test-a/other.py", "roots/test-a/parser.py"]
    );
}

#[test]
fn marked_directory_without_a_test_name_is_the_main_project() {
    assert_eq!(
        duplicate_code_reports(&[
            ("pkg/parser.py", REPEATED),
            ("roots/fixtures/conf.py", ""),
            ("roots/fixtures/parser.py", REPEATED),
        ]),
        ["pkg/parser.py", "roots/fixtures/parser.py"]
    );
}

#[test]
fn separate_distributions_are_not_compared_for_duplicate_code() {
    assert_eq!(
        duplicate_code_reports(&[
            ("first/pyproject.toml", "[project]\nname = \"first\"\n"),
            ("first/src/first/a.py", REPEATED),
            ("first/src/first/b.py", REPEATED),
            ("second/pyproject.toml", "[project]\nname = \"second\"\n"),
            ("second/src/second/c.py", REPEATED),
        ]),
        ["first/src/first/a.py", "first/src/first/b.py"]
    );
}

// The same statements under a different `if` are no repeat: the nesting of the code differs, though its text does not.
const NESTED_FIRST: &str = "def run(x, y):\n    a = 1\n    b = 2\n    if x:\n        a += 1\n        if \
    y:\n            b += 1\n        b += 10\n    c = a + b\n    d = c * 2\n    return d\n";

const NESTED_SECOND: &str = "def run(x, y):\n    a = 1\n    b = 2\n    if x:\n        a += 1\n        if \
    y:\n            b += 1\n            b += 10\n    c = a + b\n    d = c * 2\n    return d\n";

// Six repeated statements, one copy right after the end of an `if` block, whose dedent is not part of the repeat.
const AFTER_BLOCK: &str = "def run(x, y):\n    if x:\n        log(x)\n    a = 1\n    b = 2\n    c = a + b\n    d = c * \
    2\n    log(d)\n    return d\n";

const AFTER_STATEMENT: &str =
    "def run(x, y):\n    log(y)\n    a = 1\n    b = 2\n    c = a + b\n    d = c * 2\n    log(d)\n    return d\n";

#[test_case(NESTED_FIRST, NESTED_SECOND, &[] ; "nesting_differs")]
#[test_case(NESTED_FIRST, NESTED_FIRST, &["first.py", "second.py"] ; "nesting_repeats")]
#[test_case(AFTER_BLOCK, AFTER_STATEMENT, &["first.py", "second.py"] ; "repeat_after_a_block")]
fn statements_nested_differently_are_not_repeats(first: &str, second: &str, reported: &[&str]) {
    assert_eq!(duplicate_code_reports(&[("first.py", first), ("second.py", second)]), reported);
}

#[test]
fn unmarked_test_directory_is_the_main_project() {
    assert_eq!(
        duplicate_code_reports(&[("pkg/parser.py", REPEATED), ("roots/test-a/parser.py", REPEATED)]),
        ["pkg/parser.py", "roots/test-a/parser.py"]
    );
}

// A nested project rooted at a `test` directory is test input, as the directory is one, so its repeated statements are
// not compared with the rest of the run; a directory of the same layout that no test directory names is compared.
#[test_case("tools/swig/test", false ; "test_directory_is_not_compared")]
#[test_case("tools/swig/checks", true ; "other_directory_is_compared")]
fn nested_test_directory_repeats_are_not_compared(directory: &str, reported: bool) {
    let suite = "    suite.addTest(unittest.TestLoader().loadTestsFromTestCase(case))\n".repeat(9);
    let block = format!(
        "import unittest\n\n\nif __name__ == \"__main__\":\n    suite = unittest.TestSuite()\n{suite}    \
        unittest.TextTestRunner(verbosity=2).run(suite)\n"
    );
    let mut files = vec![(format!("{directory}/setup.py"), String::new())];
    files.push((format!("{directory}/a.py"), block.clone()));
    files.push((format!("{directory}/b.py"), block));
    let files: Vec<(&str, &str)> = files.iter().map(|(name, content)| (name.as_str(), content.as_str())).collect();
    let stdout = check(project(&files).path(), &["--select", "SMP530"]).1;
    assert_eq!(stdout.contains(" SMP530 "), reported, "{stdout}");
}

// A nested project is indexed on its own: a name `widget.helper()` in one nested project uses is not a use of another
// nested project's `helper`, while the same layout outside a test-named directory shares one index.
#[test_case("roots/test-a", "roots/test-b", &["roots/test-a/support.py:1:5: SMP409 `helper` is never used in the run; \
    remove it or give it a caller", "roots/test-b/use.py:4:5: SMP409 `run` is never used in the run; remove it or give it \
    a caller"] ; "nested_projects_are_indexed_apart")]
#[test_case("roots/fixtures", "roots/other", &["roots/other/use.py:4:5: SMP409 `run` is never used in the run; remove \
    it or give it a caller"] ; "directories_without_test_names_share_one_index")]
fn name_reached_only_from_another_project_is_unused(first: &str, second: &str, expected: &[&str]) {
    let root = project(&[
        (&format!("{first}/conf.py"), ""),
        (&format!("{first}/support.py"), "def helper() -> None:\n    pass\n"),
        (&format!("{second}/conf.py"), ""),
        (&format!("{second}/use.py"), "import support\n\n\ndef run(widget):\n    widget.helper()\n"),
    ]);
    let stdout = check(root.path(), &["--select", "SMP409"]).1;
    let reported: Vec<&str> = stdout.lines().filter(|line| line.contains(" SMP409 ")).collect();
    let expected: Vec<String> = expected.iter().map(|line| common::native(line)).collect();
    assert_eq!(reported, expected.iter().map(String::as_str).collect::<Vec<_>>());
}
