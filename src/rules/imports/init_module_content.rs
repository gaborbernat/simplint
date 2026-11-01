use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn init_module_content(checker: &Checker) {
    if !checker.is_init_module() {
        return;
    }
    for stmt in body_without_leading_docstring(checker.body()) {
        checker.report(Rule::InitModuleContent, stmt.range(), "`__init__.py` holds code; keep only its docstring");
    }
}
