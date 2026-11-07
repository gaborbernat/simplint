use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
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
            Expr::ListComp(ast::ExprListComp { elt, generators, .. })
            | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
            | Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => {
                self.visit_element(elt);
                for generator in generators {
                    self.visit_comprehension(generator);
                }
            }
            Expr::DictComp(ast::ExprDictComp { key, value, generators, .. }) => {
                if let Some(key) = key {
                    self.visit_element(key);
                }
                self.visit_element(value);
                for generator in generators {
                    self.visit_comprehension(generator);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

impl<'a> Conditionals {
    // Like an `if` filter, a conditional that is a comprehension's item shapes data, so only its parts are visited.
    fn visit_element(&mut self, element: &'a Expr) {
        if let Expr::If(ast::ExprIf { test, body, orelse, .. }) = element {
            self.visit_expr(test);
            self.visit_expr(body);
            self.visit_expr(orelse);
        } else {
            self.visit_expr(element);
        }
    }
}
