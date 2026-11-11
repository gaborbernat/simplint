use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::StringFlags;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{Docstring, Field};
use crate::lint::checker::Checker;
use crate::rules::edits::fix_of;

// Deletes whole fields, each with the lines that continue it. Fields that end the docstring take the blank lines before
// them too, and the closing quotes go after the text left. They go on a line of their own at the docstring's
// indentation when that text spans several lines or ends in a quote or backslash, which would end or escape the string,
// and on its last line otherwise.
pub(crate) fn field_deletion(checker: &Checker, docstring: &Docstring, fields: &[&Field]) -> Option<Fix> {
    let content = docstring.content?;
    let source = checker.source;
    let mut ranges: Vec<TextRange> = fields.iter().map(|field| field.lines).collect::<Option<_>>()?;
    ranges.sort_by_key(Ranged::start);
    let blank_between = |start: TextSize, end: TextSize| source[TextRange::new(start, end)].trim().is_empty();
    let mut trailing = ranges.len();
    let mut end = content.end();
    while trailing > 0 && blank_between(ranges[trailing - 1].end(), end) {
        trailing -= 1;
        end = ranges[trailing].start();
    }
    let mut edits: Vec<Edit> = ranges[..trailing].iter().map(|range| Edit::range_deletion(*range)).collect();
    if trailing < ranges.len() {
        if docstring.commented {
            return None;
        }
        let before = &source[TextRange::new(content.start(), ranges[trailing].start())];
        let text_end = content.start() + TextSize::of(before.trim_end());
        let kept: String = ranges[..trailing]
            .iter()
            .fold((content.start(), String::new()), |(at, mut kept), range| {
                kept.push_str(&source[TextRange::new(at, range.start())]);
                (range.end(), kept)
            })
            .1
            + &source[TextRange::new(ranges[..trailing].last().map_or(content.start(), Ranged::end), text_end)];
        let range = TextRange::new(text_end, content.end());
        let quote = docstring.literal.value.first_literal_flags().quote_style().as_char();
        edits.push(if kept.contains('\n') || kept.ends_with([quote, '\\']) {
            let line_ending = checker.line_ending();
            Edit::range_replacement(format!("{line_ending}{}", checker.indentation(docstring.literal)), range)
        } else {
            Edit::range_deletion(range)
        });
    }
    fix_of(edits, Applicability::Unsafe)
}

impl Field {
    pub(crate) fn rename(&self, name: &str) -> Option<Fix> {
        let word = TextRange::at(self.marker?.start(), TextSize::of(self.words[0].as_str()));
        Some(Fix::unsafe_edit(Edit::range_replacement(name.to_owned(), word)))
    }

    pub(crate) fn removal(&self, checker: &Checker, docstring: &Docstring) -> Option<Fix> {
        field_deletion(checker, docstring, &[self])
    }
}
