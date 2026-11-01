use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::Exports;
use crate::rules::naming::rename_fix;

pub fn unexported_public_name(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    for stmt in checker.body() {
        let identifier = match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, decorator_list, .. })
            | Stmt::ClassDef(ast::StmtClassDef { name, decorator_list, .. })
                if decorator_list.is_empty() =>
            {
                name
            }
            _ => continue,
        };
        let name = identifier.as_str();
        if name.starts_with('_')
            || is_discovered_by_name(name)
            || exported.contains(&name)
            || checker.project().is_referenced(name)
        {
            continue;
        }
        let private = format!("_{name}");
        let fix = rename_fix(checker, name, &private);
        checker.report_fix(
            Rule::UnexportedPublicName,
            identifier.range(),
            format!("`{name}` is not in `__all__`; name it `{private}`"),
            fix,
            format!("Rename `{name}` to `{private}`"),
        );
    }
}

fn is_discovered_by_name(name: &str) -> bool {
    name.starts_with("pytest_") || matches!(name, "load_tests" | "setup_module" | "teardown_module" | "setup")
}
