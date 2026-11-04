// What inlining a value into the statement that uses it takes, shared by `single-use-variable` and `walrus-candidate`:
// the walk over the scopes a body sits in, the text that replaces the use, and the width the statement then needs.
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::syntax::{for_each_body, for_each_child_body};

// Calls `visit` with each statement list under `body`, the scope it sits in and how many indented blocks enclose it.
// `enter` gives the scope of the statements inside a compound statement.
pub(super) fn for_each_scoped_body<'a, S: Copy>(
    body: &'a [Stmt],
    scope: S,
    depth: usize,
    enter: &impl Fn(S, &'a Stmt) -> S,
    visit: &mut impl FnMut(&'a [Stmt], S, usize),
) {
    visit(body, scope, depth);
    for stmt in body {
        let inner = enter(scope, stmt);
        // A `case` body sits a block inside the `match` block.
        let blocks = if stmt.is_match_stmt() { 2 } else { 1 };
        for_each_child_body(stmt, &mut |child| for_each_scoped_body(child, inner, depth + blocks, enter, visit));
    }
}

// The value as the use takes it, parenthesised when `parenthesize` is set and the source does not already parenthesise
// it. The value's range is the one its parentheses sit in, so the source text is compared with the bare value.
pub(super) fn inlined_text(checker: &Checker, value: &Expr, value_range: TextRange, parenthesize: bool) -> String {
    let text = checker.text(value_range);
    if parenthesize && text == checker.text(value.range()) { format!("({text})") } else { text.to_owned() }
}

// The names a function declares `global` or `nonlocal`, in its nested blocks and functions too.
pub(super) fn declared_outer(body: &[Stmt]) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Global(ast::StmtGlobal { names: declared, .. })
            | Stmt::Nonlocal(ast::StmtNonlocal { names: declared, .. }) = stmt
            {
                names.extend(declared.iter().map(ast::Identifier::as_str));
            }
        }
    });
    names
}

// The name a test starts with, through `not`, comparisons and `and`/`or`. `bare` is set while the name is the test
// itself rather than an operand of it.
pub(super) fn leading_name(test: &Expr, bare: bool) -> Option<(&ast::ExprName, bool)> {
    match test {
        Expr::Name(name) => Some((name, bare)),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => leading_name(operand, false),
        Expr::Compare(ast::ExprCompare { operands, .. }) => leading_name(operands.first()?, false),
        Expr::BoolOp(ast::ExprBoolOp { values, .. }) => leading_name(values.first()?, false),
        _ => None,
    }
}

// The width test of inlining: a header of `header` characters, joined on one line at the indentation of `depth` blocks,
// still fits the line width once the `name` in it becomes a value `wrapping` characters longer than the name's text.
pub(super) fn fits_line(
    checker: &Checker,
    depth: usize,
    header: usize,
    name: usize,
    value: usize,
    wrapping: usize,
) -> bool {
    let indent = checker.settings.from_ruff.indent_width * depth;
    indent + header - name + value + wrapping <= checker.settings.from_ruff.line_width
}
