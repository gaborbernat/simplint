use ruff_diagnostics::Edit;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::syntax::previous_line_start;

#[derive(Clone, Copy)]
pub(crate) enum Blanks {
    Keep,
    Shared,
    Trailing,
    WiderSide,
}

pub(crate) fn statement_lines(source: &str, range: TextRange, blanks: Blanks) -> TextRange {
    match blanks {
        Blanks::Keep => source.full_lines_range(range),
        Blanks::Shared => lines_with_shared_blanks(source, range),
        Blanks::Trailing => lines_with_trailing_blanks(source, range),
        Blanks::WiderSide => lines_keeping_separation(source, range),
    }
}

pub(crate) fn statement_removal(
    checker: &Checker,
    stmt: TextRange,
    siblings: usize,
    next: Option<TextRange>,
    blanks: Blanks,
) -> Option<Edit> {
    if siblings == 1 {
        return Some(Edit::range_replacement("pass".to_owned(), stmt));
    }
    statement_deletion(checker, stmt, next, blanks).map(Edit::range_deletion)
}

// The text deleting a statement removes: its lines when it has them to itself, with the blank lines `blanks` names. A
// statement sharing its line goes from the `;` that joins it to the statement before it, as in `run(); x = 1`, or up
// to the `next` statement, as in `"""Doc."""; run()`, unless a comment in between would go with it; `ruff format` puts
// joined statements on lines of their own either way.
pub(crate) fn statement_deletion(
    checker: &Checker,
    range: TextRange,
    next: Option<TextRange>,
    blanks: Blanks,
) -> Option<TextRange> {
    let source = checker.source;
    if checker.is_alone_on_lines(range) {
        return Some(statement_lines(source, range, blanks));
    }
    let line_start = source.line_start(range.start());
    let before = source[TextRange::new(line_start, range.start())].trim_end();
    if before.ends_with(';') && source[TextRange::new(range.end(), source.line_end(range.end()))].trim().is_empty() {
        return Some(TextRange::new(line_start + before.text_len() - TextSize::from(1), range.end()));
    }
    let up_to_next = TextRange::new(range.start(), next?.start());
    (!checker.has_comment_in(up_to_next)).then_some(up_to_next)
}

// The lines of a statement, with the blank lines after it that `ruff format` would not keep once it is gone. The
// formatter keeps two blank lines at module level and one in a block, so a statement between blank lines leaves a
// surplus it would close, which `# fmt: off` code keeps.
fn lines_with_shared_blanks(source: &str, range: TextRange) -> TextRange {
    let lines = source.full_lines_range(range);
    let limit = if leading_indentation(source.full_line_str(range.start())).is_empty() { 2 } else { 1 };
    let mut before: usize = 0;
    let mut start = lines.start();
    while start > TextSize::default() && source.full_line_str(previous_line_start(source, start)).trim().is_empty() {
        start = previous_line_start(source, start);
        before += 1;
    }
    let mut after: usize = 0;
    let mut end = lines.end();
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
        after += 1;
    }
    let mut surplus_end = lines.end();
    for _ in 0..(before + after).saturating_sub(limit).min(after) {
        surplus_end = source.full_line_end(surplus_end);
    }
    TextRange::new(lines.start(), surplus_end)
}

// Deleting a statement leaves the blank lines on both sides of it next to each other; drop the shorter run so the code
// around it keeps the wider separation, and keep everything when the statement directly follows code.
fn lines_keeping_separation(source: &str, range: TextRange) -> TextRange {
    let lines = source.full_lines_range(range);
    let before = &source[..lines.start().to_usize()];
    let leading_text = before.trim_end_matches([' ', '\t', '\r', '\n']);
    let leading = before[leading_text.len()..].matches('\n').count().saturating_sub(1);
    let with_trailing = lines_with_trailing_blanks(source, range);
    let trailing = source[lines.end().to_usize()..with_trailing.end().to_usize()].matches('\n').count();
    if leading_text.is_empty() || (leading > 0 && trailing <= leading) {
        with_trailing
    } else if leading > 0 {
        let line_end = leading_text.len() + before[leading_text.len()..].find('\n').map_or(0, |offset| offset + 1);
        TextRange::new(TextSize::try_from(line_end).unwrap_or_default(), lines.end())
    } else {
        lines
    }
}

fn lines_with_trailing_blanks(source: &str, range: TextRange) -> TextRange {
    let mut end = source.full_line_end(range.end());
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
    }
    let mut start = source.line_start(range.start());
    // At the end of the file the blank lines above go too: after a module left with comments only, `ruff format` keeps
    // them, and Ruff reports blank lines ending the file.
    if end.to_usize() == source.len() {
        while start > TextSize::default() && source.full_line_str(previous_line_start(source, start)).trim().is_empty()
        {
            start = previous_line_start(source, start);
        }
    }
    TextRange::new(start, end)
}

pub(crate) fn statement_block(checker: &Checker, stmt: &Stmt) -> TextRange {
    let source = checker.source;
    let start = match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, .. })
        | Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
            decorator_list.first().map_or(stmt.start(), Ranged::start)
        }
        _ => stmt.start(),
    };
    let mut line_start = source.line_start(start);
    // A line inside a string may start with `#` too, so only a comment token above the statement counts. A deeper
    // comment ends the block above it, as the trailing note of a method does.
    let indent = checker.indentation(stmt);
    while let Some(comment) =
        checker.comment_above(TextRange::empty(line_start)).filter(|comment| checker.indentation(*comment) == indent)
    {
        line_start = source.line_start(comment.start());
    }
    TextRange::new(line_start, source.full_line_end(stmt.end()))
}
