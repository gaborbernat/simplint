use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::collected_tests;

pub fn test_return_value(checker: &Checker) {
    if !checker.is_test_module() || !checker.settings.project.pytest.collects(checker.file_name()) {
        return;
    }
    // `test_suite` builds the suite a unittest runner loads, and is not a test.
    for test in collected_tests(checker).into_iter().filter(|test| test.name.as_str() != "test_suite") {
        let mut finder = Returns { returns: Vec::new(), yields: false };
        finder.visit_body(&test.body);
        // A generator's `return` value ends the iteration; pytest does not collect such a function as a plain test.
        if finder.yields {
            continue;
        }
        for (range, value) in finder.returns {
            // A comparison or a boolean test reads as the assertion the author meant.
            let fix = matches!(value, Expr::Compare(_) | Expr::BoolOp(_))
                .then(|| Fix::unsafe_edit(Edit::range_replacement("assert".to_owned(), keyword(range))));
            checker.report_fix(
                Rule::TestReturnValue,
                range,
                "pytest ignores what a test returns, so this test passes whatever the value is",
                fix,
                "Assert the value instead of returning it",
            );
        }
    }
}

fn keyword(statement: TextRange) -> TextRange {
    TextRange::at(statement.start(), TextSize::from(6))
}

struct Returns<'a> {
    returns: Vec<(TextRange, &'a Expr)>,
    yields: bool,
}

impl<'a> Visitor<'a> for Returns<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => return,
            // A test that returns a helper's result delegates to the helper, which checks and returns `None`.
            Stmt::Return(ast::StmtReturn { value: Some(value), .. })
                if !value.is_none_literal_expr() && !value.is_call_expr() && !value.is_await_expr() =>
            {
                self.returns.push((stmt.range(), value));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => return,
            Expr::Yield(_) | Expr::YieldFrom(_) => self.yields = true,
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
