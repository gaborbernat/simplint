#[path = "../common/mod.rs"]
mod common;

mod autodoc;
mod build_hooks;
mod coverage;
mod dataclasses;
mod doctests;
mod duplicate_code;
mod dynamic_imports;
mod entry_points;
mod nested_projects;
mod patching;
mod pth_files;
mod public_api;
mod resolution;
mod test_only_names;

const PARTITION: &str =
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Partition:\n    topic: str\n    number: int\n";

const LIBRARY: &str = "[project]\nname = \"pkg\"\n[build-system]\nrequires = []\n";

const DEMO: &str = "[project]\nname = \"demo\"\nversion = \"0\"\n";

const DEMO_BUILD: &str = "[build-system]\nrequires = []\n[project]\nname = \"demo\"\n";

const DOCSTRINGED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    \"\"\"Register the flag.\"\"\"\n    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \"\"\"Format messages.\"\"\"\n";

const UNDOCUMENTED_PLUGIN: &str = "\"\"\"Plugin.\"\"\"\n\n\ndef pytest_addoption(parser: object) -> None:\n    print(parser)\n\n\n__all__ = [\"Formatter\"]\n\n\nclass Formatter:\n    \"\"\"Format messages.\"\"\"\n";

const HOOK_PROJECT: &str = "def _run() -> None:\n    print(\"hook\")\n";

// Statements that `duplicate-code` reports when another file of their project repeats them.
const REPEATED: &str = "def build(items):\n    total = 0\n    for item in items:\n        total += item\n    count = \
                        len(items)\n    log(total)\n    log(count)\n    return total\n";

// The names `unused-definition` reports in the run, sorted.
fn unused_names(stdout: &str) -> Vec<&str> {
    let mut names: Vec<&str> =
        common::reports(stdout, "SMP409").into_iter().filter_map(|line| line.split('`').nth(1)).collect();
    names.sort_unstable();
    names
}

const FUNCTIONS: &str =
    "def public() -> None:\n    pass\n\n\ndef other() -> None:\n    pass\n\n\ndef _hidden() -> None:\n    pass\n";

const COVERAGE_PLUGIN: &str = "def coverage_init(reg: object, options: dict[str, str]) -> None:\n    pass\n\n\n";
