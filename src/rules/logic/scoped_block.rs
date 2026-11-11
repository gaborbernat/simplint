// Helpers for the rules that turn a statement which sets state, then a `try` whose `finally` undoes it, into a `with`
// block.

use ruff_diagnostics::Edit;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;

// The call a `try` runs in its `finally` clause, when that clause holds nothing else and the `try` has no `except` or
// `else`, so leaving the block does nothing but that call.
pub fn finally_call(try_stmt: &ast::StmtTry) -> Option<&ast::ExprCall> {
    let [Stmt::Expr(ast::StmtExpr { value, .. })] = try_stmt.finalbody.as_slice() else {
        return None;
    };
    let Expr::Call(call) = value.as_ref() else {
        return None;
    };
    (try_stmt.handlers.is_empty() && try_stmt.orelse.is_empty() && !try_stmt.is_star).then_some(call)
}

// Whether `target` binds a module-level name that another module of the run reads, through an import, a star import or
// an attribute, or that `__all__` lists. Removing that binding would break those readers, so the report stays without a
// fix. Unlike `Checker::is_exported`, this takes the module-scope binding, counts only the literal strings of `__all__`
// so a dynamic `__all__` does not count, and also counts a name that the signature of an exported name reaches.
pub fn is_exported_module_name(checker: &Checker, target: &ast::ExprName) -> bool {
    let name = target.id.as_str();
    checker.semantic().bindings(name).iter().any(|binding| binding.module_scope && binding.range == target.range())
        && (checker.project().is_used_elsewhere(name)
            || checker.state().export_strings(checker).iter().any(|element| element.value.to_str() == name))
}

// The edits that replace `first` through the `try` header with the line `header`, and delete the `finally` clause.
// Comments in the lines they remove would go with them, so they block the edit.
pub fn wrap_edits(checker: &Checker, first: TextRange, try_stmt: &ast::StmtTry, header: &str) -> Option<[Edit; 2]> {
    let source = checker.source;
    let body_first = try_stmt.body.first()?;
    let body_last = try_stmt.body.last()?;
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
    let tail = TextRange::new(source.full_line_end(body_last.end()), source.full_line_end(try_stmt.end()));
    if checker.has_comment_in(head) || checker.has_comment_in(tail) {
        return None;
    }
    let header = format!("{header}{}", checker.line_ending());
    Some([Edit::range_replacement(header, head), Edit::range_deletion(tail)])
}

// Whether the line before `line_start` ends with a backslash that joins it to the line starting there. A backslash in a
// comment joins nothing, so a line with a comment does not count.
fn continues_previous_line(checker: &Checker, line_start: TextSize) -> bool {
    let newline = line_start.checked_sub(TextSize::new(1)).unwrap_or_default();
    let previous = TextRange::new(checker.source.line_start(newline), line_start);
    checker.text(previous).trim_end().ends_with('\\') && !checker.has_comment_in(previous)
}
