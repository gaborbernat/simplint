use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_python_trivia::textwrap::dedent;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::project::package_parts;
use crate::registry::Rule;
use crate::rules::{delete_statement, docstring, for_each_body, indentation, is_alone_on_lines};

pub fn dependency_import_guard(checker: &mut Checker) {
    let mut found = Vec::new();
    for_each_body(checker.body(), &mut |statements, _| {
        found.extend(statements.iter().filter_map(Stmt::as_try_stmt).filter(|stmt| is_import_guard(checker, stmt)));
    });
    for stmt in found {
        let fix =
            (stmt.orelse.is_empty() && stmt.finalbody.is_empty() && is_alone_on_lines(checker.source, stmt.range()))
                .then(|| unwrap_try(checker, stmt));
        checker.report(
            Rule::DependencyImportGuard,
            stmt.range(),
            "Import guard around a required dependency; import it directly and let a broken install fail",
            fix,
        );
    }
}

fn is_import_guard(checker: &Checker, stmt: &ast::StmtTry) -> bool {
    stmt.handlers.iter().any(|ast::ExceptHandler::ExceptHandler(handler)| {
        handler.type_.as_deref().is_some_and(|type_| {
            let names: Vec<&Expr> = match type_ {
                Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().collect(),
                other => vec![other],
            };
            names.iter().any(|name| {
                UnqualifiedName::from_expr(name)
                    .is_some_and(|name| matches!(name.segments(), ["ImportError" | "ModuleNotFoundError"]))
            })
        })
    }) && stmt.body.iter().all(|import| {
        imported_modules(import).is_some_and(|modules| modules.iter().all(|module| is_required(checker, module)))
    })
}

fn imported_modules(stmt: &Stmt) -> Option<Vec<&str>> {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => Some(names.iter().map(|alias| alias.name.as_str()).collect()),
        Stmt::ImportFrom(ast::StmtImportFrom { module, level: 0, .. }) => Some(vec![module.as_ref()?.as_str()]),
        Stmt::ImportFrom(_) => Some(Vec::new()),
        _ => None,
    }
}

fn is_required(checker: &Checker, module: &str) -> bool {
    // Guarding a module that exists on one platform only is legitimate, so these never count as required.
    const PLATFORM_MODULES: &[&str] = &[
        "_winapi",
        "crypt",
        "curses",
        "fcntl",
        "grp",
        "msvcrt",
        "nis",
        "nt",
        "ossaudiodev",
        "posix",
        "pty",
        "pwd",
        "readline",
        "resource",
        "spwd",
        "syslog",
        "termios",
        "tty",
        "winreg",
        "winsound",
    ];
    let root = module.split('.').next().unwrap_or_default();
    (is_known_standard_library(checker.settings.target_version.minor, root) && !PLATFORM_MODULES.contains(&root))
        || checker.settings.dependencies.contains(&root.to_ascii_lowercase())
        || package_parts(checker.path).first().is_some_and(|package| package == root)
}

fn unwrap_try(checker: &Checker, stmt: &ast::StmtTry) -> Fix {
    let source = checker.source;
    let body =
        &source[source.full_lines_range(TextRange::new(stmt.body[0].start(), stmt.body[stmt.body.len() - 1].end()))];
    Fix::unsafe_edit(Edit::range_replacement(
        indent(&dedent(body), indentation(source, stmt)),
        source.full_lines_range(stmt.range()),
    ))
}

fn indent(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| if line.trim().is_empty() { "\n".to_owned() } else { format!("{prefix}{line}\n") })
        .collect()
}

pub fn future_annotations(checker: &mut Checker) {
    if checker.settings.target_version < PythonVersion::PY314 {
        return;
    }
    let body = checker.body();
    for stmt in body {
        let Stmt::ImportFrom(import) = stmt else {
            continue;
        };
        if import.module.as_ref().is_none_or(|module| module.as_str() != "__future__") {
            continue;
        }
        let Some(alias) = import.names.iter().find(|alias| alias.name.as_str() == "annotations") else {
            continue;
        };
        let edit = if import.names.len() == 1 {
            delete_statement(checker.source, stmt, body)
        } else {
            let kept: Vec<&str> = import
                .names
                .iter()
                .filter(|other| other.name.as_str() != "annotations")
                .map(|other| checker.text(other.range()))
                .collect();
            Edit::range_replacement(format!("from __future__ import {}", kept.join(", ")), import.range())
        };
        checker.report(
            Rule::FutureAnnotations,
            alias.range(),
            "Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`",
            Some(Fix::safe_edit(edit)),
        );
    }
}

pub fn init_module_content(checker: &mut Checker) {
    if !checker.is_init_module() {
        return;
    }
    let body = checker.body();
    for stmt in &body[usize::from(docstring(body).is_some())..] {
        checker.report(
            Rule::InitModuleContent,
            stmt.range(),
            "`__init__.py` holds code; keep only its docstring",
            None,
        );
    }
}

pub fn absolute_sibling_import(checker: &mut Checker) {
    let current = package_parts(checker.path).join(".");
    if current.is_empty() {
        return;
    }
    let mut found = Vec::new();
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements {
            if let Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) = stmt {
                let name = module.as_str();
                let relative = if name == current {
                    Some(".".to_owned())
                } else {
                    name.strip_prefix(&current).and_then(|rest| rest.strip_prefix('.')).map(|rest| format!(".{rest}"))
                };
                if let Some(relative) = relative {
                    found.push((module.range(), relative));
                }
            }
        }
    });
    for (range, relative) in found {
        checker.report(
            Rule::AbsoluteSiblingImport,
            range,
            format!("Import from the same package relatively: `from {relative} import ...`"),
            Some(Fix::safe_edit(Edit::range_replacement(relative, range))),
        );
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::DependencyImportGuard, "SMP040.py", PythonVersion::PY314 ; "dependency_import_guard")]
    #[test_case(Rule::DependencyImportGuard, "SMP040.py", PythonVersion::PY310 ; "dependency_import_guard_py310")]
    #[test_case(Rule::FutureAnnotations, "SMP108.py", PythonVersion::PY314 ; "future_annotations")]
    #[test_case(Rule::FutureAnnotations, "SMP108.py", PythonVersion::PY310 ; "future_annotations_py310")]
    #[test_case(Rule::InitModuleContent, "package/__init__.py", PythonVersion::PY314 ; "init_module_content")]
    #[test_case(Rule::InitModuleContent, "package/sub/__init__.py", PythonVersion::PY314 ; "init_module_content_docstring_only")]
    #[test_case(Rule::InitModuleContent, "SMP003_ok.py", PythonVersion::PY314 ; "init_module_content_regular_module")]
    #[test_case(Rule::AbsoluteSiblingImport, "package/sub/module.py", PythonVersion::PY314 ; "absolute_sibling_import")]
    #[test_case(Rule::AbsoluteSiblingImport, "package/__init__.py", PythonVersion::PY314 ; "absolute_sibling_import_init")]
    #[test_case(Rule::AbsoluteSiblingImport, "SMP003_ok.py", PythonVersion::PY314 ; "absolute_sibling_import_outside_package")]
    fn rule(rule: Rule, fixture: &str, target_version: PythonVersion) {
        check(rule, fixture, target_version);
    }
}
