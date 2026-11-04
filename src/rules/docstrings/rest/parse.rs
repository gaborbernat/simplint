use ruff_python_ast::{self as ast, StringFlags};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;

// What a field documents, by its name: the info fields of Sphinx's Python domain and their aliases, and the `yields` and
// `ytype` fields documentation tools accept for generators.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Param,
    Type,
    Raises,
    Var,
    VarType,
    Returns,
    ReturnType,
    Yields,
    YieldType,
    Meta,
    Unknown,
}

impl FieldKind {
    fn of(name: &str) -> Self {
        match name {
            "param" | "parameter" | "arg" | "argument" | "key" | "keyword" => Self::Param,
            "type" => Self::Type,
            "raises" | "raise" | "except" | "exception" => Self::Raises,
            "var" | "ivar" | "cvar" => Self::Var,
            "vartype" => Self::VarType,
            "returns" | "return" => Self::Returns,
            "rtype" => Self::ReturnType,
            "yields" | "yield" => Self::Yields,
            "ytype" => Self::YieldType,
            "meta" => Self::Meta,
            _ => Self::Unknown,
        }
    }
}

// One field of a field list, as `:param int count: How many.` with the lines that continue it.
pub(crate) struct Field {
    pub(crate) kind: FieldKind,
    // The words between the colons: `param`, `int` and `count` above.
    pub(crate) words: Vec<String>,
    // Where the field and its continuation lines sit in the source, whole lines with their line break, and where the text
    // between its colons sits; `None` when escapes make the docstring's text differ from its source, or the field
    // shares the line of the opening quotes.
    pub(crate) lines: Option<TextRange>,
    pub(crate) marker: Option<TextRange>,
    // Whether text other than fields separates the field from an earlier one.
    pub(crate) after_text: bool,
    // Whether the field says anything after its name, on its line or on the lines that continue it, and what.
    pub(crate) described: bool,
    pub(crate) description: String,
    // Whether the field starts a field list right under a line of text, which reStructuredText then reads as part of
    // that paragraph rather than as a field.
    pub(crate) joins_text: bool,
}

impl Field {
    // The name a field documents, its last word without the `*` or `\*` of `*args`; `None` for a field of one word.
    pub(crate) fn subject(&self) -> Option<&str> {
        match self.words.as_slice() {
            [_, .., last] => Some(last.trim_start_matches(['\\', '*'])),
            _ => None,
        }
    }

    pub(crate) fn text(&self) -> String {
        format!(":{}:", self.words.join(" "))
    }
}

// A section heading in the Google or numpydoc layout, such as `Args:` or `Parameters` over a line of dashes.
pub(crate) struct Section {
    pub(crate) heading: String,
    pub(crate) numpy: bool,
}

pub(crate) struct Docstring<'a> {
    pub(crate) literal: &'a ast::ExprStringLiteral,
    pub(crate) fields: Vec<Field>,
    pub(crate) sections: Vec<Section>,
    // Where the text between the quotes sits in the source; `None` when escapes make the text differ from its source.
    pub(crate) content: Option<TextRange>,
    // Whether a comment follows the closing quotes, such as a `# noqa` for the docstring's line, which a fix moving the
    // quotes to another line would strand.
    pub(super) commented: bool,
    // The indentation of the docstring's lines after the first, which a field must keep to belong to its field list;
    // `None` for a docstring of one line.
    pub(super) indent: Option<String>,
}

