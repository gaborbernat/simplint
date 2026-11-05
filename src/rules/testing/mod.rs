mod conditional_in_test;
mod constant_assert;
mod mock_attribute_assignment;
pub(crate) mod mock_without_spec;
mod parametrize_without_id;
mod sleep_in_test;
mod split_composite_assert;
mod stdlib_mock;
mod test_class;
pub(crate) mod up_unittest_assert_helper;
mod up_unittest_enter_context;

pub(crate) use conditional_in_test::conditional_in_test;
pub(crate) use constant_assert::constant_assert;
pub(crate) use mock_attribute_assignment::mock_attribute_assignment;
pub(crate) use parametrize_without_id::parametrize_without_id;
pub(crate) use sleep_in_test::sleep_in_test;
pub(crate) use split_composite_assert::split_composite_assert;
pub(crate) use stdlib_mock::stdlib_mock;
pub(crate) use test_class::test_class;
pub(crate) use up_unittest_enter_context::up_unittest_enter_context;

use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;

// pytest collects the functions whose name starts with `test`.
fn is_test_function(function: &ast::StmtFunctionDef) -> bool {
    function.name.as_str().starts_with("test")
}

// The callables that build a mock, which the testing rules and the receiver tracking of the project share.
pub(crate) const MOCK_CLASSES: &[&str] =
    &["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock", "PropertyMock", "create_autospec"];

// A class that visibly derives from `unittest.TestCase`; a base under another name may define the assert helpers
// differently.
pub(crate) fn is_test_case(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.arguments.as_ref().is_some_and(|arguments| {
        arguments.args.iter().any(|base| checker.semantic().resolves_to(base, &["unittest", "TestCase"]))
    })
}

pub(crate) fn in_test_case(checker: &Checker, range: TextRange) -> bool {
    checker.class_defs().iter().any(|class| class.range().contains_range(range) && is_test_case(checker, class))
}
