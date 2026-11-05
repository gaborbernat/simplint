use std::ops::RangeInclusive;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;

pub(super) struct Suite<'a> {
    pub(super) body: &'a [Stmt],
    pub(super) separator: &'static str,
}

// The fix that puts the statements at `expected` in the span's place, with the separator between them. Each statement
// keeps its decorators and the comment lines above it.
pub(super) fn reorder_fix(
    checker: &Checker,
    suite: &Suite,
    expected: &[usize],
    span: RangeInclusive<usize>,
) -> Option<Fix> {
    let source = checker.source;
    let blocks: Vec<TextRange> = suite.body.iter().map(|stmt| statement_block(source, stmt)).collect();
    let moved = &blocks[span.clone()];
    let region = TextRange::new(moved[0].start(), moved[moved.len() - 1].end());
    if checker
        .comment_ranges()
        .comments_in_range(region)
        .iter()
        .any(|comment| !moved.iter().any(|block| block.contains(comment.start())))
        || moved.windows(2).any(|pair| pair[0].end() > pair[1].start())
    {
        return None;
    }
    let text: Vec<String> = expected[span.clone()]
        .iter()
        .map(|index| {
            let block = &source[blocks[*index]];
            if block.ends_with('\n') { block.to_owned() } else { format!("{block}\n") }
        })
        .collect();
    let mut replacement = text.join(suite.separator);
    if !source[region].ends_with('\n') {
        replacement.pop();
    }
    let moved_unsafe = span.filter(|position| expected[*position] != *position).any(|position| {
        matches!(&suite.body[expected[position]], Stmt::ClassDef(_))
            || matches!(&suite.body[expected[position]], Stmt::FunctionDef(function) if !function.decorator_list.is_empty())
    });
    let edit = Edit::range_replacement(replacement, region);
    Some(if moved_unsafe { Fix::unsafe_edit(edit) } else { Fix::safe_edit(edit) })
}

fn statement_block(source: &str, stmt: &Stmt) -> TextRange {
    let start = match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, .. })
        | Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
            decorator_list.first().map_or(stmt.start(), Ranged::start)
        }
        _ => stmt.start(),
    };
    let mut line_start = source.line_start(start);
    // A byte order mark is not a line to step back into.
    while line_start > source.bom_start_offset() {
        let previous = source.line_start(line_start - TextSize::from(1));
        if !source[TextRange::new(previous, line_start)].trim_start().starts_with('#') {
            break;
        }
        line_start = previous;
    }
    TextRange::new(line_start, source.full_line_end(stmt.end()))
}
