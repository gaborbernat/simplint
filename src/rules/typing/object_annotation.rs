use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::{ExprCollector, last_segment};

pub fn statement(walker: &mut Walker, stmt: &Stmt) {
    match stmt {
        Stmt::FunctionDef(function) => {
            let is_guard = function.returns.as_deref().is_some_and(|returns| {
                let Expr::Subscript(ast::ExprSubscript { value, .. }) = returns else {
                    return false;
                };
                matches!(last_segment(value), Some("TypeIs" | "TypeGuard"))
            });
            if !is_guard && !is_dunder(function.name.as_str()) {
                for annotation in function.parameters.iter().filter_map(AnyParameterRef::annotation) {
                    report_objects(walker, annotation);
                }
            }
            if let Some(returns) = &function.returns {
                report_objects(walker, returns);
            }
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => report_objects(walker, annotation),
        _ => {}
    }
}

fn report_objects(walker: &Walker, annotation: &Expr) {
    let mut finder = ExprCollector::new(|expr: &Expr| {
        UnqualifiedName::from_expr(expr)
            .is_some_and(|name| matches!(name.segments(), ["builtins", "object"] | ["object"]))
            .then(|| expr.range())
    });
    finder.visit_expr(annotation);
    for range in finder.found {
        walker.checker.report(Rule::ObjectAnnotation, range, "`object` annotation; use the precise type");
    }
}
