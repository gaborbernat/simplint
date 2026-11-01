mod conditional_in_test;
mod constant_assert;
mod mock_attribute_assignment;
pub(crate) mod mock_without_spec;
mod parametrize_without_id;
mod test_class;

pub(crate) use conditional_in_test::conditional_in_test;
pub(crate) use constant_assert::constant_assert;
pub(crate) use mock_attribute_assignment::mock_attribute_assignment;
pub(crate) use parametrize_without_id::parametrize_without_id;
pub(crate) use test_class::test_class;

use ruff_python_ast as ast;

// pytest collects the functions whose name starts with `test`.
fn is_test_function(function: &ast::StmtFunctionDef) -> bool {
    function.name.as_str().starts_with("test")
}

const MOCK_CLASSES: &[&str] = &["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock"];
