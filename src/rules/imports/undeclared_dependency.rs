use std::collections::BTreeSet;

use ruff_python_ast::helpers::{any_over_expr, extract_handled_exceptions};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{is_stdlib_root, is_type_checking_name, top_level};

pub fn undeclared_dependency(checker: &Checker) {
    let settings = &checker.settings.project;
    let metadata = &settings.metadata;
    // Only the shipped packages run where the requirements alone are installed; tests and tools run beside the
    // development dependencies.
    let (Some(packages), Some(module_path)) = (&settings.api.packages, checker.module_path()) else {
        return;
    };
    // A module the package never imports itself, such as a plugin for another tool or an optional integration, runs
    // only where its user installed what it imports.
    if !metadata.lists_dependencies
        || checker.is_test_module()
        || !checker.project().is_package_loaded()
        || !module_path.first().is_some_and(|root| packages.contains(root))
    {
        return;
    }
    let mut imports = Imports::default();
    imports.visit_body(checker.body());
    let project = checker.project();
    for (module, range) in imports.found {
        let root = top_level(module);
        if root == "__future__"
            || is_stdlib(module)
            || packages.contains(root)
            || project.imports_run_file(module)
            || is_declared(&metadata.runtime, module)
        {
            continue;
        }
        checker.report(
            Rule::UndeclaredDependency,
            range,
            format!("`{root}` is imported, but no requirement or extra of the project installs it; declare its distribution"),
        );
    }
}

// A distribution's import name is its normalized name in most cases, so a namespace package such as `google.auth`
// matches `google-auth`, and one whose name adds a prefix or suffix, such as `grpcio` or `pyserial`, contains it.
pub(super) fn is_declared(runtime: &BTreeSet<String>, module: &str) -> bool {
    let lower = module.to_ascii_lowercase();
    let mut prefix = String::new();
    for part in lower.split('.') {
        if !prefix.is_empty() {
            prefix.push('_');
        }
        prefix.push_str(part);
        if runtime.contains(&prefix) {
            return true;
        }
    }
    let root = top_level(&lower);
    runtime.iter().any(|name| name.contains(root))
}

// The absolute imports Python runs unguarded when it loads the module: an import under `if TYPE_CHECKING:` never runs,
// one in a `try` that handles `ImportError` is optional by design, and one under a check of the Python version or the
// platform runs only where that environment provides it.
#[derive(Default)]
struct Imports<'a> {
    // Each module with the range to report.
    found: Vec<(&'a str, TextRange)>,
}

impl<'a> Visitor<'a> for Imports<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => {
                self.found.extend(names.iter().map(|alias| (alias.name.as_str(), alias.range())));
            }
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                self.found.push((module.as_str(), module.range()));
            }
            // A function imports when it runs, which an optional integration relies on to import only when used.
            Stmt::FunctionDef(_) => {}
            Stmt::If(ast::StmtIf { test, elif_else_clauses, .. }) if is_type_checking_name(test) => {
                for clause in elif_else_clauses {
                    self.visit_body(&clause.body);
                }
            }
            // The handlers hold the fallback the optional import falls back to.
            Stmt::Try(ast::StmtTry { handlers, orelse, finalbody, .. }) if handles_import_error(handlers) => {
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            Stmt::If(ast::StmtIf { test, .. }) if any_over_expr(test, &is_environment) => {}
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

fn handles_import_error(handlers: &[ast::ExceptHandler]) -> bool {
    handlers.iter().any(|handler| {
        let ast::ExceptHandler::ExceptHandler(handler) = handler;
        handler.type_.is_none()
    }) || extract_handled_exceptions(handlers).into_iter().any(|name| {
        UnqualifiedName::from_expr(name).is_some_and(|name| {
            matches!(
                name.segments().last(),
                Some(&("ImportError" | "ModuleNotFoundError" | "Exception" | "BaseException"))
            )
        })
    })
}

// A module the standard library ships on any Python version, since a version check may pick it, as `tomllib` on 3.11.
pub(super) fn is_stdlib(module: &str) -> bool {
    (7..=15).any(|minor| is_stdlib_root(minor, module))
}

fn is_environment(expr: &Expr) -> bool {
    let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = expr else {
        return false;
    };
    let Some(module) = value.as_name_expr().map(|name| name.id.as_str()) else {
        return false;
    };
    matches!((module, attr.as_str()), ("sys", "version_info" | "platform" | "implementation") | ("os", "name") | ("platform", _))
}
