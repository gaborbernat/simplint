mod module_variable_case;
pub(crate) mod single_letter_name;
mod unexported_public_name;

pub(crate) use module_variable_case::module_variable_case;
pub(crate) use unexported_public_name::unexported_public_name;

use std::borrow::Cow;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::exports::all_literal;

fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
    let ranges = checker.semantic().name_ranges(old);
    // A module name that shadows a builtin, as in `zip = zip`, shares its token with the builtin it reads.
    if checker.name_token_count(new) > 0
        || is_python_builtin(old, checker.settings.from_ruff.target_version.minor, false)
        || checker.project().is_referenced(old)
        || ranges.is_empty()
        || checker.name_token_count(old) != ranges.len()
    {
        return None;
    }
    let exports = all_elements(checker.body());
    if appears_in_strings(checker, old, &exports) {
        return None;
    }
    let renamed_exports = exports
        .into_iter()
        .filter(|element| element.value.to_str() == old)
        .filter_map(|element| Some(element.value.iter().next()?.content_range()));
    let mut edits =
        ranges.into_iter().chain(renamed_exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    Some(Fix::unsafe_edits(edits.next()?, edits))
}

// The text a string holds, not its source: `ruff format` rewrites escapes, as `\N{latin small letter x}` into
// `\N{LATIN SMALL LETTER X}`, so a word in the source can come or go while the value stays.
fn appears_in_strings(checker: &Checker, name: &str, exports: &[&ast::ExprStringLiteral]) -> bool {
    // A word is a run of letters, digits and underscores, so a name with any other character is never one.
    name.chars().all(is_word_char)
        && checker.string_values().iter().any(|(range, text)| {
            !exports.iter().any(|export| export.range().contains_range(*range)) && contains_word(text, name)
        })
}

// Searching for the word and checking the characters around each match is far cheaper than splitting the text into
// words, and a match inside a longer word cannot hide a whole one, which starts after a separator.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        !text[..start].chars().next_back().is_some_and(is_word_char)
            && !text[start + word.len()..].chars().next().is_some_and(is_word_char)
    })
}

fn is_word_char(char: char) -> bool {
    char.is_alphanumeric() || char == '_'
}

// The text of every string and bytes literal of the module, and of each literal part of an f-string or t-string, with
// its range.
pub(crate) fn string_values(body: &[Stmt]) -> Vec<(TextRange, Cow<'_, str>)> {
    let mut values = StringValues(Vec::new());
    values.visit_body(body);
    values.0
}

struct StringValues<'a>(Vec<(TextRange, Cow<'a, str>)>);

impl<'a> Visitor<'a> for StringValues<'a> {
    fn visit_string_literal(&mut self, string_literal: &'a ast::StringLiteral) {
        self.0.push((string_literal.range, Cow::Borrowed(&string_literal.value)));
    }

    fn visit_bytes_literal(&mut self, bytes_literal: &'a ast::BytesLiteral) {
        self.0.push((bytes_literal.range, String::from_utf8_lossy(&bytes_literal.value)));
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        if let ast::InterpolatedStringElement::Literal(literal) = element {
            self.0.push((literal.range, Cow::Borrowed(&literal.value)));
        }
        visitor::walk_interpolated_string_element(self, element);
    }
}

fn all_elements(body: &[Stmt]) -> Vec<&ast::ExprStringLiteral> {
    body.iter()
        .filter_map(all_literal)
        .flat_map(|(_, elements)| elements)
        .filter_map(Expr::as_string_literal_expr)
        .collect()
}
