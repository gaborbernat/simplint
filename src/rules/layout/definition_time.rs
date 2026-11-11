use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::{ExprCollector, alias_binding, pattern_capture};

// The names a class body reads as it runs: outside its methods, or in their decorators and default values.
pub(super) fn class_runtime_names(class: &ast::StmtClassDef) -> FxHashSet<&str> {
    let mut runtime_names = ExprCollector::new(|expr| expr.as_name_expr().map(|name| name.id.as_str()));
    for stmt in &class.body {
        match stmt {
            Stmt::FunctionDef(function) => {
                for expr in definition_time_exprs(function) {
                    runtime_names.visit_expr(expr);
                }
            }
            other => runtime_names.visit_stmt(other),
        }
    }
    runtime_names.found.into_iter().collect()
}

// The parts of a `def` that run when it is defined rather than called: its decorators and default values.
pub(super) fn definition_time_exprs(function: &ast::StmtFunctionDef) -> impl Iterator<Item = &Expr> {
    let defaults = function.parameters.iter_non_variadic_params().filter_map(|parameter| parameter.default.as_deref());
    function.decorator_list.iter().map(|decorator| &decorator.expression).chain(defaults)
}

// Whether a `def` calls anything as it runs, in a decorator or a default value: a call can observe the order of the
// class body, as `next()` on a shared counter does.
pub(super) fn calls_at_definition(function: &ast::StmtFunctionDef) -> bool {
    definition_time_exprs(function).any(|expr| any_over_expr(expr, &Expr::is_call_expr))
}

// The name a `def` or `class` statement binds.
pub(super) fn definition_name(stmt: &Stmt) -> Option<&ast::Identifier> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            Some(name)
        }
        _ => None,
    }
}

// The names a statement binds in the scope it sits in: the names it stores or deletes, and the functions, classes and
// imports it defines, but not the bodies of those functions and classes.
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
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.names.extend(names.iter().map(alias_binding));
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
