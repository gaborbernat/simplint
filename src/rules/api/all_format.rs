use ruff_diagnostics::{Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{self as ast, Expr};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::api::exploded;
use crate::syntax::{AllBinding, all_binding, module_statements};

pub fn all_format(checker: &Checker) {
    for stmt in checker.body() {
        let Some(AllBinding { value: Some(value), operator }) = all_binding(stmt) else {
            continue;
        };
        // `tuple += list` raises, so an augmented binding is left alone while a plain binding keeps `__all__` a tuple.
        if operator.is_some() && keeps_a_tuple(checker) {
            continue;
        }
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
            .find(|token| !matches!(token.kind(), TokenKind::NonLogicalNewline | TokenKind::Comment | TokenKind::Rpar))
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
                // Ruff's COM812 reports a list whose closing bracket follows a line break with no comma before it.
                if checker.settings.from_ruff.ruff_runs(RuffRule::MissingTrailingComma)
                    && closing_bracket_follows_line_break(checker, last, value)
                {
                    continue;
                }
                let edit = if multiline && closes_on_own_line || has_comment {
                    Edit::insertion(",".to_owned(), checker.parenthesized_range(last, value.into()).end())
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
            let edits = (comma.is_none() && magic)
                .then(|| Edit::insertion(",".to_owned(), checker.parenthesized_range(last, value.into()).end()));
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

// Whether a plain binding of `__all__` stays a tuple after the rule: it converts only a tuple at the top level that no
// `noqa` suppresses, so a tuple nested in a block or suppressed, and any value that is not a literal, keeps it one.
fn keeps_a_tuple(checker: &Checker) -> bool {
    module_statements(checker.body())
        .filter_map(|stmt| Some((stmt, all_binding(stmt)?)))
        .filter(|(_, binding)| binding.operator.is_none())
        .any(|(stmt, binding)| match binding.value {
            None | Some(Expr::List(_)) => false,
            Some(value @ Expr::Tuple(_)) => {
                let top_level = checker.body().iter().any(|top| top.range() == stmt.range());
                !top_level || checker.is_suppressed(Rule::AllFormat, value.range())
            }
            Some(_) => true,
        })
}

// Whether the last token before the closing bracket of a list is a line break, past any comment, as COM812 sees it.
fn closing_bracket_follows_line_break(checker: &Checker, last: &Expr, value: &Expr) -> bool {
    let mut kinds = checker
        .parsed
        .tokens()
        .in_range(TextRange::new(last.end(), value.end()))
        .iter()
        .map(Token::kind)
        .filter(|kind| *kind != TokenKind::Comment)
        .rev();
    kinds.next() == Some(TokenKind::Rsqb) && kinds.next() == Some(TokenKind::NonLogicalNewline)
}
