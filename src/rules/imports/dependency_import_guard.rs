use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::{any_over_body, extract_handled_exceptions};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_python_trivia::has_leading_content;
use ruff_python_trivia::textwrap::{dedent, indent};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::imports::stdlib_submodules::STDLIB_SUBMODULES;
use crate::rules::{indentation, is_alone_on_lines};
use crate::syntax::for_each_child_body;

pub fn dependency_import_guard(checker: &Checker) {
    report_guards(checker, checker.body(), None);
}

// `function` is the body of the function the statements belong to, whose locals the module's semantic model does not
// see.
fn report_guards<'a>(checker: &Checker, body: &'a [Stmt], function: Option<&'a [Stmt]>) {
    for stmt in body {
        if let Stmt::Try(stmt) = stmt
            && is_import_guard(checker, stmt)
        {
            let fix = (stmt.orelse.is_empty()
                && stmt.finalbody.is_empty()
                && is_alone_on_lines(checker.source, stmt.range())
                && !shares_line_with_unused_import(checker, &stmt.body, function))
            .then(|| unwrap_try(checker, stmt, function));
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

// `from` imports can fail on a name the module lacks on some platform or version, such as `os.geteuid` or
// `yaml.CSafeLoader`, so their guard serves a present module.
fn imported_modules(stmt: &Stmt) -> Option<Vec<&str>> {
    Some(stmt.as_import_stmt()?.names.iter().map(|alias| alias.name.as_str()).collect())
}

const LATEST_MINOR: u8 = 15;

fn is_required(checker: &Checker, module: &str) -> bool {
    let root = module.split('.').next().unwrap_or_default();
    is_standard_library(checker, module)
        || checker.settings.project.metadata.dependencies.contains(&root.to_ascii_lowercase())
}

fn is_standard_library(checker: &Checker, module: &str) -> bool {
    // Guarding a module that exists on one platform only, or that a minimal or alternative Python build may leave out,
    // is legitimate, so these never count as required.
    const PLATFORM_MODULES: &[&str] = &[
        "_bz2",
        "_ctypes",
        "_curses",
        "_curses_panel",
        "_dbm",
        "_decimal",
        "_gdbm",
        "_hashlib",
        "_lzma",
        "_multiprocessing",
        "_overlapped",
        "_posixshmem",
        "_posixsubprocess",
        "_scproxy",
        "_sqlite3",
        "_ssl",
        "_tkinter",
        "_uuid",
        "_winapi",
        "_wmi",
        "_zstd",
        "bz2",
        "compression.zstd",
        "ctypes",
        "dbm",
        "ensurepip",
        "gzip",
        "idlelib",
        "lzma",
        "mmap",
        "pyexpat",
        "sqlite3",
        "ssl",
        "tkinter",
        "turtle",
        "zlib",
        "asyncio.windows_events",
        "asyncio.windows_utils",
        "crypt",
        "curses",
        "encodings.mbcs",
        "encodings.oem",
        "fcntl",
        "grp",
        "msvcrt",
        "multiprocessing.popen_spawn_win32",
        "multiprocessing.synchronize",
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
        "xml.parsers.expat",
    ];
    let target = checker.settings.from_ruff.target_version.minor;
    let root = module.split_once('.').map_or(module, |(root, _)| root);
    // A module removed in a later version, such as `distutils` in 3.12, is optional for a project that supports both.
    let top_level = (target..=LATEST_MINOR).all(|minor| is_known_standard_library(minor, root));
    // CPython builds its `_test*` extensions for its own test suite, which release builds leave out.
    top_level
        && !root.starts_with("_test")
        && !PLATFORM_MODULES.contains(&root)
        && !PLATFORM_MODULES.contains(&module)
        && (root == module || is_standard_submodule(target, module))
}

// The top-level table cannot say whether a submodule such as `asyncio.taskgroups` ships on the target, so the
// generated table does. A submodule it lacks is never assumed present.
fn is_standard_submodule(target: u8, module: &str) -> bool {
    STDLIB_SUBMODULES.binary_search_by_key(&module, |&(name, _, _)| name).is_ok_and(|index| {
        let (_, added, removed) = STDLIB_SUBMODULES[index];
        added <= target && removed.is_none()
    })
}

// Ruff's unused-import fix would delete an import the unwrapped guard leaves unused, though that import is there to
// fail when the module is missing, so its line keeps a suppression.
const UNUSED_IMPORT_NOTE: &str = "  # noqa: F401  # fails when the module is missing";

fn unwrap_try(checker: &Checker, stmt: &ast::StmtTry, function: Option<&[Stmt]>) -> Fix {
    let source = checker.source;
    let (first, last) = (&stmt.body[0], &stmt.body[stmt.body.len() - 1]);
    let indentation = indentation(source, stmt);
    // A body on the `try:` line, as in `try: import tomllib`, starts after the colon, and the lines it is on hold the
    // `try` too.
    let body = if has_leading_content(first.start(), source) {
        let range = TextRange::new(first.start(), source.full_line_end(last.end()));
        format!("{indentation}{}", annotated(checker, &stmt.body, range, function))
    } else {
        let range = source.full_lines_range(TextRange::new(first.start(), last.end()));
        indent(&dedent(&annotated(checker, &stmt.body, range, function)), indentation).into_owned()
    };
    Fix::unsafe_edit(Edit::range_replacement(body, source.full_lines_range(stmt.range())))
}

// The source of `range` with the suppression appended to each line that holds an import nothing refers to.
fn annotated(checker: &Checker, body: &[Stmt], range: TextRange, function: Option<&[Stmt]>) -> String {
    let source = checker.source;
    let mut annotated = String::with_capacity(range.len().to_usize());
    let mut copied = range.start();
    for stmt in body.iter().filter(|stmt| has_unreferenced_import(checker, stmt, function)) {
        let end = source.line_end(stmt.end());
        annotated.push_str(&source[TextRange::new(copied, end)]);
        annotated.push_str(UNUSED_IMPORT_NOTE);
        copied = end;
    }
    annotated.push_str(&source[TextRange::new(copied, range.end())]);
    annotated
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

// A function-local import is used when the function loads its name anywhere, nested functions included.
fn has_unreferenced_import(checker: &Checker, stmt: &Stmt, function: Option<&[Stmt]>) -> bool {
    stmt.as_import_stmt().is_some_and(|import| {
        import.names.iter().any(|alias| {
            let bound = alias
                .asname
                .as_ref()
                .map_or_else(|| alias.name.as_str().split('.').next().unwrap_or_default(), |name| name.as_str());
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
