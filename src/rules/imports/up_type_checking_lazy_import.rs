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
        // A local flag named `TYPE_CHECKING` can be true at runtime.
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

// `_typeshed` is unavailable at runtime. Making its import lazy would leave an error waiting for the first use.
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
