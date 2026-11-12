use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::flow::{changes_function_unrun, first_exit, is_false_constant};
use crate::syntax::{for_each_body, last_segment};

pub fn unreachable_code(checker: &Checker) {
    for_each_body(checker.body(), &mut |body, _| {
        after_exit(checker, body);
        for stmt in body {
            false_test(checker, stmt, body.len());
        }
    });
}

fn after_exit(checker: &Checker, body: &[Stmt]) {
    let Some((index, exit)) = first_exit(body) else {
        return;
    };
    let rest = &body[index + 1..];
    let (Some(first), Some(last)) = (rest.first(), rest.last()) else {
        return;
    };
    if changes_function_unrun(rest) || rest.iter().all(is_guard) {
        return;
    }
    let range = TextRange::new(first.start(), last.end());
    let fix = deletion(checker, range);
    checker.report_fix(
        Rule::UnreachableCode,
        range,
        format!("This code never runs: {}", exit.reason()),
        fix,
        "Delete the code that never runs",
    );
}

// An `if` or `while` whose test is the constant `False` or `0` never runs its body. A body of only imports and
// annotations is the old way to show names to type checkers alone, before `TYPE_CHECKING`, and it is left alone.
fn false_test(checker: &Checker, stmt: &Stmt, siblings: usize) {
    let (test, body, has_else) = match stmt {
        Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => (test, body, !elif_else_clauses.is_empty()),
        Stmt::While(ast::StmtWhile { test, body, orelse, .. }) => (test, body, !orelse.is_empty()),
        _ => return,
    };
    if !is_false_constant(test) || body.iter().all(only_types) || changes_function_unrun(body) {
        return;
    }
    // Without a branch that runs, the whole statement goes, unless it is the only one in its block.
    let fix = (!has_else && siblings > 1).then(|| deletion(checker, stmt.range())).flatten();
    let keyword = if stmt.is_if_stmt() { "if" } else { "while" };
    checker.report_fix(
        Rule::UnreachableCode,
        TextRange::new(body[0].start(), body[body.len() - 1].end()),
        format!("This code never runs: the `{keyword}` test above it is always false"),
        fix,
        "Delete the code that never runs",
    );
}

// A statement that only fails, put after code that cannot fall through to say so, such as `assert False` after an
// endless loop or a `raise` after an `if` whose branches all return.
fn is_guard(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Raise(_) => true,
        Stmt::Assert(ast::StmtAssert { test, .. }) => is_false_constant(test),
        Stmt::Expr(ast::StmtExpr { value, .. }) => value
            .as_call_expr()
            .is_some_and(|call| matches!(last_segment(&call.func), Some("fail" | "assert_never" | "unreachable"))),
        _ => false,
    }
}

fn only_types(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Import(_) | Stmt::ImportFrom(_) | Stmt::Pass(_) => true,
        Stmt::AnnAssign(ast::StmtAnnAssign { value, .. }) => value.is_none(),
        _ => false,
    }
}

// The lines of the statements go, when no other statement shares them.
fn deletion(checker: &Checker, range: TextRange) -> Option<Fix> {
    checker
        .is_alone_on_lines(range)
        .then(|| Fix::unsafe_edit(Edit::range_deletion(checker.source.full_lines_range(range))))
}
