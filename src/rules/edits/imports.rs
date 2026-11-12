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

// The import a fix that writes `member` of `module`, or `module` itself, needs. `None` when another binding holds the
// name at `at`, the place the fix writes it, which leaves the fix off.
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

// The fix of an edit that uses an import. An edit that adds the import is isolated, so the next pass sees the import
// instead of adding it twice.
pub(crate) fn with_import(edit: Edit, import: Import, applicability: Applicability) -> Fix {
    match import {
        Import::Present => Fix::applicable_edit(edit, applicability),
        Import::Add(import) => Fix::applicable_edits(edit, [import], applicability).isolate(IsolationLevel::Group(1)),
    }
}

// The import goes where Ruff's isort keeps it: in the standard library section of the leading import block, plain
// `import x` lines before `from x import y` lines, each group in module order. A new section gets a blank line
// after it.
fn import_insertion(checker: &Checker, module: &str, member: Option<&str>) -> Option<Edit> {
    let body = checker.body();
    let source = checker.source;
    let eol = checker.line_ending();
    let text = match member {
        Some(member) => format!("from {module} import {member}{eol}"),
        None => format!("import {module}{eol}"),
    };
    let key = (member.is_some(), module.to_ascii_lowercase());
    let preamble = preamble_len(body);
    let block = body[preamble..].iter().take_while(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)));
    let minor = checker.target_version().minor;
    let stdlib: Vec<(&Stmt, (bool, String))> = block
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
    let (line, insertion) = if let Some((next, _)) = stdlib.iter().find(|(_, sort)| *sort > key) {
        (above_comments(checker, next), text)
    } else if let Some((last, _)) = stdlib.last() {
        let line_end = source.full_line_end(last.end());
        let rest = source[TextRange::new(last.end(), line_end)].trim();
        // Code after the import on its line would run before the new import lands below it.
        if !rest.is_empty() && !rest.starts_with('#') {
            return None;
        }
        (line_end, text)
    } else {
        // The statement the import goes before follows the preamble, so the body has one there.
        (above_comments(checker, &body[preamble]), format!("{text}{eol}"))
    };
    // A line that starts inside the preamble's last statement would put the import above a docstring sharing its line,
    // which then stops being `__doc__`, or above a `from __future__` import, which is a syntax error.
    if preamble > 0 && line < body[preamble - 1].end() {
        return None;
    }
    Some(Edit::insertion(insertion, line))
}

// The statements before a module's imports: its docstring and `from __future__` imports.
pub(crate) fn preamble_len(body: &[Stmt]) -> usize {
    body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count()
}

// The line an import goes on above `stmt`: above the comment lines directly over it, unless they start the file,
// where a shebang, encoding or license header stays put.
pub(crate) fn above_comments(checker: &Checker, stmt: &Stmt) -> TextSize {
    let source = checker.source;
    let mut line_start = source.line_start(stmt.start());
    while checker.comment_above(TextRange::empty(line_start)).is_some() {
        line_start = previous_line_start(source, line_start);
    }
    if line_start > source.bom_start_offset() { line_start } else { source.line_start(stmt.start()) }
}
