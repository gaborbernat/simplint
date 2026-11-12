mod conditional_in_test;
mod constant_assert;
mod loop_in_test;
mod mock_assertion_typo;
mod mock_attribute_assignment;
pub(crate) mod mock_without_spec;
mod parametrize_without_id;
pub(crate) mod recognize;
mod sleep_in_test;
mod split_composite_assert;
mod stdlib_mock;
mod test_class;
mod test_process_state;
mod test_return_value;
mod test_without_assertion;
mod uncollected_test_module;
mod unregistered_marker;
mod unused_fixture;
mod up_pytest_deprecated_api;
mod up_unittest_enter_context;

pub(crate) use conditional_in_test::{conditional_in_test, conditionals};
pub(crate) use constant_assert::constant_assert;
pub(crate) use loop_in_test::loop_in_test;
pub(crate) use mock_assertion_typo::mock_assertion_typo;
pub(crate) use mock_attribute_assignment::mock_attribute_assignment;
pub(crate) use parametrize_without_id::parametrize_without_id;
pub(crate) use sleep_in_test::sleep_in_test;
pub(crate) use split_composite_assert::split_composite_assert;
pub(crate) use stdlib_mock::stdlib_mock;
pub(crate) use test_class::test_class;
pub(crate) use test_process_state::test_process_state;
pub(crate) use test_return_value::test_return_value;
pub(crate) use test_without_assertion::test_without_assertion;
pub(crate) use uncollected_test_module::uncollected_test_module;
pub(crate) use unregistered_marker::unregistered_marker;
pub(crate) use unused_fixture::unused_fixture;
pub(crate) use up_pytest_deprecated_api::up_pytest_deprecated_api;
pub(crate) use up_unittest_enter_context::up_unittest_enter_context;

use crate::lint::checker::Checker;
use ruff_python_ast as ast;

// pytest collects the functions whose name starts with `test`.
fn is_test_function(function: &ast::StmtFunctionDef) -> bool {
    function.name.as_str().starts_with("test")
}

pub(crate) fn is_test_module_function(checker: &Checker, function: &ast::StmtFunctionDef) -> bool {
    checker.is_test_module() && is_test_function(function)
}

// A class that visibly derives from `unittest.TestCase`; a base under another name may define the assert helpers
// differently.
pub(crate) fn is_test_case(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.arguments.as_ref().is_some_and(|arguments| {
        arguments.args.iter().any(|base| checker.semantic().resolves_to(base, &["unittest", "TestCase"]))
    })
}

// A class pytest collects tests from, named `Test*`, or one deriving from `unittest.TestCase`.
fn is_test_class(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.name.as_str().starts_with("Test") || is_test_case(checker, class)
}

// The test functions pytest collects from the module: `test*` functions at module level and in test classes, other
// than fixtures, which pytest calls to set a test up.
fn collected_tests<'a>(checker: &Checker<'a>) -> Vec<&'a ast::StmtFunctionDef> {
    let is_test = |function: &ast::StmtFunctionDef| {
        is_test_function(function) && !function.decorator_list.iter().any(recognize::is_fixture)
    };
    let mut tests = Vec::new();
    for stmt in checker.body() {
        match stmt {
            ast::Stmt::FunctionDef(function) if is_test(function) => tests.push(function),
            ast::Stmt::ClassDef(class) if is_test_class(checker, class) => {
                tests.extend(class.body.iter().filter_map(ast::Stmt::as_function_def_stmt).filter(|method| is_test(method)));
            }
            _ => {}
        }
    }
    tests
}
