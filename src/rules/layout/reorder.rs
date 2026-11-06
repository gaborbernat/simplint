use std::ops::RangeInclusive;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::definition_time::calls_at_definition;
use crate::lint::checker::Checker;
use crate::syntax::last_segment;

pub(super) struct Suite<'a> {
    pub(super) body: &'a [Stmt],
    pub(super) separator: &'static str,
}

// The positions a reordering moves: from the first to the last statement that `expected` puts elsewhere.
pub(super) fn moved_span(expected: &[usize]) -> Option<RangeInclusive<usize>> {
    let low = (0..expected.len()).find(|&position| expected[position] != position)?;
    let high = (0..expected.len()).rfind(|&position| expected[position] != position).unwrap_or(low);
    Some(low..=high)
}

// The fix that puts the statements at `expected` in the span's place, with the separator between them. Each statement
// keeps its decorators and the comment lines above it. `shifts_unsafely` tells the statements whose move is unsafe.
pub(super) fn reorder_fix(
    checker: &Checker,
    suite: &Suite,
    expected: &[usize],
    span: RangeInclusive<usize>,
    shifts_unsafely: fn(&Stmt) -> bool,
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
    let moved_unsafe = span
        .filter(|position| expected[*position] != *position)
        .any(|position| shifts_unsafely(&suite.body[expected[position]]));
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

// Decorators with no effect of their own, so moving a decorated method past another only changes the order of
// evaluations that cannot be observed.
const PURE_DECORATORS: [&str; 8] =
    ["property", "cached_property", "classmethod", "staticmethod", "overload", "setter", "getter", "deleter"];

// Whether moving a definition is unsafe, `None` for a statement that is not one. A function moves safely when its
// decorators are all pure, such as `@property`, and its defaults call nothing; a class never does. A call decorator
// such as `@cache()` has no last name here, so it counts as impure.
pub(super) fn moves_definition_unsafely(stmt: &Stmt) -> Option<bool> {
    match stmt {
        Stmt::FunctionDef(function) => Some(
            function.decorator_list.iter().any(|decorator| {
                !last_segment(&decorator.expression).is_some_and(|name| PURE_DECORATORS.contains(&name))
            }) || calls_at_definition(function),
        ),
        Stmt::ClassDef(_) => Some(true),
        _ => None,
    }
}
