use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;

pub fn multiline_module_docstring(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    if let Some(literal) = docstring(checker.body())
        && literal.value.to_str().trim().contains('\n')
    {
        checker.report(
            Rule::MultilineModuleDocstring,
            literal.range(),
            "Module docstring spans more than one line; keep one line saying what the module is",
        );
    }
}
