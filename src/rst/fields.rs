use ruff_text_size::{TextRange, TextSize};

use super::lines::Line;

// A field of a field list, as `:param int count: How many.` with the lines that continue it.
pub(crate) struct ListedField<'a> {
    // The words between the colons: `param`, `int` and `count` above.
    pub(crate) words: Vec<&'a str>,
    // The text between the colons, where it sits in the text scanned.
    pub(crate) marker: TextRange,
    // The field's line and the lines that continue it, whole with their line breaks.
    pub(crate) span: TextRange,
    pub(crate) description: String,
    // Whether text other than fields separates the field from an earlier one.
    pub(crate) after_text: bool,
    // Whether the field starts a field list right under a line of text, which reStructuredText then reads as part of
    // that paragraph rather than as a field.
    pub(crate) joins_text: bool,
}

// The fields at indentation `base`, each with the deeper lines that follow it before any other text.
pub(crate) fn field_list<'a>(lines: &[Line<'a>], base: usize) -> Vec<ListedField<'a>> {
    let mut fields: Vec<ListedField> = Vec::new();
    // The field whose body a deeper line continues, and its indentation.
    let mut open: Option<(usize, usize)> = None;
    let mut text_since_field = false;
    for (index, line) in lines.iter().enumerate() {
        if line.text.is_empty() {
            continue;
        }
        let level = line.indent;
        let trimmed = line.text.trim_start();
        if level == base
            && let Some(marker) = field_marker(trimmed)
        {
            fields.push(ListedField {
                words: marker.split_whitespace().collect(),
                marker: TextRange::at(
                    line.start + TextSize::try_from(line.text.len() - trimmed.len() + 1).unwrap_or_default(),
                    TextSize::of(marker),
                ),
                span: TextRange::at(line.start, TextSize::of(line.raw)),
                description: trimmed[marker.len() + 2..].trim().to_owned(),
                after_text: text_since_field && !fields.is_empty(),
                joins_text: index > 0 && open.is_none() && !lines[index - 1].text.is_empty(),
            });
            open = Some((fields.len() - 1, level));
            text_since_field = false;
            continue;
        }
        if let Some((at, _)) = open.filter(|(_, field)| level > *field) {
            let field = &mut fields[at];
            field.span = TextRange::new(field.span.start(), line.start + TextSize::of(line.raw));
            if !field.description.is_empty() {
                field.description.push(' ');
            }
            field.description.push_str(trimmed);
            continue;
        }
        open = None;
        text_since_field = true;
    }
    fields
}

// The text between the colons of a field, as `param count` in `:param count: How many.` A role such as `:class:`Name``
// starting a line of prose has a backtick after its colon, not a space.
pub(crate) fn field_marker(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(':')?;
    let end = rest.find(':')?;
    let marker = &rest[..end];
    let after = &rest[end + 1..];
    (!marker.is_empty()
        && !marker.starts_with(char::is_whitespace)
        && !marker.contains('`')
        && (after.is_empty() || after.starts_with(char::is_whitespace)))
    .then_some(marker)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{field_list, field_marker};
    use crate::rst::lines::docstring_lines;

    #[test_case(":param a: text", Some("param a") ; "field")]
    #[test_case(":param a:", Some("param a") ; "field_without_description")]
    #[test_case(":class:`Name`", None ; "role_starting_prose")]
    #[test_case(":param a:text", None ; "no_space_after_colon")]
    #[test_case(": a: text", None ; "space_before_marker")]
    #[test_case("::", None ; "empty_marker")]
    #[test_case("param a: text", None ; "no_leading_colon")]
    fn marker_of_a_field_line(line: &str, expected: Option<&str>) {
        assert_eq!(field_marker(line), expected);
    }

    type Scanned<'a> = (Vec<&'a str>, &'a str, &'a str, String, bool, bool);

    fn scanned(text: &str) -> Vec<Scanned<'_>> {
        let lines = docstring_lines(text);
        let base = lines.first().map_or(0, |line| line.indent);
        field_list(&lines, base)
            .into_iter()
            .map(|field| {
                (
                    field.words,
                    &text[field.marker],
                    &text[field.span],
                    field.description,
                    field.after_text,
                    field.joins_text,
                )
            })
            .collect()
    }

    #[test]
    fn field_spans_its_continuation_lines() {
        assert_eq!(
            scanned(":param a: first\n    more\n:returns: r\n"),
            [
                (vec!["param", "a"], "param a", ":param a: first\n    more\n", "first more".to_owned(), false, false),
                (vec!["returns"], "returns", ":returns: r\n", "r".to_owned(), false, false),
            ]
        );
    }

    #[test]
    fn first_line_marker_is_read_from_its_own_text() {
        assert_eq!(
            scanned(":param a: x\n    b\n"),
            [(vec!["param", "a"], "param a", ":param a: x\n", "x".to_owned(), false, false)]
        );
    }

    #[test]
    fn field_with_empty_description_takes_its_continuation() {
        assert_eq!(
            scanned(":param a:\n        more\n    text\n"),
            [(vec!["param", "a"], "param a", ":param a:\n        more\n", "more".to_owned(), false, false)]
        );
    }

    #[test]
    fn field_after_text_and_joined_to_text() {
        assert_eq!(
            scanned(":param a: x\nText\n:param b: y"),
            [
                (vec!["param", "a"], "param a", ":param a: x\n", "x".to_owned(), false, false),
                (vec!["param", "b"], "param b", ":param b: y", "y".to_owned(), true, true),
            ]
        );
    }

    #[test]
    fn text_at_field_level_ends_the_field() {
        assert_eq!(
            scanned(":param a: x\nText\n    deeper\n"),
            [(vec!["param", "a"], "param a", ":param a: x\n", "x".to_owned(), false, false)]
        );
    }

    #[test]
    fn deeper_marker_continues_the_field_instead() {
        let expected =
            (vec!["param", "a"], "param a", ":param a: x\n    :param b: y\n", "x :param b: y".to_owned(), false, false);
        assert_eq!(scanned("Summary.\n\n:param a: x\n    :param b: y\n"), [expected]);
    }
}
