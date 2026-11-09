use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::for_each_body;

pub fn stdlib_mock(checker: &Checker) {
    if !checker.is_test_module() || !checker.settings.project.metadata.declared.contains("pytest_mock") {
        return;
    }
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements.iter().filter(|stmt| imports_unittest_mock(stmt)) {
            checker.report(
                Rule::StdlibMock,
                stmt.range(),
                "The project depends on pytest-mock; use the `mocker` fixture instead of `unittest.mock`",
            );
        }
    });
}

fn imports_unittest_mock(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => names.iter().any(|alias| alias.name.as_str() == "unittest.mock"),
        Stmt::ImportFrom(ast::StmtImportFrom { module, names, level, .. }) if *level == 0 => {
            match module.as_ref().map(ruff_python_ast::Identifier::as_str) {
                Some("unittest.mock") => names.iter().any(|alias| alias.name.as_str() != "create_autospec"),
                Some("unittest") => names.iter().any(|alias| alias.name.as_str() == "mock"),
                _ => false,
            }
        }
        _ => false,
    }
}
