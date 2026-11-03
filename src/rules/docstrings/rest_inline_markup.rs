use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::targets;

// The roles of Sphinx and its Python domain that take a target in backticks.
const ROLES: &[&str] = &[
    "class", "func", "meth", "attr", "mod", "obj", "exc", "data", "const", "ref", "doc", "term", "any", "code",
    "envvar", "file", "kbd", "abbr", "math",
];

pub fn rest_inline_markup(checker: &Checker) {
    for target in targets(checker) {
        // Positions in the source hold only for a docstring whose text is its source.
        let Some(content) = target.docstring.content else {
            continue;
        };
        let text = &checker.source[content];
        for (start, end) in roles_without_backticks(text) {
            let at = |offset: usize| content.start() + TextSize::try_from(offset).unwrap_or_default();
            let role = &text[start..end];
            let fix = Fix::unsafe_edits(
                Edit::insertion("`".to_owned(), at(role.rfind(':').map_or(end, |colon| start + colon + 1))),
                [Edit::insertion("`".to_owned(), at(end))],
            );
            checker.report_fix(
                Rule::RestInlineMarkup,
                TextRange::new(at(start), at(end)),
                format!("Role `{role}` has no backticks around its target, so it renders as plain text"),
                Some(fix),
                "Put the target in backticks",
            );
        }
        for (start, end) in unclosed_literals(text) {
            let at = |offset: usize| content.start() + TextSize::try_from(offset).unwrap_or_default();
            checker.report(
                Rule::RestInlineMarkup,
                TextRange::new(at(start), at(end)),
                "Inline literal opened with ``` `` ``` is not closed in its paragraph, so the rest renders as code",
            );
        }
    }
}

// Each `:role:target` with no backticks, as the range from the role's first colon to the end of its target: a name such
// as `pkg.Class` or `func()`, without the period ending a sentence.
fn roles_without_backticks(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (index, _) in text.match_indices(':') {
        if index > 0 && !text[..index].ends_with(|char: char| char.is_whitespace() || char == '(') {
            continue;
        }
        let rest = &text[index + 1..];
        // A role of the Python domain may name it, as `:py:func:`.
        let domain = if rest.starts_with("py:") { 3 } else { 0 };
        let Some(close) = rest[domain..].find(':') else {
            continue;
        };
        if !ROLES.contains(&&rest[domain..domain + close]) {
            continue;
        }
        let after = &rest[domain + close + 1..];
        let name_len = after
            .find(|char: char| !(char.is_alphanumeric() || matches!(char, '_' | '.' | '~')))
            .unwrap_or(after.len());
        let name = after[..name_len].trim_end_matches('.');
        let call = if after[name.len()..].starts_with("()") { 2 } else { 0 };
        if name.starts_with(|char: char| char.is_alphabetic() || char == '_' || char == '~') {
            found.push((index, index + 1 + domain + close + 1 + name.len() + call));
        }
    }
    found
}

// The double backticks that open an inline literal no later pair closes within its paragraph, as a range over the
// opening pair. Literal blocks, the lines indented under a line ending in `::`, hold code rather than markup.
fn unclosed_literals(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut opening: Option<usize> = None;
    let mut block: Option<usize> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let content = line.trim_end();
        let indent = content.len() - content.trim_start().len();
        if content.is_empty() {
            found.extend(opening.take().map(|at| (at, at + 2)));
            continue;
        }
        if block.is_some_and(|block| indent > block) {
            continue;
        }
        block = content.ends_with("::").then_some(indent);
        for (index, _) in content.match_indices("``") {
            opening = if opening.is_some() { None } else { Some(start + index) };
        }
    }
    found.extend(opening.map(|at| (at, at + 2)));
    found
}
