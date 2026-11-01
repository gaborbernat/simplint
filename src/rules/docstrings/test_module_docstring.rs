use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{docstring, statement_deletion};

pub fn test_module_docstring(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let body = checker.body();
    let Some(literal) = docstring(body) else {
        return;
    };
    let fix = statement_deletion(checker, body[0].range(), body.get(1).map(Ranged::range), true)
        .map(|deletion| Fix::unsafe_edit(Edit::range_deletion(deletion)));
    checker.report_fix(
        Rule::TestModuleDocstring,
        literal.range(),
        "Test module carries a docstring; remove it",
        fix,
        "Remove the docstring",
    );
}
