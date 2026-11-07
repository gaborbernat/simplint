use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::module_variable::{is_ply_name, uses_ply};
use crate::rules::rename::{kept_message, rename_fix};
use crate::syntax::{Exports, Visibility};

pub fn unexported_public_name(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    let ply = uses_ply(checker);
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
        if Visibility::of(name) != Visibility::Public
            || is_discovered_by_name(name)
            || ply && is_ply_name(name)
            || exported.contains(&name)
            || checker.project().is_used_elsewhere(name)
        {
            continue;
        }
        let private = format!("_{name}");
        // The names of a public module are the API its users import, so a rename breaks them: the author lists the name
        // or renames it by hand.
        if checker.project().role().public {
            checker.report(Rule::UnexportedPublicName, identifier.range(), kept_message(name, &private, false));
            continue;
        }
        // Code that reads the name of a class or function, as `type(self).__name__` or `helper.__name__`, sees the new
        // name once renamed.
        let class = matches!(stmt, Stmt::ClassDef(_));
        let reflected = checker.project().reads_name_of(name) || class && checker.project().reads_class_names();
        let fix = (!reflected).then(|| rename_fix(checker, name, &private)).flatten();
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
