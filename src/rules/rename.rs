use std::borrow::Cow;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast as ast;
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::rules::edits::{fix_of, keep_debug_text};
use crate::syntax::{Visibility, identifier_words};

pub(crate) fn kept_message(name: &str, new: &str, listed: bool) -> String {
    if listed {
        format!("`{name}` is in `__all__`; rename it to `{new}` by hand")
    } else {
        format!("`{name}` is not in `__all__`; list it there, or rename it to `{new}` by hand")
    }
}

pub(crate) fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
    let ranges = checker.semantic().name_ranges(old);
    // In `zip = zip`, renaming both occurrences would also rename the reference to the builtin.
    if is_offered(checker, old)
        || checker.name_token_count(new) > 0
        || is_python_builtin(old, checker.target_version().minor, false)
        || checker.project().is_used_elsewhere(old)
        || checker.project().is_read_opaquely(old)
        // A computed write may set any name of the module, as `setattr(module, name, value)` does.
        || checker.project().is_computed_written()
        // Compiled code can import any name by its spelling, which no index of the Python files sees.
        || checker.settings.project.metadata.native
        || ranges.is_empty()
        || checker.name_token_count(old) != ranges.len()
        || appears_in_strings(checker, old)
        // A name in a t-string keeps its source text, which a new spelling would change.
        || ranges.iter().any(|range| checker.state().in_source_text(checker, *range))
    {
        return None;
    }
    let exports = checker.state().export_strings(checker);
    let mut renamed_exports = Vec::new();
    for element in exports.iter().filter(|element| element.value.to_str() == old) {
        // A concatenated export such as `"lim" "it"` spans multiple string tokens.
        let [part] = element.value.as_slice() else {
            return None;
        };
        renamed_exports.push(part.content_range());
    }
    let edits = ranges.into_iter().chain(renamed_exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    fix_of(keep_debug_text(checker, edits.collect())?, Applicability::Unsafe)
}

fn is_offered(checker: &Checker, name: &str) -> bool {
    let project = checker.project();
    project.is_library_api(name) || project.role().public && Visibility::of(name) == Visibility::Public
}

// Compare decoded strings because formatting can change the spelling of an escape without changing its value.
// A string in another file may look up the old name. This file's `__all__` entries can be edited with the rename.
pub(crate) fn appears_in_strings(checker: &Checker, name: &str) -> bool {
    let words = checker.state().string_words(checker);
    name.chars().all(is_word_char)
        && (words.outside_exports.contains(name)
            || checker.project().string_word_count(name) > u32::from(words.all.contains(name)))
}

fn is_word_char(char: char) -> bool {
    char.is_alphanumeric() || char == '_'
}

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