impl<'a> Docstring<'a> {
    pub(super) fn parse(checker: &Checker, literal: &'a ast::ExprStringLiteral) -> Self {
        let text = literal.value.to_str();
        // Offsets into the source hold only while the text is the source as written, one part without escapes.
        let content = match literal.value.as_slice() {
            // A single-quoted string cannot take the line breaks a fix writes.
            [part] if part.flags.is_triple_quoted() && checker.text(part.content_range()) == text => {
                Some(part.content_range())
            }
            _ => None,
        };
        let start = content.map(TextRange::start);
        let lines: Vec<(TextSize, &str)> = text
            .split_inclusive('\n')
            .scan(TextSize::default(), |offset, line| {
                let at = *offset;
                *offset += TextSize::of(line);
                Some((at, line))
            })
            .collect();
        let indent = |line: &str| line.len() - line.trim_start().len();
        let base = lines
            .iter()
            .skip(1)
            .map(|(_, line)| line.trim_end())
            .filter(|line| !line.is_empty())
            .map(indent)
            .min()
            .unwrap_or_default();
        let body_indent = lines
            .iter()
            .skip(1)
            .map(|(_, line)| line.trim_end())
            .find(|line| !line.is_empty() && indent(line) == base)
            .map(|line| line[..base].to_owned());
        let mut fields: Vec<Field> = Vec::new();
        let mut sections = Vec::new();
        // The position and indentation of the field whose body a deeper line continues.
        let mut open: Option<(usize, usize)> = None;
        let mut text_since_field = false;
        for (index, (offset, line)) in lines.iter().enumerate() {
            let content = line.trim_end();
            if content.is_empty() {
                continue;
            }
            let level = if index == 0 { base } else { indent(content) };
            let trimmed = content.trim_start();
            if level == base
                && let Some(marker) = field_marker(trimmed)
            {
                let words: Vec<String> = marker.split_whitespace().map(str::to_owned).collect();
                let in_source = start.filter(|_| index > 0);
                let description = trimmed[marker.len() + 2..].trim().to_owned();
                let described = !description.is_empty();
                let joins_text = index > 0 && open.is_none() && !lines[index - 1].1.trim().is_empty();
                fields.push(Field {
                    kind: FieldKind::of(&words[0]),
                    words,
                    lines: in_source.map(|start| TextRange::at(start + offset, TextSize::of(*line))),
                    marker: in_source.map(|start| {
                        TextRange::at(
                            start + offset + TextSize::try_from(level + 1).unwrap_or_default(),
                            TextSize::of(marker),
                        )
                    }),
                    after_text: text_since_field && !fields.is_empty(),
                    described,
                    description,
                    joins_text,
                });
                open = Some((fields.len() - 1, level));
                text_since_field = false;
                continue;
            }
            if let Some((at, _)) = open.filter(|(_, field)| level > *field) {
                let field = &mut fields[at];
                field.described = true;
                field.description.push(' ');
                field.description.push_str(trimmed);
                if let (Some(start), Some(lines)) = (start, field.lines) {
                    field.lines = Some(TextRange::new(lines.start(), start + offset + TextSize::of(*line)));
                }
                continue;
            }
            open = None;
            text_since_field = true;
            if level == base
                && let Some(section) = section(trimmed, lines.get(index + 1).map(|(_, next)| next.trim_end()), level)
            {
                sections.push(section);
            }
        }
        let commented = checker.trailing_comment(literal).is_some();
        Self { literal, fields, sections, content, commented, indent: body_indent }
    }

    pub(crate) fn fields(&self, kinds: &[FieldKind]) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(move |field| kinds.contains(&field.kind))
    }

    pub(crate) fn has(&self, kind: FieldKind) -> bool {
        self.fields(&[kind]).next().is_some()
    }

    pub(crate) fn range(&self) -> TextRange {
        self.literal.range()
    }
}

// The text between the colons of a field, as `param count` in `:param count: How many.`. A role such as `:class:`Name``
// starting a line of prose has a backtick after its colon, not a space.
fn field_marker(line: &str) -> Option<&str> {
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

const SECTIONS: &[&str] = &[
    "Args",
    "Arguments",
    "Parameters",
    "Params",
    "Keyword Args",
    "Keyword Arguments",
    "Other Parameters",
    "Returns",
    "Return",
    "Yields",
    "Yield",
    "Raises",
    "Attributes",
];

// `Args:` over indented lines, as Google style writes a section, or `Parameters` over a line of dashes, as numpydoc does.
fn section(line: &str, next: Option<&str>, level: usize) -> Option<Section> {
    let next = next.filter(|next| !next.is_empty())?;
    let next_level = next.len() - next.trim_start().len();
    if let Some(heading) = line.strip_suffix(':').filter(|heading| SECTIONS.contains(heading))
        && next_level > level
    {
        return Some(Section { heading: heading.to_owned(), numpy: false });
    }
    let underline = next.trim();
    (SECTIONS.contains(&line) && underline.len() >= 3 && underline.chars().all(|char| char == '-'))
        .then(|| Section { heading: line.to_owned(), numpy: true })
}
