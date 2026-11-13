use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::{self as ast, Expr, Number, Operator, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::name_uses::{LocalScope, NameUses, for_each_suite};
use crate::syntax::{expr_mentions, target_names};

pub fn accumulator_loop(checker: &Checker) {
    for scope in checker.state().local_scopes(checker).iter() {
        for_each_suite(scope.body, &mut |suite| {
            for pair in suite.windows(2) {
                check_pair(checker, &pair[0], &pair[1], scope);
            }
        });
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Join,
    Sum,
}

struct Accumulation<'a> {
    test: Option<&'a Expr>,
    value: &'a Expr,
}

fn check_pair(checker: &Checker, init: &Stmt, next: &Stmt, scope: &LocalScope) {
    let Stmt::Assign(ast::StmtAssign { targets, value: start, .. }) = init else {
        return;
    };
    let [Expr::Name(target)] = targets.as_slice() else {
        return;
    };
    let Some(kind) = empty_start(start) else {
        return;
    };
    let Stmt::For(loop_stmt) = next else {
        return;
    };
    if loop_stmt.is_async || !loop_stmt.orelse.is_empty() {
        return;
    }
    let name = target.id.as_str();
    let Some(accumulation) = accumulation(&loop_stmt.body, name) else {
        return;
    };
    let uses = scope.uses();
    if expr_mentions(&loop_stmt.iter, name)
        || expr_mentions(&loop_stmt.target, name)
        || uses.is_opaque(name)
        || accumulation.test.is_some_and(|test| expr_mentions(test, name))
        || expr_mentions(accumulation.value, name)
    {
        return;
    }
    let call = match kind {
        Kind::Join => format!("{}.join", checker.text(start.as_ref())),
        Kind::Sum => "sum".to_owned(),
    };
    let fix = (loop_variables_stay_inside(loop_stmt, uses)
        && !accumulation.suspends()
        && !checker.has_comment_in(TextRange::new(init.start(), next.end())))
    .then(|| {
        let replacement = format!("{name} = {call}({})", generator(checker, loop_stmt, &accumulation));
        Fix::unsafe_edit(Edit::range_replacement(replacement, TextRange::new(init.start(), next.end())))
    });
    let (message, title) = match kind {
        Kind::Join => (format!("`{name}` is built by `+=` in a loop; `str.join` builds it in one call"), "Use `join`"),
        Kind::Sum => (format!("`{name}` is summed by `+=` in a loop; `sum` adds it up in one call"), "Use `sum`"),
    };
    checker.report_fix(
        Rule::AccumulatorLoop,
        TextRange::new(loop_stmt.start(), loop_stmt.iter.end()),
        message,
        fix,
        title,
    );
}

fn empty_start(start: &Expr) -> Option<Kind> {
    match start {
        Expr::StringLiteral(string) if string.value.is_empty() && !string.value.is_implicit_concatenated() => {
            Some(Kind::Join)
        }
        Expr::BytesLiteral(bytes) if bytes.value.is_empty() && !bytes.value.is_implicit_concatenated() => {
            Some(Kind::Join)
        }
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: Number::Int(int), .. }) if int.as_u8() == Some(0) => {
            Some(Kind::Sum)
        }
        _ => None,
    }
}

// The loop's only statement adds to the accumulator, directly or under an `if` with no other branch.
fn accumulation<'a>(body: &'a [Stmt], name: &str) -> Option<Accumulation<'a>> {
    match body {
        [Stmt::AugAssign(augmented)] => added(augmented, name).map(|value| Accumulation { test: None, value }),
        [Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. })] if elif_else_clauses.is_empty() => {
            let [Stmt::AugAssign(augmented)] = body.as_slice() else {
                return None;
            };
            added(augmented, name).map(|value| Accumulation { test: Some(test), value })
        }
        _ => None,
    }
}

// A tuple added to `""` or `0` raises, so a loop that adds one accumulates nothing.
fn added<'a>(augmented: &'a ast::StmtAugAssign, name: &str) -> Option<&'a Expr> {
    (augmented.op == Operator::Add
        && augmented.target.as_name_expr().is_some_and(|target| target.id == name)
        && !augmented.value.is_tuple_expr())
    .then_some(augmented.value.as_ref())
}

// A generator keeps its loop variables to itself, so the fix may only drop them when nothing outside the loop reads
// them.
fn loop_variables_stay_inside(loop_stmt: &ast::StmtFor, uses: &NameUses) -> bool {
    target_names(&loop_stmt.target).into_iter().all(|variable| {
        !uses.is_opaque(variable)
            && uses.stores(variable).len() == 1
            && uses.loads(variable).iter().all(|load| loop_stmt.range().contains_range(load.range))
    })
}

impl Accumulation<'_> {
    // A generator expression rejects a `yield` in its element or condition, and an `await` there makes it an
    // asynchronous generator, which `sum` and `str.join` cannot consume.
    fn suspends(&self) -> bool {
        self.test.into_iter().chain([self.value]).any(|expr| {
            any_over_expr(expr, &|inner: &Expr| matches!(inner, Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Await(_)))
        })
    }
}

fn generator(checker: &Checker, loop_stmt: &ast::StmtFor, accumulation: &Accumulation) -> String {
    let iterable_alone = loop_stmt.target.as_name_expr().is_some_and(|target| {
        accumulation.test.is_none() && accumulation.value.as_name_expr().is_some_and(|value| value.id == target.id)
    });
    // A call argument takes any expression but a bare tuple, which would split into arguments, or a `yield`.
    if iterable_alone {
        let text = checker.text(loop_stmt.iter.as_ref());
        return match loop_stmt.iter.as_ref() {
            Expr::Yield(_) | Expr::YieldFrom(_) => format!("({text})"),
            Expr::Tuple(tuple) if !tuple.parenthesized => format!("({text})"),
            _ => text.to_owned(),
        };
    }
    let target = checker.text(loop_stmt.target.as_ref());
    let iter = operand(checker, &loop_stmt.iter);
    let value = checker.text(accumulation.value);
    match accumulation.test {
        Some(test) => format!("{value} for {target} in {iter} if {}", operand(checker, test)),
        None => format!("{value} for {target} in {iter}"),
    }
}

// The iterable and the condition of a comprehension clause take neither a conditional expression, a lambda, an
// assignment expression nor a bare tuple.
fn operand(checker: &Checker, expr: &Expr) -> String {
    let text = checker.text(expr);
    match expr {
        Expr::If(_) | Expr::Lambda(_) | Expr::Named(_) | Expr::Yield(_) | Expr::YieldFrom(_) => format!("({text})"),
        Expr::Tuple(tuple) if !tuple.parenthesized => format!("({text})"),
        _ => text.to_owned(),
    }
}
