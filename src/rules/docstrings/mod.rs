mod comment_text;
mod function_docstring;
mod lines;
mod long_docstring_paragraph;
pub(crate) mod rst;
mod rst_attribute_kind;
mod rst_duplicate_param;
mod rst_empty_field;
mod rst_field_syntax;
mod rst_inline_markup;
mod rst_missing_attribute;
mod rst_missing_param;
mod rst_missing_raises;
mod rst_param_order;
mod rst_private_field;
mod rst_restating_field;
mod rst_returns;
mod rst_type_field;
mod rst_unknown_attribute;
mod rst_unknown_param;
mod test_module_docstring;
mod undocumented_public_api;

pub(crate) use function_docstring::function_docstring;
pub(crate) use long_docstring_paragraph::long_docstring_paragraph;
pub(crate) use rst_attribute_kind::rst_attribute_kind;
pub(crate) use rst_duplicate_param::rst_duplicate_param;
pub(crate) use rst_empty_field::rst_empty_field;
pub(crate) use rst_field_syntax::rst_field_syntax;
pub(crate) use rst_inline_markup::rst_inline_markup;
pub(crate) use rst_missing_attribute::rst_missing_attribute;
pub(crate) use rst_missing_param::rst_missing_param;
pub(crate) use rst_missing_raises::rst_missing_raises;
pub(crate) use rst_param_order::rst_param_order;
pub(crate) use rst_private_field::rst_private_field;
pub(crate) use rst_restating_field::rst_restating_field;
pub(crate) use rst_returns::rst_returns;
pub(crate) use rst_type_field::rst_type_field;
pub(crate) use rst_unknown_attribute::rst_unknown_attribute;
pub(crate) use rst_unknown_param::rst_unknown_param;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::comments::is_tool_directive;
use crate::rules::edits::{Blanks, statement_deletion};
use crate::rules::{indentation, is_alone_on_lines};

// A variable is documented the two ways Sphinx autodoc reads: a string right after it, or `#:` comments right above.
fn is_documented_variable(checker: &Checker, stmt: &Stmt, next: Option<&Stmt>) -> bool {
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}

// Whether a comment after a docstring is a pragma or a tool directive. Such a comment covers the docstring's line only,
// so it goes with the docstring.
pub(crate) fn is_directive_comment(checker: &Checker, comment: TextRange) -> bool {
    let text = checker.text(comment);
    text.match_indices('#').any(|(start, _)| is_tool_directive(&text[start..]))
}

// Deletes the docstring that starts a body. A comment after it stays on a line of its own, unless it is a pragma or a
// tool directive, which goes with it.
pub(crate) fn docstring_deletion(checker: &Checker, body: &[Stmt]) -> Option<Edit> {
    let stmt = &body[0];
    let deletion = statement_deletion(checker, stmt.range(), body.get(1).map(Ranged::range), Blanks::Trailing)?;
    let comment = checker.trailing_comment(stmt).filter(|comment| !is_directive_comment(checker, *comment));
    match comment {
        Some(comment) if is_alone_on_lines(checker.source, stmt.range()) => {
            let newline = checker.line_ending();
            let line = format!("{}{}{newline}", indentation(checker.source, stmt), checker.text(comment));
            Some(Edit::range_replacement(line, deletion))
        }
        _ => Some(Edit::range_deletion(deletion)),
    }
}
