// What inlining or moving a value takes, shared by the rules that do so, such as `single-use-variable` and
// `early-assignment`: the walks over the scopes a body sits in and the names it reads and binds, the classes of
// literals, the text that replaces a use, and the width the statement then needs.
mod fstring;
mod literals;
mod names;

use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::syntax::{for_each_body, for_each_child_body};

pub(in crate::rules::logic) use fstring::{fstring_body_parts, requoted_text};
pub(in crate::rules::logic) use literals::{is_constant, is_hashable};
pub(in crate::rules::logic) use names::{LOCALS_READERS, Names};

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

// `function` is the innermost `def` around a body; only statements inside a function are checked.
pub(super) fn enter_function<'a>(
    function: Option<&'a ast::StmtFunctionDef>,
    stmt: &'a Stmt,
) -> Option<&'a ast::StmtFunctionDef> {
    match stmt {
        Stmt::FunctionDef(definition) => Some(definition),
        Stmt::ClassDef(_) => None,
        _ => function,
    }
}
