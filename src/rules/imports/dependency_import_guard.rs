use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::extract_handled_exceptions;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_python_trivia::has_leading_content;
use ruff_python_trivia::textwrap::{dedent, indent};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{for_each_body, indentation, is_alone_on_lines};

pub fn dependency_import_guard(checker: &Checker) {
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements.iter().filter_map(Stmt::as_try_stmt).filter(|stmt| is_import_guard(checker, stmt)) {
            let fix = (stmt.orelse.is_empty()
                && stmt.finalbody.is_empty()
                && is_alone_on_lines(checker.source, stmt.range()))
            .then(|| unwrap_try(checker, stmt));
            let message = if stmt
                .body
                .iter()
                .filter_map(imported_modules)
                .flatten()
                .all(|module| is_standard_library(checker, module))
            {
                "Import guard around a standard library module every supported Python ships; import it directly"
            } else {
                "Import guard around a required dependency; import it directly and let a broken install fail"
            };
            checker.report_fix(Rule::DependencyImportGuard, stmt.range(), message, fix, "Remove the import guard");
        }
    });
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

fn imported_modules(stmt: &Stmt) -> Option<Vec<&str>> {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => Some(names.iter().map(|alias| alias.name.as_str()).collect()),
        // A relative import (level above 0) reaches the project's own modules, which may be optional, such as a
        // compiled speedup, so only absolute imports can name a required dependency.
        Stmt::ImportFrom(ast::StmtImportFrom { module, level: 0, .. }) => Some(vec![module.as_ref()?.as_str()]),
        _ => None,
    }
}

const LATEST_MINOR: u8 = 15;

fn is_required(checker: &Checker, module: &str) -> bool {
    let root = module.split('.').next().unwrap_or_default();
    is_standard_library(checker, root)
        || checker.settings.project.metadata.dependencies.contains(&root.to_ascii_lowercase())
}

fn is_standard_library(checker: &Checker, module: &str) -> bool {
    // Guarding a module that exists on one platform only, or that a minimal or alternative Python build may leave out,
    // is legitimate, so these never count as required.
    const PLATFORM_MODULES: &[&str] = &[
        "_bz2",
        "_ctypes",
        "_lzma",
        "_sqlite3",
        "_ssl",
        "_tkinter",
        "_winapi",
        "bz2",
        "ctypes",
        "dbm",
        "ensurepip",
        "idlelib",
        "lzma",
        "sqlite3",
        "ssl",
        "tkinter",
        "turtle",
        "zlib",
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
    // A module removed in a later version, such as `distutils` in 3.12, is optional for a project that supports both.
    (checker.settings.from_ruff.target_version.minor..=LATEST_MINOR).all(|minor| is_known_standard_library(minor, root))
        && !PLATFORM_MODULES.contains(&root)
}

fn unwrap_try(checker: &Checker, stmt: &ast::StmtTry) -> Fix {
    let source = checker.source;
    let (first, last) = (&stmt.body[0], &stmt.body[stmt.body.len() - 1]);
    let indentation = indentation(source, stmt);
    // A body on the `try:` line, as in `try: import tomllib`, starts after the colon, and the lines it is on hold the
    // `try` too.
    let body = if has_leading_content(first.start(), source) {
        format!("{indentation}{}", &source[TextRange::new(first.start(), source.full_line_end(last.end()))])
    } else {
        indent(&dedent(&source[source.full_lines_range(TextRange::new(first.start(), last.end()))]), indentation)
            .into_owned()
    };
    Fix::unsafe_edit(Edit::range_replacement(body, source.full_lines_range(stmt.range())))
}
