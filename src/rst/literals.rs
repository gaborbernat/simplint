use super::blocks::Classified;

// Where each inline literal opens that no later pair of backticks closes within its paragraph. A blank line or a code
// line ends a paragraph.
pub(crate) fn unclosed_literals(lines: &[Classified]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut opening: Option<usize> = None;
    for classified in lines {
        let line = classified.line;
        if line.text.is_empty() || classified.code {
            found.extend(opening.take());
            continue;
        }
        for (index, _) in line.text.match_indices("``") {
            opening = if opening.is_some() { None } else { Some(usize::from(line.start) + index) };
        }
    }
    found.extend(opening);
    found
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::unclosed_literals;
    use crate::rst::blocks::classify_docstring;

    #[test_case("Use ``x`` here.", &[] ; "closed")]
    #[test_case("Use ``x here.", &[4] ; "unclosed")]
    #[test_case("``a\nb`` c", &[] ; "closed_on_the_next_line")]
    #[test_case("``a`` and ``b", &[10] ; "second_unclosed")]
    #[test_case("Use ``x\n\nNext", &[4] ; "blank_line_ends_the_paragraph")]
    #[test_case("Text ``x\n.. code-block:: python\n    y\n", &[5] ; "code_line_ends_the_paragraph")]
    fn literals_left_open(text: &str, expected: &[usize]) {
        assert_eq!(unclosed_literals(&classify_docstring(text)), expected);
    }
}
