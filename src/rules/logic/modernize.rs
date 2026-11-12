use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::argument_removal;

// The attribute `expr` reads from a module the name is only ever bound to, as `re.error` is read from `re`. An
// assignment or `del` patches the module, so only reads count.
pub fn module_attribute_read<'e>(checker: &Checker, expr: &'e Expr, path: &[&str]) -> Option<&'e ast::ExprAttribute> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    (attribute.ctx.is_load()
        && checker.semantic().resolves_to(expr, path)
        && checker.semantic().is_only_import_of(&attribute.value, &path[..1]))
    .then_some(attribute)
}

pub fn rename_attribute(attribute: &ast::ExprAttribute, new: &str) -> Edit {
    Edit::range_replacement(new.to_owned(), attribute.attr.range())
}

pub fn replacement_unless_commented(checker: &Checker, text: String, range: TextRange) -> Option<Edit> {
    (!checker.has_comment_in(range)).then(|| Edit::range_replacement(text, range))
}

pub fn report_redundant_keyword(
    checker: &Checker,
    call: &ast::ExprCall,
    keyword: &ast::Keyword,
    rule: Rule,
    applicability: Applicability,
    message: &str,
    title: &str,
) {
    let fix = argument_removal(checker, call, keyword.range())
        .map(|range| Fix::applicable_edit(Edit::range_deletion(range), applicability));
    checker.report_fix(rule, keyword.range(), message, fix, title);
}
