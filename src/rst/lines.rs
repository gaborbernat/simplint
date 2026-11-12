use ruff_text_size::TextSize;

// Docutils expands a tab to the next multiple of eight columns, as Python's `expandtabs` does for a docstring.
pub(crate) const TAB_WIDTH: usize = 8;

#[derive(Clone, Copy)]
pub(crate) struct Line<'a> {
    pub(crate) start: TextSize,
    pub(crate) raw: &'a str,
    pub(crate) text: &'a str,
    pub(crate) indent: usize,
}

pub(crate) fn lines(text: &str) -> Vec<Line<'_>> {
    let mut start = TextSize::default();
    text.split_inclusive('\n')
        .map(|raw| {
            let content = raw.strip_suffix("\r\n").or_else(|| raw.strip_suffix('\n')).unwrap_or(raw);
            let line = Line { start, raw, text: content, indent: indent_columns(content, TAB_WIDTH) };
            start += TextSize::of(raw);
            line
        })
        .collect()
}

// The first line follows the opening quotes, so `inspect.cleandoc` gives it the indentation of the least indented line
// after it.
pub(crate) fn docstring_lines(text: &str) -> Vec<Line<'_>> {
    let mut doc: Vec<Line> = lines(text)
        .into_iter()
        .map(|line| {
            let content = line.text.trim_end();
            Line { text: content, indent: indent_columns(content, TAB_WIDTH), ..line }
        })
        .collect();
    let base =
        doc.iter().skip(1).filter(|line| !line.text.is_empty()).map(|line| line.indent).min().unwrap_or_default();
    if let Some(first) = doc.first_mut() {
        first.indent = base;
    }
    doc
}

pub(crate) fn indent_width(line: &str) -> usize {
    indent_columns(line, TAB_WIDTH)
}

// The whitespace a line starts with, as text: `indent_width` counts a tab as up to eight columns, so it cannot slice.
pub(crate) fn leading_whitespace(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

// The indentation of a line in columns, where a tab runs to the next multiple of `tab`.
pub(crate) fn indent_columns(line: &str, tab: usize) -> usize {
    let body = line.trim_start();
    columns(&line[..line.len() - body.len()], tab)
}

pub(crate) fn columns(text: &str, tab: usize) -> usize {
    text.chars().fold(0, |width, char| if char == '\t' { width + tab - width % tab } else { width + 1 })
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{docstring_lines, indent_columns, indent_width, lines};

    #[test_case("a\nb\n", &["a", "b"] ; "newline")]
    #[test_case("a\r\nb\r\n", &["a", "b"] ; "crlf")]
    #[test_case("a\r\n\nb", &["a", "", "b"] ; "empty_line_and_no_final_newline")]
    #[test_case("a\rb\n", &["a\rb"] ; "bare_carriage_return_stays")]
    #[test_case("", &[] ; "empty")]
    fn lines_split_as_str_lines_does(text: &str, expected: &[&str]) {
        assert_eq!(lines(text).iter().map(|line| line.text).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn lines_start_at_their_offsets() {
        let text = "ab\r\ncd\n\nef";
        let starts: Vec<usize> = lines(text).iter().map(|line| usize::from(line.start)).collect();
        assert_eq!(starts, [0, 4, 7, 8]);
    }

    #[test]
    fn docstring_lines_trim_trailing_space_and_take_the_least_indent_for_the_first_line() {
        let indents: Vec<(&str, usize)> = docstring_lines("Summary.\n    Body\n      deeper   \n    \n")
            .iter()
            .map(|line| (line.text, line.indent))
            .collect();
        assert_eq!(indents, [("Summary.", 4), ("    Body", 4), ("      deeper", 6), ("", 0)]);
    }

    #[test]
    fn docstring_first_line_without_others_has_no_indent() {
        let indents: Vec<usize> = docstring_lines("  Summary.").iter().map(|line| line.indent).collect();
        assert_eq!(indents, [0]);
    }

    #[test_case("", 0 ; "empty")]
    #[test_case("text", 0 ; "unindented")]
    #[test_case("    text", 4 ; "indented")]
    #[test_case("\t text", 9 ; "tab_and_space")]
    #[test_case("\ttext", 8 ; "tab_is_eight_columns")]
    #[test_case(" \ttext", 8 ; "tab_stops_after_a_space")]
    fn indent_width_counts_leading_whitespace(line: &str, expected: usize) {
        assert_eq!(indent_width(line), expected);
    }

    #[test_case("", 8, 0 ; "empty")]
    #[test_case("\t text", 8, 9 ; "tab_runs_to_the_next_multiple_of_eight")]
    #[test_case(" \t", 4, 4 ; "tab_after_a_space_runs_to_four")]
    #[test_case("  \tx\t", 8, 8 ; "text_stops_the_indentation")]
    fn indent_columns_expand_tabs(line: &str, tab: usize, expected: usize) {
        assert_eq!(indent_columns(line, tab), expected);
    }
}
