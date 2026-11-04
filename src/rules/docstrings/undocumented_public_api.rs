use ruff_linter::fs::relativize_path;
use ruff_python_ast::{self as ast, Stmt};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;
use crate::rules::docstrings::{ApiKind, is_documented_variable};

pub fn undocumented_public_api(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let api = checker.public_api();
    for item in &api.items {
        let documented = match item.stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) | Stmt::ClassDef(ast::StmtClassDef { body, .. }) => {
                docstring(body).is_some()
            }
            _ => is_documented_variable(checker, item.stmt, item.next),
        };
        if documented {
            continue;
        }
        let name = &item.name;
        let reason = item.via.map_or_else(String::new, |via| format!("; `{}` re-exports it", relativize_path(via)));
        let message = match item.kind {
            ApiKind::Variable => format!(
                "Public variable `{name}` is undocumented; add a docstring below it or a `#:` comment above it{reason}"
            ),
            ApiKind::Function => {
                format!("Public function `{name}` has no docstring; document it for its users{reason}")
            }
            ApiKind::Class => format!("Public class `{name}` has no docstring; document it for its users{reason}"),
            ApiKind::Method => format!("Public method `{name}` has no docstring; document it for its users{reason}"),
            ApiKind::Property => {
                format!("Public property `{name}` has no docstring; document it for its users{reason}")
            }
        };
        checker.report(Rule::UndocumentedPublicApi, item.range, message);
    }
}
