use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, ExprContext, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::{ExprCollector, alias_binding};

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

// The names a statement binds in the scope it sits in: the names it stores, and the functions, classes and imports it
// defines, but not the bodies of those functions and classes.
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
        if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, .. }) = expr {
            self.names.push(id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}

// The annotations in a `def`'s signature: its parameters and its return type.
pub(super) fn signature_annotations(function: &ast::StmtFunctionDef) -> impl Iterator<Item = &Expr> {
    function.parameters.iter().filter_map(AnyParameterRef::annotation).chain(function.returns.as_deref())
}
