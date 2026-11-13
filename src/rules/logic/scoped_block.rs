// Helpers for the rules that turn a `try`/`finally` pair into a `with` block.

use ruff_diagnostics::Edit;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;

// Leaving the block must do nothing but this call, which the `with` then takes over.
pub(super) fn finally_call(try_stmt: &ast::StmtTry) -> Option<&ast::ExprCall> {
    let [Stmt::Expr(ast::StmtExpr { value, .. })] = try_stmt.finalbody.as_slice() else {
        return None;
    };
    let Expr::Call(call) = value.as_ref() else {
        return None;
    };
    (try_stmt.handlers.is_empty() && try_stmt.orelse.is_empty() && !try_stmt.is_star).then_some(call)
}

// Unlike `Checker::is_exported`, this counts only the literal strings of `__all__`, and also the names an exported
// signature reaches.
pub(super) fn is_exported_module_name(checker: &Checker, target: &ast::ExprName) -> bool {
    let name = target.id.as_str();
    checker.semantic().bindings(name).iter().any(|binding| binding.module_scope && binding.range == target.range())
        && (checker.project().is_used_elsewhere(name)
            || checker.state().export_strings(checker).iter().any(|element| element.value.to_str() == name))
}

// Comments in the lines the edits remove would go with them, so they block the edit.
pub(super) fn wrap_edits(
    checker: &Checker,
    first: TextRange,
    try_stmt: &ast::StmtTry,
    header: &str,
) -> Option<[Edit; 2]> {
    let source = checker.source;
    let body_first = &try_stmt.body[0];
    // A body that shares the `try` line has no line of its own to keep.
    if source.line_start(body_first.start()) <= try_stmt.start() {
        return None;
    }
    let line_start = source.line_start(first.start());
    // A statement that shares its line with an earlier one would keep that statement's text before the `with`.
    if !checker.text(TextRange::new(line_start, first.start())).trim().is_empty()
        || continues_previous_line(checker, line_start)
    {
        return None;
    }
    let head = TextRange::new(first.start(), source.line_start(body_first.start()));
    let tail = TextRange::new(
        source.full_line_end(try_stmt.body[try_stmt.body.len() - 1].end()),
        source.full_line_end(try_stmt.end()),
    );
    if checker.has_comment_in(head) || checker.has_comment_in(tail) {
        return None;
    }
    Some([Edit::range_replacement(format!("{header}{}", checker.line_ending()), head), Edit::range_deletion(tail)])
}

// A backslash in a comment joins nothing, so a line with a comment does not count.
fn continues_previous_line(checker: &Checker, line_start: TextSize) -> bool {
    let newline = line_start.checked_sub(TextSize::new(1)).unwrap_or_default();
    let previous = TextRange::new(checker.source.line_start(newline), line_start);
    checker.text(previous).trim_end().ends_with('\\') && !checker.has_comment_in(previous)
}
