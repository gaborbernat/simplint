use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::semantic::future_import;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return;
    };
    if !checker.semantic().resolves_to(func, &["asyncio", "iscoroutinefunction"]) {
        return;
    }
    checker.report_fix(
        Rule::AsyncioIsCoroutineFunction,
        expr.range(),
        "`asyncio.iscoroutinefunction()` is deprecated since Python 3.14 and also accepts the legacy coroutine marker",
        fix(checker, func),
        "Call `inspect.iscoroutinefunction()`",
    );
}

// The legacy marker of `asyncio.coroutine` counts for `asyncio` only, so the rewrite can change the result.
fn fix(checker: &Checker, func: &Expr) -> Option<Fix> {
    let replacement = Edit::range_replacement("inspect.iscoroutinefunction".to_owned(), func.range());
    if has_plain_inspect_import(checker.body()) {
        return Some(Fix::unsafe_edit(replacement));
    }
    // A name bound to something else would shadow the module the rewrite needs.
    if checker.semantic().is_bound("inspect") {
        return None;
    }
    Some(Fix::unsafe_edits(replacement, [import_edit(checker)]).isolate(IsolationLevel::Group(1)))
}

fn has_plain_inspect_import(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| {
        matches!(stmt, Stmt::Import(import) if import
            .names
            .iter()
            .any(|alias| alias.name.as_str() == "inspect" && alias.asname.is_none()))
    })
}

// The import goes before the first import that sorts after it, as Ruff's isort orders them, or after the last one.
// A module without top-level imports gets it before its first statement after the docstring and future imports.
fn import_edit(checker: &Checker) -> Edit {
    const IMPORT: &str = "import inspect\n";
    let body = checker.body();
    let source = checker.source;
    let preamble = body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count();
    let imports: Vec<(&str, &Stmt)> =
        body[preamble..].iter().map_while(|stmt| imported_module(stmt).map(|module| (module, stmt))).collect();
    if let Some((_, next)) = imports.iter().find(|(module, _)| *module > "inspect") {
        return Edit::insertion(IMPORT.to_owned(), source.line_start(next.start()));
    }
    match imports.last() {
        Some((_, last)) => Edit::insertion(IMPORT.to_owned(), source.full_line_end(last.end())),
        None => Edit::insertion(format!("{IMPORT}\n"), source.line_start(body[preamble].start())),
    }
}

// The first module an import statement names, which is the one that sorts it among the others.
fn imported_module(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::Import(import) => import.names.first().map(|alias| alias.name.as_str()),
        Stmt::ImportFrom(import) => Some(import.module.as_ref().map_or("", ast::Identifier::as_str)),
        _ => None,
    }
}
