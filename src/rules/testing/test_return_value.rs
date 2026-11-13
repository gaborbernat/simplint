use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::Expr;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::collected_tests;
use crate::syntax::ReturnVisitor;

pub fn test_return_value(checker: &Checker) {
    if !checker.is_test_module() || !checker.settings.project.pytest.collects(checker.file_name()) {
        return;
    }
    // `test_suite` builds the suite a unittest runner loads, and is not a test.
    for test in collected_tests(checker).into_iter().filter(|test| test.name.as_str() != "test_suite") {
        let mut finder = ReturnVisitor::default();
        finder.visit_body(&test.body);
        // A generator's `return` value ends the iteration; pytest does not collect such a function as a plain test.
        if finder.is_generator {
            continue;
        }
        // A test that returns a helper's result delegates to the helper, which checks and returns `None`.
        let values = finder
            .returns
            .into_iter()
            .filter_map(|stmt| Some((stmt.range(), stmt.value.as_deref()?)))
            .filter(|(_, value)| !value.is_none_literal_expr() && !value.is_call_expr() && !value.is_await_expr());
        for (range, value) in values {
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
