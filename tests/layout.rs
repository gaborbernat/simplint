mod common;

use test_case::test_case;

use common::{check, project, read};

// What a fix leaves of `module.py` under a line length limit with E501 on: a fix that makes a line too long for any
// layout `ruff format` can give it is withdrawn, one that only seems to is kept.
fn fixed(line_length: usize, source: &str, select: &str) -> (i32, String, String) {
    let pyproject = format!("[tool.ruff]\nline-length = {line_length}\n[tool.ruff.lint]\nselect = [\"E501\"]\n");
    fixed_with(&pyproject, source, select)
}

fn fixed_with(pyproject: &str, source: &str, select: &str) -> (i32, String, String) {
    fixed_file(pyproject, "module.py", source, select)
}

fn fixed_file(pyproject: &str, name: &str, source: &str, select: &str) -> (i32, String, String) {
    let root = project(&[("pyproject.toml", pyproject), (name, source)]);
    let (code, stdout, _) = check(root.path(), &["--fix", "--unsafe-fixes", "--select", select]);
    (code, stdout, read(root.path(), name))
}

#[test_case(
    27, "_DOC = \"\"\"Some words here\nmore\"\"\"\n\n\ndef show() -> str:\n    return _DOC\n", "SMP050",
    "module.py:1:1: SMP050 `_DOC` is used by one function only; define it there\nFound 1 error.\n" ;
    "first line of a string over several lines"
)]
#[test_case(
    35, "_KEY = 8 * \"0123456789abcdef0123\"\n\n\ndef keys() -> list:\n    return [(True, _KEY)]\n", "SMP050",
    "module.py:1:1: SMP050 `_KEY` is used by one function only; define it there\nFound 1 error.\n" ;
    "operand after the operator of a split operation"
)]
#[test_case(
    38,
    "_TEXT = \"some words that are long\"\n\n\ndef text(flag: bool) -> str:\n    return (\"short\" if flag else _TEXT)\n",
    "SMP050",
    "module.py:1:1: SMP050 `_TEXT` is used by one function only; define it there\nFound 1 error.\n" ;
    "operand after else"
)]
#[test_case(
    38,
    "_TEXT = \"some words that are long\"\n\n\ndef text(flag: bool) -> str:\n    return \"short\" if flag else _TEXT\n",
    "SMP050",
    "module.py:1:1: SMP050 `_TEXT` is used by one function only; define it there\nFound 1 error.\n" ;
    "operand after else outside brackets"
)]
#[test_case(
    40,
    "_SKIP = {\n    # a comment long enough to matter\n    \"a\",\n}\n\n\ndef skipped(name: str) -> bool:\n    return name in \
     _SKIP\n",
    "SMP050",
    "module.py:1:1: SMP050 `_SKIP` is used by one function only; define it there\nFound 1 error.\n" ;
    "comment on its own line in brackets"
)]
#[test_case(
    42, "names = [  # names that the checks rely on\n    \"a\",\n]\nprint(names)\n", "SMP102",
    "module.py:1:1: SMP102 Module-scope variable `names` should be named `_NAMES`\nFound 1 error.\n" ;
    "comment after an opening bracket"
)]
#[test_case(
    57,
    "def label(name: str) -> str:\n    value = (\"abc \" + name + \" more words\")  # explains the value\n    return value\n",
    "SMP112",
    "module.py:2:14: SMP112 Build text with an f-string instead of `+`\nFound 1 error.\n" ;
    "comment after a value whose parentheses go"
)]
#[test_case(
    50,
    "def show(name: str) -> None:\n    print(\n        \"x\",\n        end=\"aaaa\"\n        + name\n        + \"bbbb\",  # a \
     trailing comment\n    )\n",
    "SMP112",
    "module.py:4:13: SMP112 Build text with an f-string instead of `+`\nFound 1 error.\n" ;
    "comment after the last operand of an item"
)]
#[test_case(
    40, "GREETING = \"hello general kenobi, you are bolder\"  # noqa: E501\n", "SMP101",
    "module.py:1:1: SMP101 `GREETING` is never reassigned; annotate it `Final`\nFound 1 error.\n" ;
    "pragma the annotation moves"
)]
#[test_case(
    17, "# fmt: off\nvalue = [1, 2, 3]\n# fmt: on\nprint(value)\n", "SMP102",
    "module.py:2:1: SMP102 Module-scope variable `value` should be named `_VALUE`\nFound 1 error.\n" ;
    "code under fmt off"
)]
#[test_case(
    17, "# fmt: off\nvalue = [1, 2, 3]\nNAME = \"abc\"\n# fmt: on\nprint(value, NAME)\n", "SMP101,SMP102",
    "module.py:2:1: SMP101 `value` is never reassigned; annotate it `Final`\nmodule.py:2:1: SMP102 Module-scope variable \
     `value` should be named `_VALUE`\nmodule.py:3:1: SMP101 `NAME` is never reassigned; annotate it `Final`\nmodule.py:3:1: \
     SMP102 Module-scope variable `NAME` should be named `_NAME`\nFound 4 errors.\n" ;
    "two rules under fmt off"
)]
#[test_case(
    17, "# fmt: off\nvalue = [1, 2, 3]\nprint(value)\n", "SMP102",
    "module.py:2:1: SMP102 Module-scope variable `value` should be named `_VALUE`\nFound 1 error.\n" ;
    "code under fmt off to the end"
)]
#[test_case(
    30, "value = [1, 2, 3]  # fmt: skip\nprint(value)\n", "SMP102",
    "module.py:1:1: SMP102 Module-scope variable `value` should be named `_VALUE`\nFound 1 error.\n" ;
    "statement under fmt skip"
)]
#[test_case(
    40, "NAME = \"x\"\nNESTED = f\"{f'{NAME}'}\"\nGREETING = \"<div>\" + NAME + \"</div><span>text</span>\"\n", "SMP112",
    "module.py:3:12: SMP112 Build text with an f-string instead of `+`\nFound 1 error.\n" ;
    "f-string too long for its line"
)]
#[test_case(
    40, "NAME = \"x\"\nDATA = {\n    \"key\": \"<div>\"\n    + NAME\n    + \"</div>abcdefghij\",\n}\n", "SMP112",
    "module.py:3:12: SMP112 Build text with an f-string instead of `+`\nFound 1 error.\n" ;
    "dict value glued to its key"
)]
fn fix_is_withdrawn_when_no_layout_fits(line_length: usize, source: &str, select: &str, report: &str) {
    assert_eq!(fixed(line_length, source, select), (1, report.to_owned(), source.to_owned()));
}

