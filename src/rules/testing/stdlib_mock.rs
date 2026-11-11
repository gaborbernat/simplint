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
        for stmt in statements.iter().filter(|stmt| reports_import(checker, stmt)) {
            checker.report(
                Rule::StdlibMock,
                stmt.range(),
                "The project depends on pytest-mock; use the `mocker` fixture instead of `unittest.mock`",
            );
        }
    });
}

fn is_mock_member(module: &str, member: &str) -> bool {
    match module {
        "unittest.mock" => member != "create_autospec",
        "unittest" => member == "mock",
        _ => false,
    }
}

// Whether the statement imports `unittest.mock` or a member of it that this rule reports. Ruff's TID251 reports an
// import that `banned-api` bans by its module or a parent of it, or by the full name of every mock member it brings
// in, which this rule leaves to it.
fn reports_import(checker: &Checker, stmt: &Stmt) -> bool {
    let banned = &checker.settings.from_ruff.banned_api;
    let banned_module = |module: &str| {
        banned
            .keys()
            .any(|key| module == key || module.strip_prefix(key.as_str()).is_some_and(|rest| rest.starts_with('.')))
    };
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            names.iter().any(|alias| alias.name.as_str() == "unittest.mock") && !banned_module("unittest.mock")
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. }) => {
            let module = module.as_str();
            let members: Vec<&ast::Alias> =
                names.iter().filter(|alias| is_mock_member(module, alias.name.as_str())).collect();
            !members.is_empty()
                && !banned_module(module)
                && !members.iter().all(|alias| {
                    alias.name.as_str() != "*" && banned.contains_key(format!("{module}.{}", alias.name).as_str())
                })
        }
        _ => false,
    }
}
