use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{last_segment, module_statements};

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
        if !project.is_fixture_requested(name) {
            checker.report(
                Rule::UnusedFixture,
                function.name.range(),
                format!("Fixture `{name}` is never requested; no test or fixture takes it by name"),
            );
        }
    }
}

// The name pytest requests the fixture by, or `None` when the function is no fixture or runs for every test.
fn fixture_name(function: &ast::StmtFunctionDef) -> Option<&str> {
    function.decorator_list.iter().find_map(|decorator| {
        let (func, arguments) = match &decorator.expression {
            Expr::Call(call) => (&*call.func, Some(&call.arguments)),
            expr => (expr, None),
        };
        if last_segment(func) != Some("fixture") {
            return None;
        }
        let Some(arguments) = arguments else {
            return Some(function.name.as_str());
        };
        // Autouse fixtures run for every test, whether or not a test names them, unless `autouse=False`.
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
