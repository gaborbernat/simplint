use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::StringFlags;
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::Field;
use super::parse::Docstring;
use crate::lint::checker::Checker;
use crate::rules::edits::fix_of;

impl Field {
    pub(crate) fn rename(&self, name: &str) -> Option<Fix> {
        let word = TextRange::at(self.marker?.start(), TextSize::of(self.words[0].as_str()));
        Some(Fix::unsafe_edit(Edit::range_replacement(name.to_owned(), word)))
    }

    pub(crate) fn removal(&self, checker: &Checker, docstring: &Docstring) -> Option<Fix> {
        field_deletion(checker, docstring, &[self])
    }
}

// Fields that end the docstring take the blank lines before them too. The closing quotes move to a line of their own
// when the text left spans lines or ends in a quote or backslash, which would end or escape the string.
pub(crate) fn field_deletion(checker: &Checker, docstring: &Docstring, fields: &[&Field]) -> Option<Fix> {
    let content = docstring.content?;
    let source = checker.source;
    let mut ranges: Vec<TextRange> = fields.iter().map(|field| field.lines).collect::<Option<_>>()?;
    ranges.sort_by_key(Ranged::start);
    let mut trailing = ranges.len();
    let mut end = content.end();
    while trailing > 0 && source[TextRange::new(ranges[trailing - 1].end(), end)].trim().is_empty() {
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
            Edit::range_replacement(
                format!("{}{}", checker.line_ending(), checker.indentation(docstring.literal)),
                range,
            )
        } else {
            Edit::range_deletion(range)
        });
    }
    fix_of(edits, Applicability::Unsafe)
}