// The formatter keeps the parentheses after a `def` name on its line, and when they are empty what follows them up to
// the return annotation it splits: `def test_settings_loads() -> (`.
#[test_case(25, "class TestSettings:\n    def test_loads(self) -> None:\n        assert True\n" ; "return annotation")]
#[test_case(24, "class TestSettings:\n    def test_loads(self):\n        assert True\n" ; "no return annotation")]
#[test_case(20, "class TestSettings:\n    def test_loads(self) -> None:\n        assert True\n" ; "name alone too long")]
fn fix_is_withdrawn_when_a_def_line_cannot_fit(line_length: usize, source: &str) {
    let pyproject = format!("[tool.ruff]\nline-length = {line_length}\n[tool.ruff.lint]\nselect = [\"E501\"]\n");
    assert_eq!(
        fixed_file(&pyproject, "test_module.py", source, "SMP200"),
        (
            1,
            "test_module.py:1:7: SMP200 Test class `TestSettings`; use module-level test functions with a name prefix\n\
             Found 1 error.\n"
                .to_owned(),
            source.to_owned()
        )
    );
}

#[test_case(
    "[tool.ruff]\nline-length = 17\n[tool.ruff.lint]\nselect = [\"E501\"]\n[tool.ruff.format]\nexclude = [\"module.py\"]\n",
    "value = [1, 2, 3]\nprint(value)\n",
    "SMP102",
    "module.py:1:1: SMP102 Module-scope variable `value` should be named `_VALUE`\nFound 1 error.\n" ;
    "file ruff format skips"
)]
#[test_case(
    "[tool.ruff]\nline-length = 40\n[tool.ruff.lint]\nselect = [\"F401\"]\n",
    "from typing import Final\n\nGREETING = \"hello, \" \"general kenobi\"  # noqa: B039\n",
    "SMP101",
    "module.py:3:1: SMP101 `GREETING` is never reassigned; annotate it `Final`\nFound 1 error.\n" ;
    "noqa ruff format would move"
)]
#[test_case(
    "[tool.ruff]\nline-length = 30\n[tool.ruff.lint]\nselect = [\"E501\"]\npycodestyle.max-line-length = 45\n",
    "def greet(name: str) -> str:\n    return \"hello \" + name + \" and some more words\"\n",
    "SMP112",
    "module.py:2:12: SMP112 Build text with an f-string instead of `+`\nFound 1 error.\n" ;
    "atoms judged by the format line width"
)]
// Annotated, the string no longer fits beside its name, so `ruff format` would wrap it and move the suppression.
#[test_case(
    "[tool.ruff]\nline-length = 40\npreview = true\n[tool.ruff.lint]\nselect = [\"E501\", \"RUF100\"]\n",
    "LONG = \"\"\"words words words words words words words words \"\"\"  # ruff: ignore[line-too-long]\n",
    "SMP101",
    "module.py:1:1: missing-final: `LONG` is never reassigned; annotate it `Final`\nFound 1 error.\n" ;
    "line length suppression ruff would remove"
)]
fn fix_is_withdrawn_under_format_settings(pyproject: &str, source: &str, select: &str, report: &str) {
    assert_eq!(fixed_with(pyproject, source, select), (1, report.to_owned(), source.to_owned()));
}

