use std::collections::BTreeMap;

use test_case::test_case;

use crate::common;

// A fix writes the line endings of the file it fixes: a CRLF file comes out as the LF one does, with CRLF endings.
#[test_case("SMP201", &[("module.py", "def _helper() -> int:\n    return 1\n\n\ndef main() -> int:\n    return _helper()\n")] ; "definition_before_use")]
#[test_case("SMP202", &[("module.py", "_LIMIT = [1]\n\n\ndef read() -> int:\n    return len(_LIMIT) + len(_LIMIT)\n")] ; "single_user_constant_local")]
#[test_case("SMP202", &[("module.py", "import os\n\n\n_LIMIT = 1\n\n\ndef read() -> int:\n    return _LIMIT + _LIMIT\n")] ; "single_user_constant_between_blank_lines")]
#[test_case("SMP202", &[("module.py", "_LIMIT = 1\n\n\ndef read() -> int: return _LIMIT + _LIMIT\n")] ; "single_user_constant_on_header")]
#[test_case("SMP203", &[("module.py", "class Shape:\n    def area(self) -> int:\n        return 1\n\n    def __init__(self) -> None:\n        self.size = 1\n")] ; "class_member_order")]
#[test_case("SMP401", &[("module.py", "def run() -> None:\n    pass\n"), ("user.py", "from module import run\n")] ; "missing_from_all_new")]
#[test_case("SMP401", &[("module.py", "__all__ = [\n    \"run\",\n]\n\n\ndef run() -> None:\n    pass\n\n\ndef stop() -> None:\n    pass\n"), ("user.py", "from module import run, stop\n")] ; "missing_from_all_lines")]
#[test_case("SMP405", &[("module.py", "__all__ = (\"a\", \"b\")\n\na = b = 1\n")] ; "all_format")]
#[test_case("SMP406", &[("module.py", "__all__ = [\"run\"]\n\n\ndef run() -> None:\n    pass\n")] ; "all_not_last")]
#[test_case("SMP601", &[("module.py", "LIMIT = 3\n\n\ndef read() -> int:\n    return LIMIT\n")] ; "missing_final_import")]
#[test_case("SMP801", &[("tests/test_shape.py", "class TestShape:\n    def test_one(self) -> None:\n        assert True\n\n    def test_two(self) -> None:\n        assert True\n")] ; "test_class")]
#[test_case("SMP812", &[("tests/test_cwd.py", "import unittest\nfrom unittest import mock\n\n\nclass Cwd(unittest.TestCase):\n    def test_cwd(self) -> None:\n        patcher = mock.patch(\"os.getcwd\")\n        patched = patcher.start()\n        self.addCleanup(patcher.stop)\n        assert patched\n")] ; "unittest_enter_context")]
fn fix_keeps_crlf_line_endings(select: &str, files: &[(&str, &str)]) {
    let lf = fixed(select, files);
    let crlf_files: Vec<(&str, String)> =
        files.iter().map(|(name, source)| (*name, source.replace('\n', "\r\n"))).collect();
    let crlf = fixed(select, &crlf_files.iter().map(|(name, source)| (*name, source.as_str())).collect::<Vec<_>>());
    assert_ne!(lf.get(files[0].0).map(String::as_str), Some(files[0].1));
    assert_eq!(crlf, lf.into_iter().map(|(name, source)| (name, source.replace('\n', "\r\n"))).collect());
}

fn fixed(select: &str, files: &[(&str, &str)]) -> BTreeMap<String, String> {
    let root = common::project(&[&[("pyproject.toml", "[project]\nrequires-python = \">=3.14\"\n")], files].concat());
    common::check(root.path(), &["--no-cache", "--fix", "--unsafe-fixes", "--select", select]);
    files.iter().map(|(name, _)| ((*name).to_owned(), common::read(root.path(), name))).collect()
}
