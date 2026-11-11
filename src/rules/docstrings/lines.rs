use ruff_text_size::TextSize;

// The first line follows the opening quotes, so `inspect.cleandoc` gives it the indentation of the least indented line
// after it.
pub(super) fn doc_lines(text: &str) -> Vec<DocLine<'_>> {
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

pub(super) struct DocLine<'a> {
    pub(super) start: TextSize,
    pub(super) raw: &'a str,
    pub(super) text: &'a str,
    pub(super) indent: usize,
}

pub(super) fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
