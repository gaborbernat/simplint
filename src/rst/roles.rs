use super::blocks::Classified;

// A `:role:target` with no backticks: where the role starts, where its target starts and ends, and whether backticks
// around the target make a valid reference.
pub(crate) struct BareRole {
    pub(crate) start: usize,
    pub(crate) target: usize,
    pub(crate) end: usize,
    pub(crate) fixable: bool,
}

// The target runs to the next whitespace, without the punctuation ending a sentence or a closing parenthesis the target
// did not open, so `(:meth:Reader.read).` ends at `read` while `:func:foo(x)` keeps its call.
pub(crate) fn bare_roles(lines: &[Classified]) -> Vec<BareRole> {
    // The roles of Sphinx and its Python domain that take a target in backticks, so a role written without them is
    // still recognised.
    const TARGET_ROLES: &[&str] = &[
        "class", "func", "meth", "attr", "mod", "obj", "exc", "data", "const", "ref", "doc", "term", "any", "code",
        "envvar", "file", "kbd", "abbr", "math",
    ];
    let mut found = Vec::new();
    for classified in lines.iter().filter(|classified| !classified.code) {
        let line = classified.line;
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
            if !TARGET_ROLES.contains(&&rest[domain..domain + close]) {
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
            let target_start = usize::from(line.start) + index + 1 + domain + close + 1;
            found.push(BareRole {
                start: usize::from(line.start) + index,
                target: target_start,
                end: target_start + target.len(),
                // A character a reference cannot hold, as the `=` of `:attr:debug=True`, leaves where the target ends
                // to the author.
                fixable: target.chars().all(|char| char.is_alphanumeric() || "_.~-/+()<>!".contains(char)),
            });
        }
    }
    found
}

// The target of each backticked role of Sphinx's Python domain in a line, without the title of an explicit
// `Title <target>`.
pub(crate) fn references(line: &str) -> Vec<&str> {
    // The roles of the Python domain that name an object, which the documented names must match.
    const OBJECT_ROLES: [&str; 12] =
        ["func", "class", "meth", "attr", "data", "obj", "mod", "exc", "const", "type", "deco", "property"];
    let parts: Vec<&str> = line.split('`').collect();
    (1..parts.len())
        .step_by(2)
        .filter_map(|at| {
            let (before, content) = (parts[at - 1], parts[at]);
            let role = match before.strip_suffix(':') {
                Some(rst) => rst.rsplit(':').next().unwrap_or_default(),
                None => before.strip_suffix('}')?.rsplit_once('{')?.1,
            };
            OBJECT_ROLES.contains(&role.strip_prefix("py:").unwrap_or(role)).then_some(())?;
            Some(content.rsplit_once('<').map_or(content, |(_, inner)| inner.trim_end_matches('>')))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{bare_roles, references};
    use crate::rst::blocks::classify_docstring;

    #[test_case(":func:`~pkg.mod.name`.", &["~pkg.mod.name"] ; "tilde_prefix_is_left_to_the_caller")]
    #[test_case(":class:`Title <pkg.Thing>`", &["pkg.Thing"] ; "explicit_title")]
    #[test_case(":class:`Title <~pkg.Thing>`", &["~pkg.Thing"] ; "explicit_title_keeps_its_tilde")]
    #[test_case("{func}`a.b()`", &["a.b()"] ; "myst_role_with_call")]
    #[test_case(":py:meth:`x`", &["x"] ; "python_domain_prefix")]
    #[test_case(":func:`a` and :class:`b`", &["a", "b"] ; "two_roles")]
    #[test_case(":ref:`label`", &[] ; "role_outside_the_object_roles")]
    fn backticked_targets(line: &str, expected: &[&str]) {
        assert_eq!(references(line), expected);
    }

    #[test_case("See :meth:Reader.read.", &[(4, 10, 21, true)] ; "trailing_full_stop")]
    #[test_case("(:meth:Reader.read).", &[(1, 7, 18, true)] ; "closing_parenthesis_it_did_not_open")]
    #[test_case(":func:foo(x)", &[(0, 6, 12, true)] ; "call_keeps_its_parentheses")]
    #[test_case(":attr:debug=True", &[(0, 6, 16, false)] ; "character_a_reference_cannot_hold")]
    #[test_case(":py:func:foo", &[(0, 9, 12, true)] ; "python_domain_prefix_bare")]
    #[test_case("a:func:foo", &[] ; "role_inside_a_word")]
    #[test_case(":guilabel:foo", &[] ; "role_outside_the_target_roles")]
    #[test_case(":func:-x", &[] ; "target_not_starting_with_a_name")]
    fn bare_roles_and_their_target_offsets(text: &str, expected: &[(usize, usize, usize, bool)]) {
        let found: Vec<_> = bare_roles(&classify_docstring(text))
            .iter()
            .map(|role| (role.start, role.target, role.end, role.fixable))
            .collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn bare_role_in_a_code_block_is_not_reported() {
        assert!(bare_roles(&classify_docstring("Example::\n\n    :func:foo\nText\n")).is_empty());
    }
}
