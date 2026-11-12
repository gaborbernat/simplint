use std::fs;
use std::path::Path;

use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::collected_tests;
use crate::settings::PytestConfig;
use crate::syntax::{docstring, is_main_guard};

pub fn uncollected_test_module(checker: &Checker) {
    let file_name = checker.file_name();
    if !checker.is_test_module() || matches!(file_name, "conftest.py" | "__init__.py") || !file_name.ends_with(".py") {
        return;
    }
    let pytest = &checker.settings.project.pytest;
    if pytest.collects(file_name) || is_run_on_purpose(checker, file_name) || !has_collected_sibling(checker.path, pytest) {
        return;
    }
    let tests = collected_tests(checker);
    let Some(first) = tests.first() else {
        return;
    };
    // A test another module imports runs where that module is collected.
    if tests.iter().any(|test| checker.project().is_imported(test.name.as_str())) {
        return;
    }
    checker.report(
        Rule::UncollectedTestModule,
        first.name.range(),
        format!(
            "pytest never collects `{file_name}`, since `python_files` ({}) does not match it, so its tests never run; rename the file to match",
            pytest.python_files().iter().map(|pattern| format!("`{pattern}`")).collect::<Vec<_>>().join(", ")
        ),
    );
}

// A script with a main guard, a module another test names as data for `pytester`, or one whose docstring gives the
// command that runs it is kept out of the default run on purpose.
fn is_run_on_purpose(checker: &Checker, file_name: &str) -> bool {
    let stem = file_name.trim_end_matches(".py");
    checker.body().iter().any(is_main_guard)
        || checker.project().string_word_count(stem) > 0
        || docstring(checker.body()).is_some_and(|text| text.value.to_str().contains(file_name))
}

// Outside a directory pytest collects from, such as one holding type-checker inputs, `test_*` functions are not tests.
fn has_collected_sibling(path: &Path, pytest: &PytestConfig) -> bool {
    let Some(directory) = path.parent() else {
        return false;
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry.file_name().to_str().is_some_and(|name| name.ends_with(".py") && name != "conftest.py" && pytest.collects(name))
    })
}
