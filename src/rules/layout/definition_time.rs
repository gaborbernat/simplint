use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::{alias_binding, import_aliases, is_property_accessor, last_segment, pattern_capture};

// The names a class body calls while it runs, directly, as a decorator, or by handing them to a call that may call
// them. Reading a method, as `__str__ = __repr__` or `property(_get)` do, runs nothing.
pub(super) fn class_runtime_names(class: &ast::StmtClassDef) -> FxHashSet<&str> {
    let mut called = CalledNames::default();
    for stmt in &class.body {
        match stmt {
            // `@name.setter` hands the property a new accessor without calling the getter `name` holds.
            Stmt::FunctionDef(function) => {
                for decorator in
                    function.decorator_list.iter().filter(|decorator| !is_property_accessor(&decorator.expression))
                {
                    if let Expr::Name(name) = &decorator.expression {
                        called.names.insert(name.id.as_str());
                    }
                    called.visit_expr(&decorator.expression);
                }
                let defaults =
                    function.parameters.iter_non_variadic_params().filter_map(|parameter| parameter.default.as_deref());
                for default in defaults {
                    called.visit_expr(default);
                }
            }
            other => called.visit_stmt(other),
        }
    }
    called.names
}

#[derive(Default)]
struct CalledNames<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for CalledNames<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            if let Expr::Name(callee) = &*call.func {
                self.names.insert(callee.id.as_str());
            }
            // These wrap the function they are given without calling it.
            let wraps = last_segment(&call.func).is_some_and(|name| {
                matches!(name, "property" | "staticmethod" | "classmethod" | "partialmethod" | "cached_property")
            });
            if !wraps {
                let values =
                    call.arguments.args.iter().chain(call.arguments.keywords.iter().map(|keyword| &keyword.value));
                self.names.extend(values.filter_map(|value| value.as_name_expr()).map(|name| name.id.as_str()));
            }
        }
        visitor::walk_expr(self, expr);
    }
}

pub(super) fn definition_time_exprs(function: &ast::StmtFunctionDef) -> impl Iterator<Item = &Expr> {
    let defaults = function.parameters.iter_non_variadic_params().filter_map(|parameter| parameter.default.as_deref());
    function.decorator_list.iter().map(|decorator| &decorator.expression).chain(defaults)
}

// Whether a `def` calls anything as it runs, in a decorator or a default value: a call can observe the order of the
// class body, as `next()` on a shared counter does.
pub(super) fn calls_at_definition(function: &ast::StmtFunctionDef) -> bool {
    definition_time_exprs(function).any(|expr| any_over_expr(expr, &Expr::is_call_expr))
}

pub(super) fn definition_name(stmt: &Stmt) -> Option<&ast::Identifier> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            Some(name)
        }
        _ => None,
    }
}

pub(super) fn bound_names(stmt: &Stmt) -> Vec<&str> {
    let mut bound = BoundNames::default();
    bound.visit_stmt(stmt);
    bound.names
}

#[derive(Default)]
struct BoundNames<'a> {
    names: Vec<&'a str>,
}

impl<'a> Visitor<'a> for BoundNames<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
                self.names.push(name.as_str());
            }
            Stmt::Import(_) | Stmt::ImportFrom(_) => {
                self.names.extend(import_aliases(stmt).iter().map(alias_binding));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Store | ExprContext::Del, .. }) = expr {
            self.names.push(id.as_str());
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        self.names.extend(pattern_capture(pattern).map(ast::Identifier::as_str));
        visitor::walk_pattern(self, pattern);
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = handler;
        self.names.extend(name.iter().map(ast::Identifier::as_str));
        visitor::walk_except_handler(self, handler);
    }
}
