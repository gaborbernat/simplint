// The rename fix module-level naming rules share.

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::rules::{keep_debug_text, source_text_spans};
use crate::syntax::all_literal;

pub fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
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
    // A name in a t-string keeps its source text, which a new spelling would change.
    let source_spans = source_text_spans(checker.body());
    if ranges.iter().any(|range| source_spans.iter().any(|span| span.contains_range(*range))) {
        return None;
    }
    let renamed_exports = exports
        .into_iter()
        .filter(|element| element.value.to_str() == old)
        .filter_map(|element| Some(element.value.iter().next()?.content_range()));
    let edits = ranges.into_iter().chain(renamed_exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    let mut edits = keep_debug_text(checker.source, checker.body(), edits.collect())?.into_iter();
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

fn all_elements(body: &[Stmt]) -> Vec<&ast::ExprStringLiteral> {
    body.iter()
        .filter_map(all_literal)
        .flat_map(|(_, elements)| elements)
        .filter_map(Expr::as_string_literal_expr)
        .collect()
}
