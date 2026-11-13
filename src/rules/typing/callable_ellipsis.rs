use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use super::is_callable;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Expr::Subscript(ast::ExprSubscript { value, slice, range, .. }) = expr else {
        return;
    };
    let Expr::Tuple(ast::ExprTuple { elts, .. }) = slice.as_ref() else {
        return;
    };
    if is_callable(walker.checker.semantic(), value)
        && elts.first().is_some_and(Expr::is_ellipsis_literal_expr)
        && !uncalled_test_parameter(walker, expr)
    {
        walker.checker.report(
            Rule::CallableEllipsis,
            *range,
            "`Callable[..., T]` hides the signature; describe it with a `Protocol`",
        );
    }
}

fn uncalled_test_parameter(walker: &Walker, annotation: &Expr) -> bool {
    if !walker.checker.is_test_module() {
        return false;
    }
    let Some(Scope::Function(function)) = walker.scopes().last() else {
        return false;
    };
    function.parameters.iter().any(|parameter| {
        parameter.annotation().is_some_and(|owned| owned.range().contains_range(annotation.range())) && {
            let mut calls = CallsParameter { name: parameter.name().as_str(), found: false };
            calls.visit_body(&function.body);
            !calls.found
        }
    })
}

struct CallsParameter<'a> {
    name: &'a str,
    found: bool,
}

impl<'a> Visitor<'a> for CallsParameter<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr
            && (call.func.as_name_expr().is_some_and(|name| name.id.as_str() == self.name)
                || call.func.as_attribute_expr().is_some_and(|attribute| {
                    attribute.attr.as_str() == "__call__"
                        && attribute.value.as_name_expr().is_some_and(|name| name.id.as_str() == self.name)
                }))
        {
            self.found = true;
        }
        visitor::walk_expr(self, expr);
    }
}
