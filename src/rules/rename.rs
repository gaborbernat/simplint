// The rename fix module-level naming rules share.

use std::borrow::Cow;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast as ast;
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::rules::edits::{fix_of, keep_debug_text};
use crate::syntax::{Visibility, identifier_words};

// The report for a name a public module keeps: its users import it, so the author lists it or renames it by hand, or,
// when `__all__` lists it already, renames it by hand.
pub(crate) fn kept_message(name: &str, new: &str, listed: bool) -> String {
    if listed {
        format!("`{name}` is in `__all__`; rename it to `{new}` by hand")
    } else {
        format!("`{name}` is not in `__all__`; list it there, or rename it to `{new}` by hand")
    }
}

// A name a library's users reach: the library's API, or a public name of a public module.
fn is_offered(checker: &Checker, name: &str) -> bool {
    let project = checker.project();
    project.is_library_api(name) || project.role().public && Visibility::of(name) == Visibility::Public
}

pub fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
    let ranges = checker.semantic().name_ranges(old);
    // A module name that shadows a builtin, as in `zip = zip`, shares its token with the builtin it reads.
    if is_offered(checker, old)
        || checker.name_token_count(new) > 0
        || is_python_builtin(old, checker.target_version().minor, false)
        || checker.project().is_used_elsewhere(old)
        // Code the index cannot follow may read the name, so only the fix is withheld, never the report.
        || checker.project().is_read_opaquely(old)
        // A computed write may set any name of the module, as `setattr(module, name, value)` does.
        || checker.project().is_computed_written()
        || ranges.is_empty()
        || checker.name_token_count(old) != ranges.len()
    {
        return None;
    }
    let exports = checker.state().export_strings(checker);
    if appears_in_strings(checker, old) {
        return None;
    }
    // A name in a t-string keeps its source text, which a new spelling would change.
    let source_spans = checker.state().source_text_spans(checker);
    if ranges.iter().any(|range| source_spans.iter().any(|span| span.contains_range(*range))) {
        return None;
    }
    let renamed_exports = exports
        .iter()
        .filter(|element| element.value.to_str() == old)
        .filter_map(|element| Some(element.value.iter().next()?.content_range()));
    let edits = ranges.into_iter().chain(renamed_exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    fix_of(keep_debug_text(checker, edits.collect())?, Applicability::Unsafe)
}

// The text a string holds, not its source: `ruff format` rewrites escapes, as `\N{latin small letter x}` into
// `\N{LATIN SMALL LETTER X}`, so a word in the source can come or go while the value stays.
// The strings of other files count too, as a test or a generated template may look the name up by its spelling. A
// file's own export list names the name without reaching it, so only its other strings and the strings of other files
// block the rename.
pub(crate) fn appears_in_strings(checker: &Checker, name: &str) -> bool {
    let words = checker.state().string_words(checker);
    // A word is a run of letters, digits and underscores, so a name with any other character is never one.
    name.chars().all(is_word_char)
        && (words.outside_exports.contains(name)
            || checker.project().string_word_count(name) > u32::from(words.all.contains(name)))
}

// The words of a file's strings, as runs of letters, digits and underscores, which is how a name appears in a string
// that spells it. `outside_exports` holds those of the strings outside the export lists, which block a rename alone.
#[derive(Default)]
pub(super) struct StringWords {
    all: FxHashSet<String>,
    outside_exports: FxHashSet<String>,
}

impl StringWords {
    pub(super) fn of(values: &[(TextRange, Cow<'_, str>)], exports: &[&ast::ExprStringLiteral]) -> Self {
        let mut words = Self::default();
        for (range, text) in values {
            let outside = !exports.iter().any(|export| export.range().contains_range(*range));
            for word in identifier_words(text).filter(|word| !word.is_empty()) {
                if outside && !words.outside_exports.contains(word) {
                    words.outside_exports.insert(word.to_owned());
                }
                if !words.all.contains(word) {
                    words.all.insert(word.to_owned());
                }
            }
        }
        words
    }
}

fn is_word_char(char: char) -> bool {
    char.is_alphanumeric() || char == '_'
}
