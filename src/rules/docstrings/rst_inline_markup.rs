use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::lines::doc_lines;

// The roles of Sphinx and its Python domain that take a target in backticks.
const ROLES: &[&str] = &[
    "class", "func", "meth", "attr", "mod", "obj", "exc", "data", "const", "ref", "doc", "term", "any", "code",
    "envvar", "file", "kbd", "abbr", "math",
];

// The directives whose content is code, so it holds no inline markup.
const CODE_DIRECTIVES: &[&str] =
    &["code-block", "code", "sourcecode", "doctest", "testcode", "testoutput", "ipython", "jupyter-execute"];

pub fn rst_inline_markup(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        // Positions in the source hold only for a docstring whose text is its source.
        let Some(content) = target.docstring.content else {
            continue;
        };
        let text = &checker.source[content];
        let at = |offset: usize| content.start() + TextSize::try_from(offset).unwrap_or_default();
        let lines = lines(text);
        for role in roles_without_backticks(&lines) {
            let written = &text[role.start..role.end];
            let fix = role.fixable.then(|| {
                Fix::unsafe_edits(
                    Edit::insertion("`".to_owned(), at(role.target)),
                    [Edit::insertion("`".to_owned(), at(role.end))],
                )
            });
            checker.report_fix(
                Rule::RstInlineMarkup,
                TextRange::new(at(role.start), at(role.end)),
                format!("Role `{written}` has no backticks around its target, so it renders as plain text"),
                fix,
                "Put the target in backticks",
            );
        }
        for opening in unclosed_literals(&lines) {
            checker.report(
                Rule::RstInlineMarkup,
                TextRange::new(at(opening), at(opening + 2)),
                "Inline literal opened with ``` `` ``` is not closed in its paragraph, so the rest renders as code",
            );
        }
    }
}

// A line of the docstring with where it starts, and whether it is code: the literal block indented under a paragraph
// that ends in `::`, the content of a code directive such as `.. code-block:: python`, or a doctest block, from a `>>>`
// line to the next blank line.
struct Line<'a> {
    start: usize,
    text: &'a str,
    code: bool,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut block: Option<usize> = None;
    let mut doctest = false;
    for line in doc_lines(text) {
        let content = line.text;
        let indent = line.indent;
        let mut push = |code: bool| lines.push(Line { start: usize::from(line.start), text: content, code });
        if content.is_empty() {
            doctest = false;
            push(block.is_some());
            continue;
        }
        if block.is_some_and(|block| indent > block) {
            push(true);
            continue;
        }
        block = None;
        let trimmed = content.trim_start();
        doctest |= trimmed.starts_with(">>>");
        push(doctest);
        // A directive's content is prose unless the directive shows code, as `.. note::` against `.. code-block::`.
        let opens = match trimmed.strip_prefix(".. ").and_then(|directive| directive.split_once("::")) {
            Some((name, _)) => CODE_DIRECTIVES.contains(&name.trim()),
            None => content.ends_with("::") && !doctest,
        };
        if opens {
            block = Some(indent);
        }
    }
    lines
}

// A `:role:target` with no backticks: where the role starts, where its target starts and ends, and whether backticks
// around the target make a valid reference.
struct Role {
    start: usize,
    target: usize,
    end: usize,
    fixable: bool,
}

// The target runs to the next whitespace, without the punctuation ending a sentence or a closing parenthesis the target
// did not open, so `(:meth:Reader.read).` ends at `read` while `:func:foo(x)` keeps its call.
fn roles_without_backticks(lines: &[Line]) -> Vec<Role> {
    let mut found = Vec::new();
    for line in lines.iter().filter(|line| !line.code) {
        let text = line.text;
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
            let mut target = &after[..after.find(char::is_whitespace).unwrap_or(after.len())];
            loop {
                let unopened = target.ends_with(')') && target.matches(')').count() > target.matches('(').count();
                if target.ends_with(['.', ',', ';', ':', '!', '?', '\'', '"']) || unopened {
                    target = &target[..target.len() - 1];
                } else {
                    break;
                }
            }
            if !target.starts_with(|char: char| char.is_alphanumeric() || char == '_' || char == '~') {
                continue;
            }
            // A character a reference cannot hold, as the `=` of `:attr:debug=True`, leaves where the target ends to
            // the author.
            let fixable = target.chars().all(|char| char.is_alphanumeric() || "_.~-/+()<>!".contains(char));
            let target_start = line.start + index + 1 + domain + close + 1;
            found.push(Role {
                start: line.start + index,
                target: target_start,
                end: target_start + target.len(),
                fixable,
            });
        }
    }
    found
}

// Where each inline literal opens that no later pair of backticks closes within its paragraph. A blank line or a code
// line ends a paragraph.
fn unclosed_literals(lines: &[Line]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut opening: Option<usize> = None;
    for line in lines {
        if line.text.is_empty() || line.code {
            found.extend(opening.take());
            continue;
        }
        for (index, _) in line.text.match_indices("``") {
            opening = if opening.is_some() { None } else { Some(line.start + index) };
        }
    }
    found.extend(opening);
    found
}
