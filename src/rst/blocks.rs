use super::lines::{Line, docstring_lines};

pub(crate) struct Classified<'a> {
    pub(crate) line: Line<'a>,
    pub(crate) code: bool,
}

// A line is code when it sits in a literal block indented under a paragraph that ends in `::`, in the content of a
// code directive such as `.. code-block:: python`, or in a doctest block, from a `>>>` line to the next blank line.
pub(crate) fn classify_docstring(text: &str) -> Vec<Classified<'_>> {
    const CODE_DIRECTIVES: &[&str] =
        &["code-block", "code", "sourcecode", "doctest", "testcode", "testoutput", "ipython", "jupyter-execute"];
    let mut lines = Vec::new();
    let mut block: Option<usize> = None;
    let mut doctest = false;
    for line in docstring_lines(text) {
        let content = line.text;
        let indent = line.indent;
        let mut push = |code: bool| lines.push(Classified { line, code });
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

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::classify_docstring;

    #[test_case("Text.\n", &[false] ; "prose")]
    #[test_case("Example::\n\n    x = 1\n\nAfter\n", &[false, true, true, true, false] ; "literal_block")]
    #[test_case(".. code-block:: python\n\n    x = 1\n\nText\n", &[false, true, true, true, false] ; "code_directive")]
    #[test_case(".. note::\n\n    Body\n", &[false, false, false] ; "prose_directive")]
    #[test_case(">>> f()\n1\n\nText\n", &[true, true, false, false] ; "doctest_block")]
    #[test_case(">>> x::\n\n    y\n", &[true, false, false] ; "doctest_line_does_not_open_a_literal_block")]
    fn lines_are_code_or_prose(text: &str, expected: &[bool]) {
        assert_eq!(classify_docstring(text).iter().map(|line| line.code).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn classified_lines_start_at_their_offsets() {
        let starts: Vec<usize> =
            classify_docstring("Text.\n\n  More\n").iter().map(|line| usize::from(line.line.start)).collect();
        assert_eq!(starts, [0, 6, 7]);
    }
}
