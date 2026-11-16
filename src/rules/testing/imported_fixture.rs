use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::recognize::is_fixture;
use crate::syntax::module_statements;

pub fn imported_fixture(checker: &Checker) {
    let project = checker.project();
    for stmt in module_statements(checker.body()) {
        let Some(function) =
            stmt.as_function_def_stmt().filter(|function| function.decorator_list.iter().any(is_fixture))
        else {
            continue;
        };
        let name = function.name.as_str();
        let how = if project.is_imported_by_others(name) {
            "another module imports it"
        } else if checker.dunder_all().lists(name) {
            "`__all__` exports it"
        } else {
            continue;
        };
        checker.report(
            Rule::ImportedFixture,
            function.name.range(),
            format!(
                "Fixture `{name}` is shared by import, as {how}; share it from a `conftest.py` or a plugin instead"
            ),
        );
    }
}
