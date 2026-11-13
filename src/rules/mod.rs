pub(crate) mod api;
pub(crate) mod call_sites;
pub(crate) mod callee;
pub(crate) mod classes;
pub(crate) mod comments;
pub(crate) mod docstrings;
pub(crate) mod edits;
pub(crate) mod exception_classes;
mod frameworks;
pub(crate) mod imports;
pub(crate) mod layout;
pub(crate) mod logic;
pub(crate) mod module_variable;
pub(crate) mod naming;
mod public_api;
mod rename;
mod state;
pub(crate) mod testing;
pub(crate) mod typing;

pub(crate) use state::FileState;

use ruff_python_ast::Expr;
use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::syntax::is_type_checking_name;

// A doctest runs in its module's globals, so an example that names a definition of the module uses it, whether or not
// the example imports it.
pub(crate) fn is_named_by_doctest(checker: &Checker, name: &str) -> bool {
    checker.state().doctest_words(checker).contains(name)
}

// Whether the file is a Sphinx configuration, whose module variables Sphinx reads by their names and types.
pub(crate) fn is_sphinx_conf(checker: &Checker) -> bool {
    checker.file_name() == "conf.py"
}

// A class name is CapWords, which starts with an uppercase letter once leading underscores are dropped, and is not all
// caps, as PEP 8 spells classes.
pub(crate) fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

// The paragraphs of prose longer than `limit` words, each reported once from its first line to the line that passes
// the limit. Each item is a line's range and text, or `None` where a line ends the paragraph. A heading underline ends
// a paragraph too, and a line opening a block, such as a list item, a field or a section header, starts a new one.
pub(crate) fn long_paragraphs<'a>(
    lines: impl IntoIterator<Item = Option<(TextRange, &'a str)>>,
    limit: usize,
) -> Vec<TextRange> {
    let mut long = Vec::new();
    let mut words = 0;
    let mut first = None;
    for line in lines {
        let Some((range, text)) = line.filter(|(_, text)| !is_underline(text)) else {
            words = 0;
            first = None;
            continue;
        };
        if opens_block(text) {
            words = 0;
            first = None;
        }
        let start = *first.get_or_insert(range.start());
        let before = words;
        words += text.split_whitespace().count();
        if before <= limit && words > limit {
            long.push(TextRange::new(start, range.end()));
        }
    }
    long
}

fn is_underline(text: &str) -> bool {
    text.len() > 2 && text.chars().all(|char| matches!(char, '-' | '=' | '~' | '^' | '*' | '+' | '#'))
}

// A bullet or enumerated item, an RST field or directive, a Google section header or entry, or a numpydoc entry.
fn opens_block(text: &str) -> bool {
    let (head, rest) = text.split_once(' ').unwrap_or((text, ""));
    let enumerated = head.strip_suffix(['.', ')']).is_some_and(|number| {
        number == "#" || (!number.is_empty() && number.chars().all(|char| char.is_ascii_digit()))
    });
    let field = text.strip_prefix(':').is_some_and(|field| field.contains(':'));
    let typed = rest.starts_with(':')
        || rest
            .strip_prefix('(')
            .and_then(|typed| typed.split_once(')'))
            .is_some_and(|(_, after)| after.starts_with(':'));
    let entry = head.strip_suffix(':').is_some_and(is_identifier) || (is_identifier(head) && typed);
    let section = text.strip_suffix(':').is_some_and(|title| {
        title.starts_with(char::is_uppercase) && title.chars().all(|char| char.is_alphabetic() || char == ' ')
    });
    matches!(head, "-" | "*" | "+" | "\u{2022}" | "..") || enumerated || field || entry || section
}

fn is_identifier(word: &str) -> bool {
    let name = word.trim_start_matches('*');
    name.starts_with(|char: char| char.is_alphabetic() || char == '_')
        && name.chars().all(|char| char.is_alphanumeric() || matches!(char, '_' | '.'))
}

// Ruff ranks names isort-style: constants in SCREAMING_SNAKE_CASE first, then CamelCase classes, then the rest. The two
// orders differ in the details: RUF022, sorting `__all__`, takes any name with a letter and no lowercase one for a
// constant and compares case-sensitively, while isort, sorting the names of a `from` import, wants more than one
// character for a constant and ignores case.
pub(crate) fn export_key(name: &str) -> (u8, String) {
    let rank = if name.chars().any(char::is_alphabetic) && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_owned())
}

pub(crate) fn isort_key(name: &str) -> (u8, String) {
    let rank = if name.len() > 1 && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_ascii_lowercase())
}

// Ruff's test: a name `TYPE_CHECKING` by its own spelling or through `typing`, or any `.TYPE_CHECKING` attribute.
pub(crate) fn is_type_checking(test: &Expr, checker: &Checker) -> bool {
    is_type_checking_name(test) || (test.is_name_expr() && checker.semantic().match_typing_expr(test, "TYPE_CHECKING"))
}
