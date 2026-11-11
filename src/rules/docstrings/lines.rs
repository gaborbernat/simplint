use ruff_text_size::TextSize;

pub(crate) fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

// A line of a docstring's text: where it starts, the line as written with its break, the line without trailing
// whitespace, and its indentation.
pub(crate) struct DocLine<'a> {
    pub(crate) start: TextSize,
    pub(crate) raw: &'a str,
    pub(crate) text: &'a str,
    pub(crate) indent: usize,
}

// The lines of a docstring's text with the indentation `inspect.cleandoc` gives them: the first line follows the opening
// quotes, so it takes the indentation of the least indented line after it.
pub(crate) fn doc_lines(text: &str) -> Vec<DocLine<'_>> {
    let mut start = TextSize::default();
    let lines: Vec<DocLine> = text
        .split_inclusive('\n')
        .map(|raw| {
            let line = DocLine { start, raw, text: raw.trim_end(), indent: indent_width(raw.trim_end()) };
            start += TextSize::of(raw);
            line
        })
        .collect();
    let base =
        lines.iter().skip(1).filter(|line| !line.text.is_empty()).map(|line| line.indent).min().unwrap_or_default();
    lines
        .into_iter()
        .enumerate()
        .map(|(number, line)| DocLine { indent: if number == 0 { base } else { line.indent }, ..line })
        .collect()
}
