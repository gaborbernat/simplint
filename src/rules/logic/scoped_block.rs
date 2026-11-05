// Helpers for the rules that turn a statement which sets state, then a `try` whose `finally` undoes it, into a `with`
// block.

use ruff_diagnostics::Edit;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange};

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

// The one positional argument of a call, when it has no keyword arguments.
pub fn sole_argument(call: &ast::ExprCall) -> Option<&Expr> {
    match call.arguments.args.as_ref() {
        [argument] if call.arguments.keywords.is_empty() => Some(argument),
        _ => None,
    }
}

// The range of the innermost function or class that holds `range`, or the whole file at module level.
pub fn scope_of(checker: &Checker, range: TextRange) -> TextRange {
    checker
        .function_defs()
        .iter()
        .map(Ranged::range)
        .chain(checker.class_defs().iter().map(Ranged::range))
        .filter(|scope| scope.contains_range(range))
        .min_by_key(|scope| scope.len())
        .unwrap_or_else(|| TextRange::up_to(checker.source.text_len()))
}

// How many identifiers in `scope` spell `name`. An attribute such as `obj.name` reads another binding, so it does not
// count. Any other spelling, a parameter or an import included, may bind the name, so the fix leaves it alone.
pub fn identifier_uses(checker: &Checker, scope: TextRange, name: &str) -> usize {
    let tokens = checker.parsed.tokens().in_range(scope);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind() == TokenKind::Identifier
                && checker.text(token.range()) == name
                && index.checked_sub(1).is_none_or(|previous| tokens[previous].kind() != TokenKind::Dot)
        })
        .count()
}

// The edits that replace `first` through the `try` header with `header`, and delete the `finally` clause. Comments in
// the lines they remove would go with them, so they block the edit.
pub fn wrap_edits(checker: &Checker, first: TextRange, try_stmt: &ast::StmtTry, header: &str) -> Option<[Edit; 2]> {
    let source = checker.source;
    let body_first = try_stmt.body.first()?;
    let body_last = try_stmt.body.last()?;
    // A body that shares the `try` line has no line of its own to keep.
    if source.line_start(body_first.start()) <= try_stmt.start() {
        return None;
    }
    let head = TextRange::new(first.start(), source.line_start(body_first.start()));
    let tail = TextRange::new(source.full_line_end(body_last.end()), source.full_line_end(try_stmt.end()));
    if checker.has_comment_in(head) || checker.has_comment_in(tail) {
        return None;
    }
    Some([Edit::range_replacement(header.to_owned(), head), Edit::range_deletion(tail)])
}
