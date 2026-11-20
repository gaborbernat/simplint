use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::recognize::is_fixture;
use crate::syntax::module_statements;

pub fn unused_fixture(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let project = checker.project();
    // A conftest fixture may override a fixture of a plugin the tests load, which the plugin's own fixtures request.
    if checker.file_name() == "conftest.py" && project.loads_pytest_plugins() {
        return;
    }
    for stmt in module_statements(checker.body()) {
        let Some(function) = stmt.as_function_def_stmt() else {
            continue;
        };
        let Some(name) = fixture_name(function) else {
            continue;
        };
        if !project.is_fixture_requested(name) && !PLUGIN_REQUESTED.contains(&name) {
            checker.report(
                Rule::UnusedFixture,
                function.name.range(),
                format!("Fixture `{name}` is never requested; no test or fixture takes it by name"),
            );
        }
    }
}

// Plugins can request these fixture overrides even when no project test mentions them.
const PLUGIN_REQUESTED: &[&str] = &[
    "anyio_backend",
    "event_loop_policy",
    "django_db_setup",
    "django_db_modify_db_settings",
    "django_db_use_migrations",
    "django_db_keepdb",
    "django_db_createdb",
];

fn fixture_name(function: &ast::StmtFunctionDef) -> Option<&str> {
    function.decorator_list.iter().filter(|decorator| is_fixture(decorator)).find_map(|decorator| {
        let Some(arguments) = decorator.expression.as_call_expr().map(|call| &call.arguments) else {
            return Some(function.name.as_str());
        };
        // An expression for `autouse` may evaluate to true.
        let autouse = arguments.find_keyword("autouse");
        if autouse.is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_none_or(|literal| literal.value)) {
            return None;
        }
        Some(
            arguments
                .find_keyword("name")
                .and_then(|keyword| keyword.value.as_string_literal_expr())
                .map_or(function.name.as_str(), |name| name.value.to_str()),
        )
    })
}
