use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;

pub fn callable_ellipsis(checker: &mut Checker) {
    let mut finder = ExprFinder::new(|expr| {
        let Expr::Subscript(ast::ExprSubscript { value, slice, .. }) = expr else {
            return false;
        };
        let Expr::Tuple(ast::ExprTuple { elts, .. }) = slice.as_ref() else {
            return false;
        };
        UnqualifiedName::from_expr(value).is_some_and(|name| name.segments().last() == Some(&"Callable"))
            && elts.first().is_some_and(Expr::is_ellipsis_literal_expr)
    });
    finder.visit_body(checker.body());
    for range in finder.found {
        checker.report(
            Rule::CallableEllipsis,
            range,
            "`Callable[..., T]` hides the signature; describe it with a `Protocol`",
            None,
        );
    }
}

struct ExprFinder<F> {
    predicate: F,
    found: Vec<TextRange>,
}

impl<F: Fn(&Expr) -> bool> ExprFinder<F> {
    fn new(predicate: F) -> Self {
        Self { predicate, found: Vec::new() }
    }
}

impl<'a, F: Fn(&Expr) -> bool> Visitor<'a> for ExprFinder<F> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if (self.predicate)(expr) {
            self.found.push(expr.range());
        }
        visitor::walk_expr(self, expr);
    }
}

pub fn object_annotation(checker: &mut Checker) {
    let mut finder = ObjectAnnotations { exempt_parameters: false, found: Vec::new() };
    finder.visit_body(checker.body());
    for range in finder.found {
        checker.report(Rule::ObjectAnnotation, range, "`object` annotation; use the precise type", None);
    }
}

struct ObjectAnnotations {
    exempt_parameters: bool,
    found: Vec<TextRange>,
}

impl ObjectAnnotations {
    fn collect(&mut self, annotation: &Expr) {
        let mut finder = ExprFinder::new(|expr| {
            UnqualifiedName::from_expr(expr)
                .is_some_and(|name| matches!(name.segments(), ["builtins", "object"] | ["object"]))
        });
        finder.visit_expr(annotation);
        self.found.extend(finder.found);
    }
}

impl<'a> Visitor<'a> for ObjectAnnotations {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                let is_guard = function.returns.as_deref().is_some_and(|returns| {
                    let Expr::Subscript(ast::ExprSubscript { value, .. }) = returns else {
                        return false;
                    };
                    UnqualifiedName::from_expr(value)
                        .is_some_and(|name| matches!(name.segments().last(), Some(&("TypeIs" | "TypeGuard"))))
                });
                let saved =
                    std::mem::replace(&mut self.exempt_parameters, is_guard || is_dunder(function.name.as_str()));
                visitor::walk_stmt(self, stmt);
                self.exempt_parameters = saved;
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { annotation, value, .. }) => {
                self.collect(annotation);
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_parameter(&mut self, parameter: &'a ast::Parameter) {
        if let Some(annotation) = &parameter.annotation
            && !self.exempt_parameters
        {
            self.collect(annotation);
        }
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.collect(expr);
    }
}

pub fn vars_call(checker: &mut Checker) {
    if checker.semantic().is_bound("vars") {
        return;
    }
    let builtin: Vec<TextRange> = checker.semantic().references_to("vars").map(|reference| reference.range).collect();
    let mut finder = ExprFinder::new(|expr| {
        expr.as_call_expr()
            .and_then(|call| call.func.as_name_expr())
            .is_some_and(|name| name.id.as_str() == "vars" && builtin.contains(&name.range))
    });
    finder.visit_body(checker.body());
    for range in finder.found {
        checker.report(Rule::VarsCall, range, "`vars()` erases types; read attributes from a typed object", None);
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::CallableEllipsis, "SMP105.py" ; "callable_ellipsis")]
    #[test_case(Rule::ObjectAnnotation, "SMP106.py" ; "object_annotation")]
    #[test_case(Rule::VarsCall, "SMP107.py" ; "vars_call")]
    #[test_case(Rule::VarsCall, "SMP107_bound.py" ; "vars_call_shadowed")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
