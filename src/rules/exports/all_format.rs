use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Expr};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::exploded;
use crate::rules::exports::{AllBinding, all_binding};

pub fn all_format(checker: &Checker) {
    for stmt in checker.body() {
        let Some(AllBinding { value: Some(value), .. }) = all_binding(stmt) else {
            continue;
        };
        let (elements, is_list) = match value {
            Expr::List(ast::ExprList { elts, .. }) => (elts, true),
            Expr::Tuple(ast::ExprTuple { elts, .. }) => (elts, false),
            _ => continue,
        };
        let Some(last) = elements.last() else {
            if !is_list {
                let fix = Fix::unsafe_edit(Edit::range_replacement("[]".to_owned(), value.range()));
                checker.report_fix(
                    Rule::AllFormat,
                    value.range(),
                    "`__all__` is a tuple; make it a list",
                    Some(fix),
                    "Convert `__all__` to a list",
                );
            }
            continue;
        };
        let comma = checker
            .parsed
            .tokens()
            .after(last.end())
            .iter()
            .find(|token| !matches!(token.kind(), TokenKind::NonLogicalNewline | TokenKind::Comment))
            .filter(|token| token.kind() == TokenKind::Comma);
        let multiline = checker.source.contains_line_break(value.range());
        let has_comment = checker.has_comment_in(value.range());
        let items: Vec<String> = elements.iter().map(|elt| checker.text(elt.range()).to_owned()).collect();
        let base = leading_indentation(checker.source.full_line_str(value.start()));
        // With `skip-magic-trailing-comma`, `ruff format` drops the comma from a list that fits on one line.
        let magic = checker.settings.from_ruff.magic_trailing_comma;
        if is_list {
            if comma.is_none() && magic {
                // A comma right before a closing bracket on the same line is one Ruff's COM819 removes again.
                let closes_on_own_line = checker.source[TextRange::new(last.end(), value.end())].contains('\n');
                let edit = if multiline && closes_on_own_line || has_comment {
                    Edit::insertion(",".to_owned(), last.end())
                } else {
                    Edit::range_replacement(exploded(checker, base, &items), value.range())
                };
                checker.report_fix(
                    Rule::AllFormat,
                    value.range(),
                    "`__all__` has no trailing comma; add one so the formatter keeps one name per line",
                    Some(Fix::safe_edit(edit)),
                    "Add a trailing comma",
                );
            }
            continue;
        }
        // A list holds the same names; only code that concatenates `__all__` with a tuple would notice the change.
        let parenthesized = value.as_tuple_expr().is_some_and(|tuple| tuple.parenthesized);
        let fix = if parenthesized && (multiline || has_comment) {
            // Swapping the brackets keeps the layout of a tuple written one name per line.
            let open = Edit::range_replacement("[".to_owned(), TextRange::at(value.start(), TextSize::from(1)));
            let close = Edit::range_replacement(
                "]".to_owned(),
                TextRange::at(value.end() - TextSize::from(1), TextSize::from(1)),
            );
            let edits = (comma.is_none() && magic).then(|| Edit::insertion(",".to_owned(), last.end()));
            Fix::unsafe_edits(open, std::iter::once(close).chain(edits))
        } else if magic {
            Fix::unsafe_edit(Edit::range_replacement(exploded(checker, base, &items), value.range()))
        } else {
            Fix::unsafe_edit(Edit::range_replacement(format!("[{}]", items.join(", ")), value.range()))
        };
        let message = if magic {
            "`__all__` is a tuple; make it a list with a trailing comma"
        } else {
            "`__all__` is a tuple; make it a list"
        };
        checker.report_fix(Rule::AllFormat, value.range(), message, Some(fix), "Convert `__all__` to a list");
    }
}
