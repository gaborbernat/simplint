use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_function;

pub fn conditional_in_test(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let mut finder = Conditionals { found: Vec::new() };
    for function in checker.function_defs() {
        if is_test_function(function) {
            finder.visit_body(&function.body);
        }
    }
    let mut found = finder.found;
    found.sort_by_key(Ranged::start);
    found.dedup();
    for range in found {
        checker.report(Rule::ConditionalInTest, range, "Branching inside a test; parameterize instead");
    }
}

struct Conditionals {
    found: Vec<TextRange>,
}

impl<'a> Visitor<'a> for Conditionals {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::If(_) | Stmt::Match(_) => {
                self.found.push(stmt.range());
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::If(_) => {
                self.found.push(expr.range());
                visitor::walk_expr(self, expr);
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}
