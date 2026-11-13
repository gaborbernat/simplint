use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;

// The names a module writes through `globals()`, or through `vars()` and `locals()` with no argument at module level.
// A computed key, or an `update`, `setdefault` or `pop` of that dict, may set any name, so the module's names cannot be
// listed.
pub(super) struct GlobalsWrites<'a, 'c> {
    checker: &'c Checker<'a>,
    names: FxHashSet<&'a str>,
    any: bool,
    // The bodies of the functions, classes and lambdas the walk is in, whose `vars()` and `locals()` are not the
    // module's.
    scopes: u32,
}

impl<'a, 'c> GlobalsWrites<'a, 'c> {
    pub(super) fn of(checker: &'c Checker<'a>) -> Self {
        let mut writes = Self { checker, names: FxHashSet::default(), any: false, scopes: 0 };
        writes.visit_body(checker.body());
        writes
    }

    pub(super) fn writes(&self, name: &str) -> bool {
        self.any || self.names.contains(name)
    }

    fn is_globals_call(&self, expr: &Expr) -> bool {
        let semantic = self.checker.semantic();
        expr.as_call_expr().is_some_and(|call| {
            semantic.is_builtin(&call.func, "globals")
                || self.scopes == 0
                    && call.arguments.is_empty()
                    && (semantic.is_builtin(&call.func, "vars") || semantic.is_builtin(&call.func, "locals"))
        })
    }

    // The body of a function, class or lambda is its own scope, while its header runs in the enclosing one.
    fn scoped_body(&mut self, body: &'a [Stmt]) {
        self.scopes += 1;
        self.visit_body(body);
        self.scopes -= 1;
    }
}

impl<'a> Visitor<'a> for GlobalsWrites<'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef {
                parameters, body, decorator_list, returns, type_params, ..
            }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = type_params {
                    self.visit_type_params(type_params);
                }
                self.visit_parameters(parameters);
                if let Some(returns) = returns {
                    self.visit_annotation(returns);
                }
                self.scoped_body(body);
            }
            Stmt::ClassDef(ast::StmtClassDef { arguments, body, decorator_list, type_params, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = type_params {
                    self.visit_type_params(type_params);
                }
                if let Some(arguments) = arguments {
                    self.visit_arguments(arguments);
                }
                self.scoped_body(body);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Lambda(ast::ExprLambda { parameters, body, .. }) = expr {
            if let Some(parameters) = parameters {
                self.visit_parameters(parameters);
            }
            self.scopes += 1;
            self.visit_expr(body);
            self.scopes -= 1;
            return;
        }
        match expr {
            Expr::Subscript(ast::ExprSubscript {
                value, slice, ctx: ExprContext::Store | ExprContext::Del, ..
            }) if self.is_globals_call(value) => match slice.as_string_literal_expr() {
                Some(key) => _ = self.names.insert(key.value.to_str()),
                None => self.any = true,
            },
            Expr::Call(ast::ExprCall { func, .. }) => {
                if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = func.as_ref()
                    && matches!(attr.as_str(), "update" | "setdefault" | "pop")
                    && self.is_globals_call(value)
                {
                    self.any = true;
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
