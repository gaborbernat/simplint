use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::api::{listed_exports, references_outside};
use crate::rules::docstrings::Need;
use crate::syntax::{Visibility, module_statements};

pub fn test_only_code(checker: &Checker) {
    let project = checker.project();
    // An `import *` of this module hands its names to code the index cannot follow, as `unused-export` assumes too.
    if checker.is_test_module() || project.is_star_import_source() {
        return;
    }
    let api = checker.state().public_api(checker);
    let listed = listed_exports(checker);
    for stmt in module_statements(checker.body()) {
        // A decorator may register the definition with something outside the run, such as a framework or a plugin list.
        let (name, range, decorated) = match stmt {
            Stmt::FunctionDef(function) => {
                (function.name.as_str(), function.name.range(), !function.decorator_list.is_empty())
            }
            Stmt::ClassDef(class) => (class.name.as_str(), class.name.range(), !class.decorator_list.is_empty()),
            _ => continue,
        };
        if decorated
            || Visibility::of(name) == Visibility::Dunder
            || api.is_public(stmt.range(), Need::Any)
            || listed.contains(&name)
            || !project.is_used_only_by_tests(name)
            || references_outside(checker, name, stmt.range())
        {
            continue;
        }
        checker.report(
            Rule::TestOnlyCode,
            range,
            format!("`{name}` is used only by tests; move it into the tests or a test helper module"),
        );
    }
}
