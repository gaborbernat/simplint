use std::collections::BTreeMap;

use test_case::test_case;

use crate::common;

const PROJECT: &str = "[project]\nrequires-python = \">=3.14\"\n";

// A fix writes the line endings of the file it fixes: a CRLF file comes out as the LF one does, with CRLF endings.
#[test_case(PROJECT, "SMP201", &[("module.py", "def _helper() -> int:\n    return 1\n\n\ndef main() -> int:\n    return _helper()\n")] ; "definition_before_use")]
#[test_case(PROJECT, "SMP202", &[("module.py", "_LIMIT = [1]\n\n\ndef read() -> int:\n    return len(_LIMIT) + len(_LIMIT)\n")] ; "single_user_constant_local")]
#[test_case(PROJECT, "SMP202", &[("module.py", "import os\n\n\n_LIMIT = 1\n\n\ndef read() -> int:\n    return _LIMIT + _LIMIT\n")] ; "single_user_constant_between_blank_lines")]
#[test_case(PROJECT, "SMP202", &[("module.py", "_LIMIT = 1\n\n\ndef read() -> int: return _LIMIT + _LIMIT\n")] ; "single_user_constant_on_header")]
#[test_case(PROJECT, "SMP203", &[("module.py", "class Shape:\n    def area(self) -> int:\n        return 1\n\n    def __init__(self) -> None:\n        self.size = 1\n")] ; "class_member_order")]
#[test_case(PROJECT, "SMP401", &[("module.py", "def run() -> None:\n    pass\n"), ("user.py", "from module import run\n")] ; "missing_from_all_new")]
#[test_case(PROJECT, "SMP401", &[("module.py", "__all__ = [\n    \"run\",\n]\n\n\ndef run() -> None:\n    pass\n\n\ndef stop() -> None:\n    pass\n"), ("user.py", "from module import run, stop\n")] ; "missing_from_all_lines")]
#[test_case(PROJECT, "SMP405", &[("module.py", "__all__ = (\"a\", \"b\")\n\na = b = 1\n")] ; "all_format")]
#[test_case(PROJECT, "SMP406", &[("module.py", "__all__ = [\"run\"]\n\n\ndef run() -> None:\n    pass\n")] ; "all_not_last")]
#[test_case(PROJECT, "SMP601", &[("module.py", "LIMIT = 3\n\n\ndef read() -> int:\n    return LIMIT\n")] ; "missing_final_import")]
#[test_case(PROJECT, "SMP801", &[("tests/test_shape.py", "class TestShape:\n    def test_one(self) -> None:\n        assert True\n\n    def test_two(self) -> None:\n        assert True\n")] ; "test_class")]
#[test_case(PROJECT, "SMP812", &[("tests/test_cwd.py", "import unittest\nfrom unittest import mock\n\n\nclass Cwd(unittest.TestCase):\n    def test_cwd(self) -> None:\n        patcher = mock.patch(\"os.getcwd\")\n        patched = patcher.start()\n        self.addCleanup(patcher.stop)\n        assert patched\n")] ; "unittest_enter_context")]
#[test_case("[tool.ruff]\ntarget-version = \"py311\"\n", "SMP513", &[("module.py", "import contextlib\nimport os\n\n\ndef build(target):\n    old = os.getcwd()\n    os.chdir(target)\n    try:\n        run()\n    finally:\n        os.chdir(old)\n")] ; "chdir_restore")]
#[test_case("[tool.ruff]\ntarget-version = \"py314\"\n", "SMP512", &[("module.py", "import contextvars\n\nuser = contextvars.ContextVar(\"user\")\n\n\ndef handle(value):\n    token = user.set(value)\n    try:\n        run()\n    finally:\n        user.reset(token)\n")] ; "context_var_token_reset")]
#[test_case("[tool.ruff]\ntarget-version = \"py313\"\n", "SMP524", &[("module.py", "import warnings\n\n\ndef connect(host):\n    warnings.warn(\"connect() is deprecated\", DeprecationWarning, stacklevel=2)\n    return host\n")] ; "warnings_deprecated")]
#[test_case("[tool.ruff]\ntarget-version = \"py314\"\n", "SMP526", &[("module.py", "def total(items: list[int]) -> int:\n    count = 0\n    items.sort()\n    print(items); print(count)\n    return 0\n")] ; "early_assignment_split_line")]
fn fix_keeps_crlf_line_endings(pyproject: &str, select: &str, files: &[(&str, &str)]) {
    let lf = fixed(pyproject, select, files);
    let crlf_files: Vec<(&str, String)> =
        files.iter().map(|(name, source)| (*name, source.replace('\n', "\r\n"))).collect();
    let crlf =
        fixed(pyproject, select, &crlf_files.iter().map(|(name, source)| (*name, source.as_str())).collect::<Vec<_>>());
    assert_ne!(lf.get(files[0].0).map(String::as_str), Some(files[0].1));
    assert_eq!(crlf, lf.into_iter().map(|(name, source)| (name, source.replace('\n', "\r\n"))).collect());
}

fn fixed(pyproject: &str, select: &str, files: &[(&str, &str)]) -> BTreeMap<String, String> {
    let root = common::project(&[&[("pyproject.toml", pyproject)], files].concat());
    common::check(root.path(), &["--no-cache", "--fix", "--unsafe-fixes", "--select", select]);
    files.iter().map(|(name, _)| ((*name).to_owned(), common::read(root.path(), name))).collect()
}

// The diagnostics of `select` on a module, and the module after its fixes.
fn report_and_fix(select: &str, source: &str) -> (String, String) {
    let root = common::project(&[("pyproject.toml", PROJECT), ("module.py", source)]);
    let report = common::check(root.path(), &["--no-cache", "--select", select]).1;
    common::check(root.path(), &["--no-cache", "--fix", "--unsafe-fixes", "--select", select]);
    (report, common::read(root.path(), "module.py"))
}

#[test_case("SMP201", "# Reads the raw value.\ndef _helper() -> int:\n    return 1\n\n\ndef main() -> int:\n    return _helper()\n" ; "comment_moves_with_definition")]
#[test_case("SMP406", "import os\n\n# fmt: off\n__all__ = [\n    \"first\", \"second\",\n]\n# fmt: on\n\n\ndef first() -> str:\n    return os.sep\n\n\ndef second() -> str:\n    return os.linesep\n" ; "all_not_last_fenced")]
fn crlf_reports_and_fixes_match_lf(select: &str, source: &str) {
    let lf = report_and_fix(select, source);
    let crlf = report_and_fix(select, &source.replace('\n', "\r\n"));
    assert_ne!(lf.1, source);
    assert_eq!(crlf.0, lf.0);
    assert_eq!(crlf.1, lf.1.replace('\n', "\r\n"));
}

#[test_case("\n" ; "lf")]
#[test_case("\r\n" ; "crlf")]
fn comment_above_exempts_constant(line_ending: &str) {
    let source = "import sys\n\n# Reserved name; nothing else may claim it.\n_STORAGE_NAME = \"reserved\"\n\n\ndef claim(names: list[str]) -> None:\n    names.append(_STORAGE_NAME)\n\n\nif __name__ == \"__main__\":\n    claim(sys.argv)\n";
    let root = common::project(&[("pyproject.toml", PROJECT), ("module.py", &source.replace('\n', line_ending))]);
    assert_eq!(
        common::check(root.path(), &["--no-cache", "--select", "SMP202"]),
        (0, common::PASSED.to_owned(), String::new())
    );
}
