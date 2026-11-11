use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_trivia::textwrap::dedent;
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, statement_lines};
use crate::syntax::{is_lazy, top_level};

pub fn up_type_checking_lazy_import(checker: &Checker) {
    let body = checker.body();
    for stmt in body {
        let Stmt::If(if_stmt) = stmt else {
            continue;
        };
        // Only `typing`'s name is false at run time for certain: a same-named flag may be a true constant, which runs
        // the imports today.
        if !if_stmt.elif_else_clauses.is_empty()
            || !checker.semantic().resolves_to(&if_stmt.test, &["typing", "TYPE_CHECKING"])
            || !if_stmt.body.iter().all(is_lazy_importable)
        {
            continue;
        }
        checker.report_fix(
            Rule::TypeCheckingLazyImport,
            if_stmt.test.range(),
            "Imports under `if TYPE_CHECKING:` can be lazy imports from Python 3.15; use `lazy` on each one",
            lazy_fix(checker, body, stmt, &if_stmt.body),
            "Make the imports lazy and remove the `TYPE_CHECKING` guard",
        );
    }
}

// `lazy` imports are a statement form, so `from x import *` and `from __future__` have none.
fn is_lazy_importable(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Import(_) => true,
        Stmt::ImportFrom(ast::StmtImportFrom { module, names, .. }) => {
            module.as_ref().is_none_or(|module| module.as_str() != "__future__")
                && names.iter().all(|alias| alias.name.as_str() != "*")
        }
        _ => false,
    }
}

// The lazy import binds the name on first use, so a module the guard hid, such as `_typeshed`, fails there. Comments
// in the block would be lost, so a block holding one is reported without a fix.
fn lazy_fix(checker: &Checker, body: &[Stmt], stmt: &Stmt, imports: &[Stmt]) -> Option<Fix> {
    let runtime_missing = imports.iter().any(|import| {
        matches!(import, Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), .. })
            if top_level(module.as_str()) == "_typeshed")
            || matches!(import, Stmt::Import(ast::StmtImport { names, .. })
                if names.iter().any(|alias| top_level(alias.name.as_str()) == "_typeshed"))
    });
    if runtime_missing
        || checker.has_comment_in(stmt.range())
        || !checker.is_alone_on_lines(stmt.range())
        || !imports.iter().all(|import| checker.is_alone_on_lines(import.range()))
    {
        return None;
    }
    let source = checker.source;
    let lazy = imports
        .iter()
        .map(|import| {
            let prefix = if is_lazy(import) { "" } else { "lazy " };
            format!("{prefix}{}", dedent(&source[source.full_lines_range(import.range())]))
        })
        .collect::<Vec<_>>()
        .concat();
    let replacement = Edit::range_replacement(lazy, source.full_lines_range(stmt.range()));
    // The guard's `TYPE_CHECKING` name is then unused, unless the file spells it out elsewhere.
    let typing_import = (checker.name_token_count("TYPE_CHECKING") == 2)
        .then(|| body.iter().find(|stmt| is_type_checking_import(stmt) && checker.is_alone_on_lines(stmt.range())))
        .flatten()
        .map(|import| Edit::range_deletion(statement_lines(source, import.range(), Blanks::Keep)));
    Some(Fix::applicable_edits(replacement, typing_import, Applicability::Unsafe))
}

fn is_type_checking_import(stmt: &Stmt) -> bool {
    let Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. }) = stmt else {
        return false;
    };
    module.as_str() == "typing"
        && names.len() == 1
        && names[0].asname.is_none()
        && names[0].name.as_str() == "TYPE_CHECKING"
}
