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

// A doctest can use a module global without importing it.
pub(crate) fn is_named_by_doctest(checker: &Checker, name: &str) -> bool {
    checker.state().doctest_words(checker).contains(name)
}

// Treat any file named `conf.py` as a possible Sphinx configuration.
pub(crate) fn is_sphinx_conf(checker: &Checker) -> bool {
    checker.file_name() == "conf.py"
}

// An unresolved `_Reader` looks like a class by PEP 8 convention. An all-caps name may be a constant.
pub(crate) fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

// Each report spans the start of the paragraph through the first line over the word limit.
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

// Both sort keys put constants before classes and other names. For `__all__`, a single uppercase letter counts as a
// constant and ties are case-sensitive. For imports, a name such as `X` sorts with class names and ties ignore ASCII
// case.
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

// Accept a bare or qualified `TYPE_CHECKING` and imported aliases for the typing constant.
pub(crate) fn is_type_checking(test: &Expr, checker: &Checker) -> bool {
    is_type_checking_name(test) || (test.is_name_expr() && checker.semantic().match_typing_expr(test, "TYPE_CHECKING"))
}
