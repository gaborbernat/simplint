use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Pattern, Stmt, TypeParam};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::{self, RuleSet};

// One walk over the module drives the node rules, as Ruff's `Checker` drives its own: each node goes to the handlers
// its kind has among the enabled rules, which read where the node sits from the walker instead of walking again.
pub struct Walker<'c, 'a> {
    pub checker: &'c Checker<'a>,
    rules: RuleSet,
    // The expressions holding the current one, outermost first, ending with the current one.
    expressions: Vec<&'a Expr>,
    // The scopes holding the current node, outermost first, the module left out.
    scopes: Vec<Scope<'a>>,
    // The statements holding the current node, outermost first.
    statements: Vec<&'a Stmt>,
    in_fstring: bool,
    in_comprehension: bool,
}

// A def, class or lambda opens a scope covering its header, decorators and default values included.
#[derive(Clone, Copy)]
pub enum Scope<'a> {
    Function(&'a ast::StmtFunctionDef),
    Class(&'a ast::StmtClassDef),
    Lambda(&'a ast::ExprLambda),
}

impl Ranged for Scope<'_> {
    fn range(&self) -> TextRange {
        match self {
            Self::Function(function) => function.range,
            Self::Class(class) => class.range,
            Self::Lambda(lambda) => lambda.range,
        }
    }
}

impl Scope<'_> {
    // The body of the scope, which its header's decorators, defaults and bases do not belong to.
    pub fn body_range(&self) -> TextRange {
        let (body, end) = match self {
            Self::Function(function) => (&function.body, function.end()),
            Self::Class(class) => (&class.body, class.end()),
            Self::Lambda(lambda) => return lambda.body.range(),
        };
        TextRange::new(body.first().map_or(end, Ranged::start), end)
    }
}

impl<'c, 'a> Walker<'c, 'a> {
    pub fn run(checker: &'c Checker<'a>, rules: RuleSet) {
        let mut walker = Self {
            checker,
            rules,
            expressions: Vec::new(),
            scopes: Vec::new(),
            statements: Vec::new(),
            in_fstring: false,
            in_comprehension: false,
        };
        walker.visit_body(checker.body());
    }

    pub fn expression_parent(&self) -> Option<&'a Expr> {
        self.expressions.iter().rev().nth(1).copied()
    }

    pub fn expressions(&self) -> &[&'a Expr] {
        &self.expressions
    }

    pub fn scopes(&self) -> &[Scope<'a>] {
        &self.scopes
    }

    // The innermost statement holding the current node.
    pub fn statement(&self) -> Option<&'a Stmt> {
        self.statements.last().copied()
    }

    pub fn in_fstring(&self) -> bool {
        self.in_fstring
    }

    pub fn in_comprehension(&self) -> bool {
        self.in_comprehension
    }
}

impl<'a> Visitor<'a> for Walker<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.statements.push(stmt);
        registry::statement(self.rules, self, stmt);
        match stmt {
            Stmt::FunctionDef(function) => {
                self.in_scope(&Scope::Function(function), |walker| visitor::walk_stmt(walker, stmt));
            }
            Stmt::ClassDef(class) => self.in_scope(&Scope::Class(class), |walker| visitor::walk_stmt(walker, stmt)),
            _ => visitor::walk_stmt(self, stmt),
        }
        self.statements.pop();
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
        registry::scope(self.rules, self, scope);
        self.scopes.push(*scope);
        walk(self);
        self.scopes.pop();
    }
}
