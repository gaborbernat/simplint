mod all_format;
mod all_not_last;
mod init_module_content;
mod missing_from_all;
mod module_variable_prefix;
mod test_only_code;
mod unexported_public_name;
mod unused_export;

pub(crate) use all_format::all_format;
pub(crate) use all_not_last::all_not_last;
pub(crate) use init_module_content::init_module_content;
pub(crate) use missing_from_all::missing_from_all;
pub(crate) use module_variable_prefix::module_variable_prefix;
pub(crate) use test_only_code::test_only_code;
pub(crate) use unexported_public_name::unexported_public_name;
pub(crate) use unused_export::unused_export;

use crate::lint::checker::Checker;

// The one-name-per-line layout `ruff format` gives a list with a trailing comma, so the fix needs no formatter pass.
fn exploded(checker: &Checker, base: &str, items: &[String]) -> String {
    let indent = checker.stylist.indentation().as_str();
    let lines = items.iter().fold(String::new(), |text, item| text + base + indent + item + ",\n");
    format!("[\n{lines}{base}]")
}
