use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::map_callable;
use ruff_python_ast::{self as ast, CmpOp, Expr, Stmt, UnaryOp};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

// The comparison and binary operator methods, for which `NotImplemented` lets Python try the other operand's method.
const OPERATORS: [&str; 34] = [
    "__eq__", "__ne__", "__lt__", "__le__", "__gt__", "__ge__", "__add__", "__sub__", "__mul__", "__matmul__",
    "__truediv__", "__floordiv__", "__mod__", "__divmod__", "__pow__", "__lshift__", "__rshift__", "__and__", "__xor__",
    "__or__", "__radd__", "__rsub__", "__rmul__", "__rmatmul__", "__rtruediv__", "__rfloordiv__", "__rmod__",
    "__rdivmod__", "__rpow__", "__rlshift__", "__rrshift__", "__rand__", "__rxor__", "__ror__",
];

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(function) = stmt else {
        return;
    };
    let name = function.name.as_str();
    if !matches!(walker.scopes().last(), Some(Scope::Class(_))) || !OPERATORS.contains(&name) {
        return;
    }
    let Some(other) = function.parameters.args.get(1).or_else(|| function.parameters.posonlyargs.get(1)) else {
        return;
    };
    let checker = walker.checker;
    for stmt in &function.body {
        let Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) = stmt else {
            continue;
        };
        let [rejection] = body.as_slice() else {
            continue;
        };
        if !elif_else_clauses.is_empty() || !rejects_type(checker, test, other.parameter.name.as_str()) {
            continue;
        }
        let what = match rejection {
            Stmt::Return(ast::StmtReturn { value: Some(value), .. }) if value.is_boolean_literal_expr() => {
                format!("returns `{}`", checker.text(value.range()))
            }
            Stmt::Raise(ast::StmtRaise { exc: Some(exc), cause: None, .. }) if raises_type_error(checker, exc) => {
                "raises `TypeError`".to_owned()
            }
            _ => continue,
        };
        // Python answers `False` for `==` and raises `TypeError` for the others once both operands return
        // `NotImplemented`, but the other operand's method may now answer instead, so the fix is unsafe.
        let fix = (!checker.has_comment_in(rejection.range()))
            .then(|| Fix::unsafe_edit(Edit::range_replacement("return NotImplemented".to_owned(), rejection.range())));
        checker.report_fix(
            Rule::RejectedOperand,
            rejection.range(),
            format!(
                "`{name}` {what} for an operand type it does not handle, which stops Python from asking the other \
                 operand; return `NotImplemented`"
            ),
            fix,
            "Return `NotImplemented`",
        );
    }
}

// `not isinstance(other, ...)`, `type(other) is not ...` or `type(other) != ...`.
fn rejects_type(checker: &Checker, test: &Expr, other: &str) -> bool {
    let is_other = |expr: &Expr| expr.as_name_expr().is_some_and(|name| name.id.as_str() == other);
    match test {
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => {
            operand.as_call_expr().is_some_and(|call| {
                checker.semantic().is_builtin(&call.func, "isinstance")
                    && call.arguments.args.first().is_some_and(is_other)
            })
        }
        Expr::Compare(ast::ExprCompare { ops, operands, .. }) => {
            matches!(ops.as_ref(), [CmpOp::IsNot | CmpOp::NotEq])
                && operands.first().and_then(Expr::as_call_expr).is_some_and(|call| {
                    checker.semantic().is_builtin(&call.func, "type")
                        && matches!(call.arguments.args.as_ref(), [argument] if is_other(argument))
                })
        }
        _ => false,
    }
}

fn raises_type_error(checker: &Checker, exc: &Expr) -> bool {
    checker.semantic().is_builtin(map_callable(exc), "TypeError")
}
