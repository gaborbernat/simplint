use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::semantic::future_import;
use crate::syntax::{is_stdlib_root, previous_line_start};

#[derive(Clone)]
pub(crate) enum Import {
    Present,
    Add(Edit),
}

// A local binding can shadow a top-level import at the point where a fix uses it.
pub(crate) fn ensure_import(checker: &Checker, module: &str, member: Option<&str>, at: TextRange) -> Option<Import> {
    let binding = member.unwrap_or(module);
    if checker.semantic().is_bound_around(binding, at) {
        return None;
    }
    if checker.state().top_level_imports(checker).contains(&(module, member)) {
        return Some(Import::Present);
    }
    if checker.semantic().is_bound(binding) {
        return None;
    }
    Some(Import::Add(import_insertion(checker, module, member)?))
}

// Apply one import-adding fix per pass so the next fix can reuse the import.
pub(crate) fn with_import(edit: Edit, import: Import, applicability: Applicability) -> Fix {
    match import {
        Import::Present => Fix::applicable_edit(edit, applicability),
        Import::Add(import) => Fix::applicable_edits(edit, [import], applicability).isolate(IsolationLevel::Group(1)),
    }
}

// Match the default standard-library import order to avoid a second edit from Ruff.
fn import_insertion(checker: &Checker, module: &str, member: Option<&str>) -> Option<Edit> {
    let body = checker.body();
    let eol = checker.line_ending();
    let text = match member {
        Some(member) => format!("from {module} import {member}{eol}"),
        None => format!("import {module}{eol}"),
    };
    let preamble = preamble_len(body);
    let minor = checker.target_version().minor;
    let stdlib: Vec<(&Stmt, (bool, String))> = body[preamble..]
        .iter()
        .take_while(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)))
        .filter_map(|stmt| match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => names
                .iter()
                .all(|alias| is_stdlib_root(minor, alias.name.as_str()))
                .then(|| (stmt, (false, names[0].name.to_ascii_lowercase()))),
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                is_stdlib_root(minor, module.as_str()).then(|| (stmt, (true, module.to_ascii_lowercase())))
            }
            _ => None,
        })
        .collect();
    let key = (member.is_some(), module.to_ascii_lowercase());
    let (line, insertion) = if let Some((next, _)) = stdlib.iter().find(|(_, sort)| *sort > key) {
        (above_comments(checker, next), text)
    } else if let Some((last, _)) = stdlib.last() {
        let source = checker.source;
        let line_end = source.full_line_end(last.end());
        let rest = source[TextRange::new(last.end(), line_end)].trim();
        // A statement on the same line might need the new import before it runs.
        if !rest.is_empty() && !rest.starts_with('#') {
            return None;
        }
        (line_end, text)
    } else {
        // The statement being fixed guarantees there is code after the preamble.
        (above_comments(checker, &body[preamble]), format!("{text}{eol}"))
    };
    // A shared line can put the insertion before the module docstring or a future import. Both must stay ahead of
    // ordinary imports.
    if preamble > 0 && line < body[preamble - 1].end() {
        return None;
    }
    Some(Edit::insertion(insertion, line))
}

pub(crate) fn preamble_len(body: &[Stmt]) -> usize {
    body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count()
}

// Keep comments with the statement below them, except for a header at the start of the file.
pub(crate) fn above_comments(checker: &Checker, stmt: &Stmt) -> TextSize {
    let source = checker.source;
    let mut line_start = source.line_start(stmt.start());
    while checker.comment_above(TextRange::empty(line_start)).is_some() {
        line_start = previous_line_start(source, line_start);
    }
    if line_start > source.bom_start_offset() { line_start } else { source.line_start(stmt.start()) }
}
