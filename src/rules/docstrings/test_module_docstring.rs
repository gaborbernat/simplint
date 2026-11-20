use ruff_diagnostics::Fix;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::{docstring_deletion, ruff_reports_empty_docstring};
use crate::syntax::docstring;

pub fn test_module_docstring(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let body = checker.body();
    let Some(literal) = docstring(body) else {
        return;
    };
    if carries_notice(literal.value.to_str()) {
        return;
    }
    // Removing the docstring can leave a file-level suppression unused.
    let fix = (!checker.state().has_file_noqa(checker))
        .then(|| docstring_deletion(checker, body))
        .flatten()
        .map(Fix::unsafe_edit);
    if fix.is_none() && ruff_reports_empty_docstring(checker, literal) {
        return;
    }
    checker.report_fix(
        Rule::TestModuleDocstring,
        literal.range(),
        "Test module carries a docstring; remove it",
        fix,
        "Remove the docstring",
    );
}

// Preserve license notices embedded in copied tests.
fn carries_notice(text: &str) -> bool {
    let text = text.to_lowercase();
    ["copyright", "\u{a9}", "spdx-license-identifier", "licensed under", "permission is hereby granted"]
        .iter()
        .any(|marker| text.contains(marker))
}
