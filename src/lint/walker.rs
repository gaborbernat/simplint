use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt, TypeParam};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::{self, RuleSet};

// One walk over the module drives the node rules, as Ruff's `Checker` drives its own: each node goes to the handlers
// its kind has among the enabled rules, which read where the node sits from the walker instead of walking again.
pub struct Walker<'c, 'a> {
    pub checker: &'c Checker<'a>,
    rules: RuleSet,
    // The names reported in each open scope, the module's first: SMP010 reports a name once per scope.
    reported_names: Vec<FxHashSet<&'a str>>,
    // The expressions holding the current one, outermost first, ending with the current one.
    expressions: Vec<&'a Expr>,
    in_fstring: bool,
    in_comprehension: bool,
    in_for_target: bool,
}

// A def, class or lambda opens a scope covering its header, decorators and default values included.
pub enum Scope<'a> {
    Function(&'a ast::StmtFunctionDef),
    Class,
    Lambda(&'a ast::ExprLambda),
}

impl<'c, 'a> Walker<'c, 'a> {
    pub fn run(checker: &'c Checker<'a>, rules: RuleSet) {
        let mut walker = Self {
            checker,
            rules,
            reported_names: vec![FxHashSet::default()],
            expressions: Vec::new(),
            in_fstring: false,
            in_comprehension: false,
            in_for_target: false,
        };
        walker.visit_body(checker.body());
    }

    // Whether the current scope has not reported `name` yet; from now on it has.
    pub fn report_name_once(&mut self, name: &'a str) -> bool {
        self.reported_names.last_mut().expect("the module scope is never left").insert(name)
    }

    pub fn expression_parent(&self) -> Option<&'a Expr> {
        self.expressions.iter().rev().nth(1).copied()
    }

    pub fn in_fstring(&self) -> bool {
        self.in_fstring
    }

    pub fn in_comprehension(&self) -> bool {
        self.in_comprehension
    }

    pub fn in_for_target(&self) -> bool {
        self.in_for_target
    }
}

impl<'a> Visitor<'a> for Walker<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        registry::statement(self.rules, self, stmt);
        match stmt {
            Stmt::FunctionDef(function) => {
                self.in_scope(&Scope::Function(function), |walker| visitor::walk_stmt(walker, stmt));
            }
            Stmt::ClassDef(_) => self.in_scope(&Scope::Class, |walker| visitor::walk_stmt(walker, stmt)),
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => {
                self.visit_expr(iter);
                let outer = std::mem::replace(&mut self.in_for_target, true);
                self.visit_expr(target);
                self.in_for_target = outer;
                self.visit_body(body);
                self.visit_body(orelse);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.expressions.push(expr);
        registry::expression(self.rules, self, expr);
        match expr {
            Expr::Lambda(lambda) => self.in_scope(&Scope::Lambda(lambda), |walker| visitor::walk_expr(walker, expr)),
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                let outer = std::mem::replace(&mut self.in_comprehension, true);
                visitor::walk_expr(self, expr);
                self.in_comprehension = outer;
            }
            Expr::FString(_) => {
                let outer = std::mem::replace(&mut self.in_fstring, true);
                visitor::walk_expr(self, expr);
                self.in_fstring = outer;
            }
            _ => visitor::walk_expr(self, expr),
        }
        self.expressions.pop();
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        registry::except_handler(self.rules, self, handler);
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        registry::pattern(self.rules, self, pattern);
        visitor::walk_pattern(self, pattern);
    }

    fn visit_type_param(&mut self, type_param: &'a TypeParam) {
        registry::type_param(self.rules, self, type_param);
        visitor::walk_type_param(self, type_param);
    }
}

impl<'a> Walker<'_, 'a> {
    // The owner's own handlers have run in the scope around it; the scope's handlers run before any part of it is
    // walked.
    fn in_scope(&mut self, scope: &Scope<'a>, walk: impl FnOnce(&mut Self)) {
        self.reported_names.push(FxHashSet::default());
        registry::scope(self.rules, self, scope);
        walk(self);
        self.reported_names.pop();
    }
}
