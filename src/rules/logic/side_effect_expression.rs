use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::{self as ast, AnyNodeRef, BoolOp, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::for_each_body;

pub fn side_effect_expression(checker: &Checker) {
    for_each_body(checker.body(), &mut |body, _| {
        for stmt in body {
            if let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt {
                check(checker, stmt, value);
            }
        }
    });
}

fn check(checker: &Checker, stmt: &Stmt, value: &Expr) {
    match value {
        Expr::ListComp(ast::ExprListComp { elt, generators, .. })
        | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
            if acts(elt) =>
        {
            let kind = if value.is_list_comp_expr() { "list" } else { "set" };
            checker.report_fix(
                Rule::SideEffectExpression,
                value.range(),
                format!("This {kind} comprehension is built only for the calls in it; write a `for` loop"),
                loop_fix(checker, stmt, elt, generators),
                "Write a `for` loop",
            );
        }
        Expr::DictComp(ast::ExprDictComp { key, value: item, .. })
            if key.as_deref().is_some_and(acts) || acts(item) =>
        {
            checker.report(
                Rule::SideEffectExpression,
                value.range(),
                "This dict comprehension is built only for the calls in it; write a `for` loop",
            );
        }
        Expr::BoolOp(operation) if operation.values.last().is_some_and(acts) => {
            let word = if operation.op == BoolOp::And { "and" } else { "or" };
            checker.report_fix(
                Rule::SideEffectExpression,
                value.range(),
                format!("This `{word}` is used as a statement to decide whether to run its last part; write an `if`"),
                if_fix(checker, stmt, operation),
                "Write an `if`",
            );
        }
        _ => {}
    }
}

// Whether the expression does something beyond producing a value: a call, an `await` or a `yield`.
fn acts(expr: &Expr) -> bool {
    any_over_expr(expr, &|expr: &Expr| {
        matches!(expr, Expr::Call(_) | Expr::Await(_) | Expr::Yield(_) | Expr::YieldFrom(_))
    })
}

// The comprehension becomes nested `for` and `if` statements around its element. The loop variables then stay bound
// after the loop, where a comprehension keeps them to itself, so the fix is unsafe.
fn loop_fix(checker: &Checker, stmt: &Stmt, elt: &Expr, generators: &[ast::Comprehension]) -> Option<Fix> {
    if !rewritable(checker, stmt) {
        return None;
    }
    let indentation = checker.indentation(stmt);
    let eol = checker.line_ending();
    let mut lines = Vec::new();
    let mut depth = 0;
    for generator in generators {
        let parent = AnyNodeRef::from(generator);
        let keyword = if generator.is_async { "async for" } else { "for" };
        lines.push(format!(
            "{}{keyword} {} in {}:",
            "    ".repeat(depth),
            checker.parenthesized_text(&generator.target, parent),
            checker.parenthesized_text(&generator.iter, parent)
        ));
        depth += 1;
        for condition in &generator.ifs {
            lines.push(format!("{}if {}:", "    ".repeat(depth), checker.parenthesized_text(condition, parent)));
            depth += 1;
        }
    }
    lines.push(format!("{}{}", "    ".repeat(depth), checker.text(elt.range())));
    let text = lines.join(&format!("{eol}{indentation}"));
    Some(Fix::unsafe_edit(Edit::range_replacement(text, stmt.range())))
}

// The fix rewrites the statement in place, so it must stand alone on its lines and hold no comment that would be lost.
fn rewritable(checker: &Checker, stmt: &Stmt) -> bool {
    checker.is_alone_on_lines(stmt.range()) && !checker.has_comment_in(stmt.range())
}

// `a and f()` runs `f()` when `a` is true, and `a or f()` when it is false. The `if` tests the same operands the same
// number of times, except for an `or` of three or more, whose `not (a or b)` tests the last of them twice.
fn if_fix(checker: &Checker, stmt: &Stmt, operation: &ast::ExprBoolOp) -> Option<Fix> {
    let (Some((last, tests)), true) = (operation.values.split_last(), rewritable(checker, stmt)) else {
        return None;
    };
    let op = operation.op;
    let joiner = if op == BoolOp::And { " and " } else { " or " };
    let parent = AnyNodeRef::from(operation);
    let test = tests.iter().map(|test| checker.parenthesized_text(test, parent)).collect::<Vec<_>>().join(joiner);
    let (test, fix): (String, fn(Edit) -> Fix) = match (op, tests) {
        (BoolOp::And, _) => (test, Fix::safe_edit),
        (BoolOp::Or, [single]) if is_atom(single) || test.len() > single.range().len().to_usize() => {
            (format!("not {test}"), Fix::safe_edit)
        }
        (BoolOp::Or, [_]) => (format!("not ({test})"), Fix::safe_edit),
        (BoolOp::Or, _) => (format!("not ({test})"), Fix::unsafe_edit),
    };
    let statement = format!(
        "if {test}:{}{}    {}",
        checker.line_ending(),
        checker.indentation(stmt),
        checker.parenthesized_text(last, parent)
    );
    Some(fix(Edit::range_replacement(statement, stmt.range())))
}

// An operand `not` binds to directly, without parentheses of its own.
fn is_atom(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Name(_)
            | Expr::Attribute(_)
            | Expr::Subscript(_)
            | Expr::Call(_)
            | Expr::List(_)
            | Expr::Dict(_)
            | Expr::Set(_)
            | Expr::Tuple(_)
            | Expr::NumberLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
    )
}
