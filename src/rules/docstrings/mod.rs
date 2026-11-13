mod function_docstring;
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
mod rst_unraised_exception;
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
pub(crate) use rst_unraised_exception::rst_unraised_exception;
pub(crate) use test_module_docstring::test_module_docstring;
pub(crate) use undocumented_public_api::undocumented_public_api;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::map_callable;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::{self as ast, Decorator, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::comments::is_tool_directive;
use crate::rules::edits::{Blanks, statement_deletion};

// Ruff's pydocstyle rules skip these functions, so this rule does too.
fn ruff_ignores_function(checker: &Checker, decorators: &[Decorator]) -> bool {
    let ignored = &checker.settings.from_ruff.ignore_decorators;
    decorators.iter().any(|decorator| {
        checker
            .semantic()
            .ruff_qualified_name(map_callable(&decorator.expression), checker.module_path())
            .is_some_and(|name| ignored.iter().any(|listed| QualifiedName::from_dotted_name(listed) == name))
    })
}

// Ruff's D419 reads the text from the source, so an escape such as `\x20` counts as text. It skips implicitly
// concatenated docstrings.
fn ruff_reports_empty_docstring(checker: &Checker, literal: &ast::ExprStringLiteral) -> bool {
    literal.as_single_part_string().is_some_and(|part| checker.text(part.content_range()).trim().is_empty())
}

fn raised_class(exc: &Expr) -> &Expr {
    map_callable(exc)
}

// Sphinx autodoc reads a string after a variable, or a `#:` comment above it or at the end of its line.
fn is_documented_variable(checker: &Checker, stmt: &Stmt, next: Option<&Stmt>) -> bool {
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}

fn docstring_deletion(checker: &Checker, body: &[Stmt]) -> Option<Edit> {
    let stmt = &body[0];
    let deletion = statement_deletion(checker, stmt.range(), body.get(1).map(Ranged::range), Blanks::Trailing)?;
    let comment = checker.trailing_comment(stmt).filter(|comment| !is_directive_comment(checker, *comment));
    match comment {
        Some(comment) if checker.is_alone_on_lines(stmt.range()) => {
            let newline = checker.line_ending();
            let line = format!("{}{}{newline}", checker.indentation(stmt), checker.text(comment));
            Some(Edit::range_replacement(line, deletion))
        }
        _ => Some(Edit::range_deletion(deletion)),
    }
}

// Such a comment covers only the docstring's line, so it goes with the docstring.
fn is_directive_comment(checker: &Checker, comment: TextRange) -> bool {
    let text = checker.text(comment);
    text.match_indices('#').any(|(start, _)| is_tool_directive(&text[start..]))
}