#[test_case(
    40,
    "_M = \"a message with some spaces\"\n\n\ndef fail(error: Exception) -> None:\n    raise ValueError(_M) from error\n",
    "SMP050",
    "def fail(error: Exception) -> None:\n    raise ValueError(\"a message with some spaces\") from error\n" ;
    "code after a bracket the formatter splits"
)]
#[test_case(
    60,
    "def link() -> str:\n    return \"see the docs at \" + \"https://example.com/\" + \"some/very/long/path/to/page\"\n",
    "SMP112",
    "def link() -> str:\n    return \"see the docs at https://example.com/some/very/long/path/to/page\"\n" ;
    "line ending in a url"
)]
#[test_case(
    40,
    "from typing import Final\n\nGREETING = \"hello, \" \"general kenobi\"\n",
    "SMP101",
    "from typing import Final\n\nGREETING: Final[str] = \"hello, \" \"general kenobi\"\n" ;
    "line ruff format can shorten"
)]
#[test_case(
    40,
    "\"\"\"Greetings.\n\nA docstring line that runs past the forty character limit.\n\"\"\"\n\nfrom typing import Final\n\nGREETING \
     = \"hello, \" \"general kenobi\"\nprint(GREETING)\n",
    "SMP101",
    "\"\"\"Greetings.\n\nA docstring line that runs past the forty character limit.\n\"\"\"\n\nfrom typing import Final\n\nGREETING: \
     Final[str] = \"hello, \" \"general kenobi\"\nprint(GREETING)\n" ;
    "long line the fix does not touch"
)]
fn fix_is_kept_when_a_layout_fits(line_length: usize, source: &str, select: &str, expected: &str) {
    assert_eq!(
        fixed(line_length, source, select),
        (0, "Found 1 error (1 fixed, 0 remaining).\n".to_owned(), expected.to_owned())
    );
}

// The formatter starts a line with the operator when it splits a binary operation, `not in` included, but never with
// an augmented assignment. The formatter's line width is narrower than Ruff's limit, so the split is all that decides.
#[test_case(
    38,
    "_TEXT = \"some words that are long\"\n\n\ndef text() -> str:\n    value = \"a\"\n    value |= _TEXT\n    return value\n",
    1,
    "module.py:1:1: SMP050 `_TEXT` is used by one function only; define it there\nFound 1 error.\n",
    "_TEXT = \"some words that are long\"\n\n\ndef text() -> str:\n    value = \"a\"\n    value |= _TEXT\n    return value\n" ;
    "augmented assignment stays on its line"
)]
#[test_case(
    41,
    "_SKIP = \"some words that are long\"\n\n\ndef skipped(name: str) -> bool:\n    return name not in _SKIP\n",
    0,
    "Found 1 error (1 fixed, 0 remaining).\n",
    "def skipped(name: str) -> bool:\n    return name not in \"some words that are long\"\n" ;
    "operand after not in"
)]
fn split_operation_starts_with_its_operator(limit: usize, source: &str, code: i32, stdout: &str, expected: &str) {
    let pyproject = format!(
        "[tool.ruff]\nline-length = 30\n[tool.ruff.lint]\nselect = [\"E501\"]\n[tool.ruff.lint.pycodestyle]\n\
         max-line-length = {limit}\n"
    );
    assert_eq!(fixed_with(&pyproject, source, "SMP050"), (code, stdout.to_owned(), expected.to_owned()));
}

// Comments and pragmas that keep their lines whatever the layout: a comment of one word, one after strings joined
// without an operator, and a pragma after a value too long even in parentheses of its own.
#[test]
fn fix_is_kept_when_comments_and_pragmas_keep_their_lines() {
    let source = "names = [\n    #tight\n    \"a\" \"b\"  # joined\n]\nVALUE = (\"a long string value that cannot fit\")  # \
                  noqa: E501\nprint(names, VALUE)\n";
    assert_eq!(
        fixed(40, source, "SMP102"),
        (
            0,
            "Found 2 errors (2 fixed, 0 remaining).\n".to_owned(),
            source.replace("names", "_NAMES").replace("VALUE", "_VALUE")
        )
    );
}
