use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;

pub fn multiline_module_docstring(checker: &Checker) {
    // A public module of a library introduces its API in its docstring, listing what it exports, as PEP 257 asks.
    if checker.is_test_module() || checker.project().role().public {
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
