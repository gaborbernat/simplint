use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Number, Operator, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

// The methods that change a list, dict, set or deque in place.
const MUTATORS: [&str; 18] = [
    "add",
    "append",
    "appendleft",
    "clear",
    "difference_update",
    "discard",
    "extend",
    "extendleft",
    "insert",
    "intersection_update",
    "pop",
    "popitem",
    "remove",
    "reverse",
    "setdefault",
    "sort",
    "symmetric_difference_update",
    "update",
];

// Repeating a list of rows only copies references, so `[[0] * n] * m` holds one row `m` times. That is harmless while
// nothing changes a row, as with data handed to `numpy.array()`, so only a stored repetition whose element is then
// changed through the stored name, such as `grid[i][j] = 1` or `grid[i].append(x)`, is reported.
pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return;
    };
    let [target] = targets.as_slice() else {
        return;
    };
    if !matches!(target, Expr::Name(_) | Expr::Attribute(_)) {
        return;
    }
    let Expr::BinOp(ast::ExprBinOp { left, op: Operator::Mult, right, .. }) = value.as_ref() else {
        return;
    };
    let checker = walker.checker;
    let (sequence, count) = match (left.as_ref(), right.as_ref()) {
        (Expr::List(_) | Expr::Tuple(_), _) => (left.as_ref(), right.as_ref()),
        (_, Expr::List(_) | Expr::Tuple(_)) => (right.as_ref(), left.as_ref()),
        _ => return,
    };
    let elements = match sequence {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => unreachable!("matched as a list or tuple display above"),
    };
    // Zero or one copy shares nothing.
    if matches!(
        count,
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: Number::Int(int), .. })
            if int.as_u8().is_some_and(|copies| copies <= 1)
    ) {
        return;
    }
    let Some(element) = elements.iter().find(|element| is_mutable(checker, element)) else {
        return;
    };
    // A local name is changed in its own function; a module name or an attribute anywhere in the file.
    let region = match (target, walker.scopes().last()) {
        (Expr::Name(_), Some(Scope::Function(function))) => function.body.as_slice(),
        _ => checker.body(),
    };
    let mut finder = Mutation { target: ComparableExpr::from(target), found: false };
    finder.visit_body(region);
    if !finder.found {
        return;
    }
    checker.report_fix(
        Rule::SharedMutableRepetition,
        value.range(),
        format!(
            "Every copy is the one `{}` object, so changing an element through `{}` changes all of them",
            checker.text(element),
            checker.text(target)
        ),
        comprehension(checker, value, sequence, count),
        "Build each element in a comprehension",
    );
}

struct Mutation<'t> {
    target: ComparableExpr<'t>,
    found: bool,
}

impl Mutation<'_> {
    // `target[i]`, an element of the repetition.
    fn is_element(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Subscript(ast::ExprSubscript { value, .. })
                if ComparableExpr::from(value.as_ref()) == self.target
        )
    }

    // `target[i][j]` or `target[i].name`, a part of an element.
    fn is_inside_element(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Subscript(ast::ExprSubscript { value, .. }) | Expr::Attribute(ast::ExprAttribute { value, .. }) => {
                self.is_element(value)
            }
            _ => false,
        }
    }
}

impl<'a> Visitor<'a> for Mutation<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.found |= match stmt {
            Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                targets.iter().any(|target| self.is_inside_element(target))
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => self.is_inside_element(target),
            // `grid[i] += [x]` extends the shared list in place.
            Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
                self.is_inside_element(target) || self.is_element(target)
            }
            Stmt::Delete(ast::StmtDelete { targets, .. }) => {
                targets.iter().any(|target| self.is_inside_element(target))
            }
            _ => false,
        };
        if !self.found {
            walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(ast::ExprCall { func, .. }) = expr
            && let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = func.as_ref()
            && MUTATORS.contains(&attr.as_str())
            && self.is_element(value)
        {
            self.found = true;
        }
        if !self.found {
            walk_expr(self, expr);
        }
    }
}

// A display, a comprehension or a call that builds a new mutable object, and a list repeated by `*`, which is a list.
fn is_mutable(checker: &Checker, element: &Expr) -> bool {
    match element {
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::Mult | Operator::Add, right, .. }) => {
            matches!(left.as_ref(), Expr::List(_)) || matches!(right.as_ref(), Expr::List(_))
        }
        _ => checker.semantic().is_mutable_value(element),
    }
}

// `[element] * count` becomes `[element for _ in range(count)]`, which evaluates the element once per copy. Only a list
// of one element has that form, and only while `range` is the builtin and the element does not read `_`.
fn comprehension(checker: &Checker, repetition: &Expr, sequence: &Expr, count: &Expr) -> Option<Fix> {
    let Expr::List(ast::ExprList { elts, .. }) = sequence else {
        return None;
    };
    let [element] = elts.as_slice() else {
        return None;
    };
    if checker.semantic().is_bound("range") || checker.identifier_count_in(element.range(), "_") > 0 {
        return None;
    }
    let replacement =
        format!("[{} for _ in range({})]", checker.parenthesized_text(element, sequence.into()), checker.text(count));
    Some(Fix::unsafe_edit(Edit::range_replacement(replacement, repetition.range())))
}
