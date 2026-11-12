// A directive of a page with the options under it: `.. name:: argument` in reStructuredText, or `{name} argument`
// behind a fence of backticks or colons in MyST.
pub(crate) struct Directive<'a> {
    pub(crate) name: &'a str,
    pub(crate) argument: &'a str,
    pub(crate) options: Vec<(&'a str, &'a str)>,
}

// The options are the `:key: value` lines right under a directive; any other line ends it.
pub(crate) fn directives<'a>(lines: &[&'a str]) -> Vec<Directive<'a>> {
    let mut found = Vec::new();
    let mut open: Option<Directive> = None;
    for &line in lines {
        let trimmed = line.trim();
        if let Some((name, argument)) = directive(trimmed) {
            found.extend(open.replace(Directive { name, argument, options: Vec::new() }));
        } else if let (Some(directive), Some(option)) = (open.as_mut(), option(trimmed)) {
            directive.options.push(option);
        } else {
            found.extend(open.take());
        }
    }
    found.extend(open);
    found
}

fn directive(line: &str) -> Option<(&str, &str)> {
    if let Some(rest) = line.strip_prefix(".. ") {
        let (name, argument) = rest.split_once("::")?;
        return (!name.contains(' ')).then(|| (name, argument.trim()));
    }
    let rest = line.trim_start_matches(['`', ':']).strip_prefix('{').filter(|_| line.starts_with(['`', ':']))?;
    let (name, argument) = rest.split_once('}')?;
    Some((name, argument.trim()))
}

fn option(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.strip_prefix(':')?.split_once(':')?;
    (!key.is_empty() && !key.contains(' ')).then(|| (key, value.trim()))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::directives;

    type Parsed<'a> = (&'a str, &'a str, Vec<(&'a str, &'a str)>);

    fn found(text: &str) -> Vec<Parsed<'_>> {
        let lines: Vec<&str> = text.lines().collect();
        directives(&lines)
            .into_iter()
            .map(|directive| (directive.name, directive.argument, directive.options))
            .collect()
    }

    #[test_case(".. automodule:: pkg.mod\n", &[("automodule", "pkg.mod", vec![])] ; "rst")]
    #[test_case("```{automodule} pkg.mod\n", &[("automodule", "pkg.mod", vec![])] ; "myst_backtick_fence")]
    #[test_case(":::{automodule} pkg.mod\n", &[("automodule", "pkg.mod", vec![])] ; "myst_colon_fence")]
    #[test_case(".. py:function:: f(a)\n", &[("py:function", "f(a)", vec![])] ; "py_domain_name")]
    #[test_case(".. not a directive:: x\n", &[] ; "name_with_a_space")]
    #[test_case(".. _label:\n", &[] ; "target_without_double_colon")]
    #[test_case(".. comment\n", &[] ; "comment_without_double_colon")]
    #[test_case("{automodule} pkg.mod\n", &[] ; "myst_without_fence")]
    #[test_case("```{automodule pkg.mod\n", &[] ; "myst_without_closing_brace")]
    #[test_case("", &[] ; "empty")]
    fn directive_forms(text: &str, expected: &[Parsed<'_>]) {
        assert_eq!(found(text), expected);
    }

    #[test]
    fn options_under_a_directive() {
        let expected = [("automodule", "pkg", vec![("members", ""), ("exclude-members", "a, b")])];
        assert_eq!(found(".. automodule:: pkg\n   :members:\n   :exclude-members: a, b\n"), expected);
    }

    #[test]
    fn options_under_a_myst_directive() {
        let expected = [("automodule", "pkg", vec![("members", "")])];
        assert_eq!(found("```{automodule} pkg\n:members:\n"), expected);
    }

    #[test]
    fn option_line_without_a_second_colon_is_text() {
        let expected = [("automodule", "pkg", vec![])];
        assert_eq!(found(".. automodule:: pkg\n:members\n"), expected);
    }

    #[test]
    fn text_ends_the_options() {
        let expected = [("automodule", "pkg", vec![("members", "")])];
        assert_eq!(found(".. automodule:: pkg\n:members:\nText\n:private-members:\n"), expected);
    }

    #[test]
    fn option_key_with_a_space_is_text() {
        let expected = [("automodule", "pkg", vec![])];
        assert_eq!(found(".. automodule:: pkg\n:two words: x\n"), expected);
    }

    #[test]
    fn directive_after_directive_is_recorded_each() {
        let expected = [("function", "a", vec![]), ("class", "b", vec![])];
        assert_eq!(found(".. function:: a\n.. class:: b\n"), expected);
    }
}
