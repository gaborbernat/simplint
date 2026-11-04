use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::{Docstring, Field, FieldKind, Parameter};
use crate::lint::checker::Checker;
use crate::rules::indentation;

// Deletes whole fields, each with the lines that continue it. Fields that end the docstring take the blank lines before
// them too, and the closing quotes go after the text left, on a line of their own at the docstring's indentation when
// that text spans several lines, and on its last line otherwise.
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
        edits.push(if kept.contains('\n') {
            let line_ending = checker.stylist.line_ending().as_str();
            Edit::range_replacement(format!("{line_ending}{}", indentation(source, docstring.literal)), range)
        } else {
            Edit::range_deletion(range)
        });
    }
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    Some(Fix::unsafe_edits(first, edits))
}

// Where a field goes in the order Sphinx renders a field list: parameters in the order the signature takes them, then
// attributes, then the return or yield, then the exceptions, then anything else.
pub(crate) fn rank(kind: FieldKind, subject: Option<&str>, parameters: &[Parameter]) -> usize {
    match kind {
        FieldKind::Param | FieldKind::Type => subject
            .and_then(|subject| parameters.iter().position(|parameter| parameter.name == subject))
            .unwrap_or(parameters.len()),
        FieldKind::Var | FieldKind::VarType => 1 << 16,
        FieldKind::Returns | FieldKind::ReturnType | FieldKind::Yields | FieldKind::YieldType => 2 << 16,
        FieldKind::Raises => 3 << 16,
        FieldKind::Meta | FieldKind::Unknown => 4 << 16,
    }
}

// Adds fields with empty descriptions, `added` holding each one's rank and text, such as `:param path:`. Each goes before
// the first field ranked after it, or at the end of the field list; a docstring without fields gets a field list after a
// blank line, with the closing quotes on a line of their own. `None` when the docstring's text differs from its source,
// or a field shares the line of the opening quotes.
pub(crate) fn field_insertion(
    checker: &Checker,
    docstring: &Docstring,
    parameters: &[Parameter],
    added: &[(usize, String)],
) -> Option<Fix> {
    let content = docstring.content?;
    let source = checker.source;
    let indent = docstring.indent.as_deref().unwrap_or_else(|| indentation(source, docstring.literal));
    let newline = checker.stylist.line_ending().as_str();
    let existing: Vec<(usize, TextRange)> = docstring
        .fields
        .iter()
        .map(|field| Some((rank(field.kind, field.subject(), parameters), field.lines?)))
        .collect::<Option<_>>()?;
    let mut before: Vec<(TextSize, String)> = Vec::new();
    let mut at_end = String::new();
    let mut added: Vec<&(usize, String)> = added.iter().collect();
    added.sort_by_key(|(rank, _)| *rank);
    for (rank, text) in added {
        let line = format!("{indent}{text}{newline}");
        // After the last field that goes before it, so a field out of order, as one for a parameter that is gone, does
        // not pull the new one away from its neighbors.
        let next = existing.iter().rposition(|(existing, _)| existing <= rank).map_or(0, |anchor| anchor + 1);
        match existing.get(next) {
            Some((_, lines)) => match before.iter_mut().find(|(at, _)| *at == lines.start()) {
                Some((_, block)) => block.push_str(&line),
                None => before.push((lines.start(), line)),
            },
            None => at_end.push_str(&line),
        }
    }
    let mut edits: Vec<Edit> = before.into_iter().map(|(at, block)| Edit::insertion(block, at)).collect();
    if !at_end.is_empty() {
        let body = &source[content];
        let (at, prefix) = match existing.last() {
            Some((_, lines)) => (lines.end(), ""),
            // A blank line separates the description from the new field list.
            None => match body.rfind('\n').filter(|&index| body[index + 1..].trim().is_empty()) {
                Some(index) => {
                    let blank = body[..index].trim_end_matches([' ', '\t']).ends_with('\n');
                    (content.start() + TextSize::try_from(index + 1).ok()?, if blank { "" } else { newline })
                }
                None => (content.end(), ""),
            },
        };
        // Text before the closing quotes on the same line moves the quotes to a line of their own.
        let edit = if source[..at.to_usize()].ends_with('\n') {
            Edit::insertion(format!("{prefix}{at_end}"), at)
        } else if docstring.commented {
            return None;
        } else {
            let separator = if existing.is_empty() { format!("{newline}{newline}") } else { newline.to_owned() };
            Edit::insertion(format!("{separator}{at_end}{indent}"), at)
        };
        edits.push(edit);
    }
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    Some(Fix::unsafe_edits(first, edits))
}
