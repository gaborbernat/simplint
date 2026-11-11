mod all_format;
mod all_list;
mod all_not_last;
mod init_module_content;
mod missing_from_all;
mod module_variable_prefix;
mod test_only_code;
mod test_only_parameter;
mod unexported_public_name;
mod unused_definition;
mod unused_export;
mod unused_return_value;

pub(crate) use all_format::all_format;
pub(crate) use all_not_last::all_not_last;
pub(crate) use init_module_content::{init_module_content, is_type_checking};
pub(crate) use missing_from_all::missing_from_all;
pub(crate) use module_variable_prefix::module_variable_prefix;
pub(crate) use test_only_code::test_only_code;
pub(crate) use test_only_parameter::test_only_parameter;
pub(crate) use unexported_public_name::unexported_public_name;
pub(crate) use unused_definition::unused_definition;
pub(crate) use unused_export::unused_export;
pub(crate) use unused_return_value::unused_return_value;

use all_list::{AllList, exploded, is_alone_on_line};
use ruff_python_ast as ast;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::syntax::ReturnVisitor;

// The module uses the name outside its own definition, so the module needs it whatever the tests do.
pub(super) fn references_outside(checker: &Checker, name: &str, definition: TextRange) -> bool {
    checker.semantic().references_to(name).iter().any(|reference| !definition.contains_range(reference.range))
}

// The function's `return` statements hand back a value other than `None`, which a caller may read. A method that only
// returns `self` offers chaining, a style of its interface rather than a value, so it counts as returning none.
pub(super) fn returns_value(function: &ast::StmtFunctionDef) -> bool {
    let mut returns = ReturnVisitor::default();
    returns.visit_body(&function.body);
    let values: Vec<&ast::Expr> = returns
        .returns
        .iter()
        .filter_map(|stmt| stmt.value.as_deref())
        .filter(|value| !value.is_none_literal_expr())
        .collect();
    !values.is_empty()
        && !values.iter().all(|value| value.as_name_expr().is_some_and(|name| name.id.as_str() == "self"))
}
