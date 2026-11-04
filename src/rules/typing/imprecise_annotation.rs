use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::ExprCollector;
use crate::syntax::last_segment;

const OBJECT_MESSAGE: &str = "`object` annotation; use the precise type";
const ANY_MESSAGE: &str = "`Any` accepts any value and turns off type checking for it; use a precise type";

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
                    report_imprecise(walker, annotation, true);
                }
            }
            if let Some(returns) = &function.returns {
                report_imprecise(walker, returns, true);
            }
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => report_imprecise(walker, annotation, false),
        _ => {}
    }
}

// Ruff's ANN401 already reports `Any` as the whole annotation of a parameter or return, so `whole_any_by_ann401`
// leaves that one to it.
fn report_imprecise(walker: &Walker, annotation: &Expr, whole_any_by_ann401: bool) {
    let semantic = walker.checker.semantic();
    let mut finder = ExprCollector::new(|expr: &Expr| {
        if UnqualifiedName::from_expr(expr)
            .is_some_and(|name| matches!(name.segments(), ["builtins", "object"] | ["object"]))
        {
            return Some((expr.range(), OBJECT_MESSAGE));
        }
        let covered_by_ann401 = whole_any_by_ann401 && expr.range() == annotation.range();
        (!covered_by_ann401 && semantic.match_typing_expr(expr, "Any")).then(|| (expr.range(), ANY_MESSAGE))
    });
    finder.visit_expr(annotation);
    for (range, message) in finder.found {
        walker.checker.report(Rule::ImpreciseAnnotation, range, message);
    }
}
