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
        for stmt in statements.iter().filter(|stmt| imports_unittest_mock(stmt) && !banned_by_ruff(checker, stmt)) {
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
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. }) => {
            names.iter().any(|alias| is_mock_member(module.as_str(), alias.name.as_str()))
        }
        _ => false,
    }
}

fn is_mock_member(module: &str, member: &str) -> bool {
    match module {
        "unittest.mock" => member != "create_autospec",
        "unittest" => member == "mock",
        _ => false,
    }
}

// Ruff's TID251 reports an import that `banned-api` bans by its module or a parent of it, or by a member's full name,
// which this rule leaves to it.
fn banned_by_ruff(checker: &Checker, stmt: &Stmt) -> bool {
    let banned = &checker.settings.from_ruff.banned_api;
    let banned_module = |module: &str| {
        banned
            .keys()
            .any(|key| module == key || module.strip_prefix(key.as_str()).is_some_and(|rest| rest.starts_with('.')))
    };
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            names.iter().any(|alias| alias.name.as_str() == "unittest.mock" && banned_module(alias.name.as_str()))
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. }) => {
            let module = module.as_str();
            banned_module(module)
                || names.iter().any(|alias| {
                    alias.name.as_str() != "*"
                        && is_mock_member(module, alias.name.as_str())
                        && banned.contains_key(format!("{module}.{}", alias.name).as_str())
                })
        }
        _ => false,
    }
}
