use ruff_text_size::{TextRange, TextSize};

use super::blocks::{Shown, marks};
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
    let blocks = marks(lines);
    let mut fields: Vec<ListedField> = Vec::new();
    // The field whose body a deeper line continues, and its indentation.
    let mut open: Option<(usize, usize)> = None;
    let mut text_since_field = false;
    let mut previous_field = None;
    for (index, line) in lines.iter().enumerate() {
        if line.text.is_empty() {
            continue;
        }
        let level = line.indent;
        let trimmed = line.text.trim_start();
        if level == base
            && blocks[index].shown == Shown::Prose
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
                joins_text: index > 0 && blocks[index - 1].paragraph && previous_field != Some(index - 1),
            });
            open = Some((fields.len() - 1, level));
            previous_field = Some(index);
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

// The text between the colons of a field, as `param count` in `:param count: How many.` Like docutils, a colon that
// follows more text stays in the name, an escaped one included, and the closing colon needs a space or the end after
// it and no space before it.
pub(crate) fn field_marker(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(':')?;
    if rest.starts_with(|char: char| char == ':' || char.is_ascii_whitespace()) {
        return None;
    }
    let mut chars = rest.char_indices();
    while let Some((index, char)) = chars.next() {
        match char {
            '\\' => {
                chars.next()?;
            }
            ':' => {
                let after = &rest[index + 1..];
                if after.is_empty() || after.starts_with(|char: char| char.is_ascii_whitespace()) {
                    return (!rest[..index].ends_with(|char: char| char.is_ascii_whitespace()))
                        .then_some(&rest[..index]);
                }
                if after.starts_with('`') {
                    return None;
                }
            }
            _ => {}
        }
    }
    None
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
    #[test_case(":param a\\: b: desc", Some("param a\\: b") ; "escaped_colon_in_name")]
    #[test_case(":Field\\: names\\: with\\: colons\\:: are possible.", Some("Field\\: names\\: with\\: colons\\:") ; "escaped_colons_then_marker")]
    #[test_case(":\\: Not a field either.", None ; "escaped_colon_right_after_the_opening")]
    #[test_case(":param a\\", None ; "backslash_at_the_end")]
    #[test_case(":param x:y: desc", Some("param x:y") ; "colon_followed_by_text_in_name")]
    #[test_case(":field::name:", Some("field::name") ; "colons_in_a_row")]
    #[test_case(":param `x`: desc", Some("param `x`") ; "backticks_in_name")]
    #[test_case(":param a : desc", None ; "space_before_closing_colon")]
    #[test_case(":param a:\tdesc", Some("param a") ; "tab_after_closing_colon")]
    #[test_case(":param a:\u{a0}desc", None ; "non_breaking_space_is_not_a_separator")]
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
    fn field_after_a_bullet_is_not_joined() {
        assert_eq!(
            scanned("- item\n:param a: desc"),
            [(vec!["param", "a"], "param a", ":param a: desc", "desc".to_owned(), false, false)]
        );
    }

    #[test]
    fn field_after_a_literal_block_is_not_joined() {
        assert_eq!(
            scanned("Example::\n\n    code\n:param a: desc"),
            [(vec!["param", "a"], "param a", ":param a: desc", "desc".to_owned(), false, false)]
        );
    }

    #[test]
    fn field_after_a_directive_line_is_not_joined() {
        assert_eq!(
            scanned(".. note:: Text\n:param a: desc"),
            [(vec!["param", "a"], "param a", ":param a: desc", "desc".to_owned(), false, false)]
        );
    }

    #[test]
    fn only_the_first_field_of_a_joined_run_is_flagged() {
        assert_eq!(
            scanned("Text\n:param a: x\n:param b: y"),
            [
                (vec!["param", "a"], "param a", ":param a: x\n", "x".to_owned(), false, true),
                (vec!["param", "b"], "param b", ":param b: y", "y".to_owned(), false, false),
            ]
        );
    }

    #[test]
    fn enumerated_item_without_indented_next_line_is_text_before_a_field() {
        assert_eq!(
            scanned("Summary.\n\n1. Item\n:param a: y"),
            [(vec!["param", "a"], "param a", ":param a: y", "y".to_owned(), false, true)]
        );
    }

    #[test]
    fn quoted_literal_lines_are_not_fields() {
        assert_eq!(scanned("Example::\n\n:param a: desc\n"), []);
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
