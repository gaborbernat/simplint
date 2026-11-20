use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::{any_over_body, extract_handled_exceptions};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_trivia::has_leading_content;
use ruff_python_trivia::textwrap::{dedent, indent};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::imports::stdlib::ships_everywhere;
use crate::syntax::{alias_root_binding, for_each_child_body};

pub fn dependency_import_guard(checker: &Checker) {
    report_guards(checker, checker.body(), None);
}

// The module-level semantic model excludes function locals.
fn report_guards<'a>(checker: &Checker, body: &'a [Stmt], function: Option<&'a [Stmt]>) {
    for stmt in body {
        if let Stmt::Try(stmt) = stmt
            && is_import_guard(checker, stmt)
        {
            let fix = (stmt.orelse.is_empty()
                && stmt.finalbody.is_empty()
                && checker.is_alone_on_lines(stmt.range())
                && !checker.has_comment_in(stmt.range())
                && !shares_line_with_unused_import(checker, &stmt.body, function))
            .then(|| unwrap_try(checker, stmt, function));
            let message = if stmt
                .body
                .iter()
                .filter_map(imported_modules)
                .flatten()
                .all(|module| ships_everywhere(checker.target_version().minor, module))
            {
                "Import guard around a standard library module every supported Python ships; import it directly"
            } else {
                "Import guard around a required dependency; import it directly and let a broken install fail"
            };
            checker.report_fix(Rule::DependencyImportGuard, stmt.range(), message, fix, "Remove the import guard");
        }
        let inner = match stmt {
            Stmt::FunctionDef(definition) => Some(definition.body.as_slice()),
            _ => function,
        };
        for_each_child_body(stmt, &mut |child| report_guards(checker, child, inner));
    }
}

fn is_import_guard(checker: &Checker, stmt: &ast::StmtTry) -> bool {
    extract_handled_exceptions(&stmt.handlers).into_iter().any(|name| {
        UnqualifiedName::from_expr(name).is_some_and(|name| {
            matches!(
                name.segments(),
                ["ImportError" | "ModuleNotFoundError"] | ["builtins", "ImportError" | "ModuleNotFoundError"]
            )
        })
    }) && stmt.body.iter().all(|import| {
        imported_modules(import).is_some_and(|modules| modules.iter().all(|module| is_required(checker, module)))
    })
}

// Even an installed module can lack the requested name, such as `os.geteuid` outside POSIX.
fn imported_modules(stmt: &Stmt) -> Option<Vec<&str>> {
    Some(stmt.as_import_stmt()?.names.iter().map(|alias| alias.name.as_str()).collect())
}

fn is_required(checker: &Checker, module: &str) -> bool {
    ships_everywhere(checker.target_version().minor, module) || checker.settings.project.metadata.requires(module)
}

// Ruff's cleanup splits statements that share a line before it removes an import, so a suppression at the end of a
// shared line would cover only one of them, and the others would lose theirs.
fn shares_line_with_unused_import(checker: &Checker, body: &[Stmt], function: Option<&[Stmt]>) -> bool {
    body.windows(2).any(|pair| {
        !checker.source[TextRange::new(pair[0].end(), pair[1].start())].contains('\n')
            && (has_unreferenced_import(checker, &pair[0], function)
                || has_unreferenced_import(checker, &pair[1], function))
    })
}

fn has_unreferenced_import(checker: &Checker, stmt: &Stmt, function: Option<&[Stmt]>) -> bool {
    stmt.as_import_stmt().is_some_and(|import| {
        import.names.iter().any(|alias| {
            let bound = alias_root_binding(alias);
            match function {
                Some(body) => !any_over_body(
                    body,
                    |expr| matches!(expr, Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, .. }) if id == bound),
                ),
                None => checker.semantic().references_to(bound).is_empty(),
            }
        })
    })
}

fn unwrap_try(checker: &Checker, stmt: &ast::StmtTry, function: Option<&[Stmt]>) -> Fix {
    let source = checker.source;
    let (first, last) = (&stmt.body[0], &stmt.body[stmt.body.len() - 1]);
    let indentation = checker.indentation(stmt);
    // An inline `try: import tomllib` shares its first line with the header.
    let body = if has_leading_content(first.start(), source) {
        let range = TextRange::new(first.start(), source.full_line_end(last.end()));
        format!("{indentation}{}", annotated(checker, &stmt.body, range, function))
    } else {
        let range = source.full_lines_range(TextRange::new(first.start(), last.end()));
        indent(&dedent(&annotated(checker, &stmt.body, range, function)), indentation).into_owned()
    };
    Fix::unsafe_edit(Edit::range_replacement(body, source.full_lines_range(stmt.range())))
}

fn annotated(checker: &Checker, body: &[Stmt], range: TextRange, function: Option<&[Stmt]>) -> String {
    let source = checker.source;
    let mut annotated = String::with_capacity(range.len().to_usize());
    let mut copied = range.start();
    for stmt in body.iter().filter(|stmt| has_unreferenced_import(checker, stmt, function)) {
        let end = source.line_end(stmt.end());
        annotated.push_str(&source[TextRange::new(copied, end)]);
        // Keep Ruff from removing the import that exposes a broken installation.
        annotated.push_str("  # noqa: F401  # fails when the module is missing");
        copied = end;
    }
    annotated.push_str(&source[TextRange::new(copied, range.end())]);
    annotated
}
