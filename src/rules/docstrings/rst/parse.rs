use ruff_python_ast::{self as ast, StringFlags};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rst::{docstring_lines, field_list, field_marker, indent_width};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

pub(crate) struct Field {
    pub(crate) kind: FieldKind,
    // For `:param int count:` these are `param`, `int` and `count`.
    pub(crate) words: Vec<String>,
    pub(crate) primary: Option<&'static str>,
    // A field on the opening line has no full line to edit. Both ranges also require a source-mapped docstring.
    pub(crate) lines: Option<TextRange>,
    pub(crate) marker: Option<TextRange>,
    pub(crate) after_text: bool,
    // Indented continuation lines can supply the entire description.
    pub(crate) described: bool,
    pub(crate) description: String,
    // Without a blank line after a paragraph, docutils treats the first field as prose.
    pub(crate) joins_text: bool,
}

impl Field {
    // Skip the empty-description check until the field has valid syntax.
    pub(crate) fn problem(&self) -> Option<String> {
        let text = self.text();
        let word = &self.words[0];
        match self.kind {
            FieldKind::Unknown => Some(format!(
                "Unknown field `{text}`; Sphinx reads `param`, `type`, `raises`, `returns`, `rtype`, `var`, `vartype` \
                 and their aliases"
            )),
            FieldKind::Param | FieldKind::Type | FieldKind::Var | FieldKind::VarType if self.subject().is_none() => {
                Some(format!("Field `{text}` names nothing; write `:{word} name:`"))
            }
            FieldKind::Raises if self.subject().is_none() => {
                Some(format!("Field `{text}` names no exception; write `:{word} Error:`"))
            }
            FieldKind::Returns | FieldKind::ReturnType | FieldKind::Yields | FieldKind::YieldType
                if self.words.len() > 1 =>
            {
                Some(format!("Field `{text}` takes no name; write `:{word}:`"))
            }
            _ => None,
        }
    }

    pub(crate) fn text(&self) -> String {
        format!(":{}:", self.words.join(" "))
    }

    // Normalize variadic argument stars and reST backticks before comparing names with the signature.
    pub(crate) fn subject(&self) -> Option<&str> {
        match self.words.as_slice() {
            [_, .., last] => Some(last.trim_start_matches(['\\', '*']).trim_matches('`')),
            _ => None,
        }
    }

    pub(crate) fn range_in(&self, docstring: &Docstring) -> TextRange {
        self.marker.unwrap_or_else(|| docstring.range())
    }
}

pub(crate) struct Section {
    pub(crate) heading: String,
    pub(crate) numpy: bool,
}

pub(crate) struct Docstring<'a> {
    pub(crate) literal: &'a ast::ExprStringLiteral,
    pub(crate) fields: Vec<Field>,
    pub(crate) sections: Vec<Section>,
    // Available when the decoded text maps directly to one triple-quoted literal.
    pub(crate) content: Option<TextRange>,
    // Moving the closing quotes can strand a trailing directive on a different line.
    pub(super) commented: bool,
}

impl<'a> Docstring<'a> {
    pub(super) fn parse(checker: &Checker, literal: &'a ast::ExprStringLiteral) -> Self {
        let text = literal.value.to_str();
        let content = match literal.value.as_slice() {
            // Line-based edits require triple quotes and offsets that match the decoded text.
            [part] if part.flags.is_triple_quoted() && checker.text(part.content_range()) == text => {
                Some(part.content_range())
            }
            _ => None,
        };
        let start = content.map(TextRange::start);
        let lines = docstring_lines(text);
        let base = lines.first().map_or(0, |line| line.indent);
        let scanned = field_list(&lines, base);
        let sections = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.indent == base && !line.text.is_empty())
            .filter(|(_, line)| field_marker(line.text.trim_start()).is_none())
            .filter_map(|(index, line)| {
                section(line.text.trim_start(), lines.get(index + 1).map(|next| next.text), base)
            })
            .collect();
        let fields = scanned
            .into_iter()
            .map(|listed| {
                // A field on the opening line shares that line with the quotes.
                let in_source = start.filter(|_| listed.span.start() > TextSize::default());
                let (kind, primary) = field_name(listed.words[0]);
                Field {
                    kind,
                    words: listed.words.into_iter().map(str::to_owned).collect(),
                    primary,
                    lines: in_source.map(|start| listed.span + start),
                    marker: in_source.map(|start| listed.marker + start),
                    after_text: listed.after_text,
                    described: !listed.description.is_empty(),
                    description: listed.description,
                    joins_text: listed.joins_text,
                }
            })
            .collect();
        Self { literal, fields, sections, content, commented: checker.trailing_comment(literal).is_some() }
    }

    pub(crate) fn subjects(&self, kinds: &[FieldKind]) -> impl Iterator<Item = &str> {
        self.fields(kinds).filter_map(Field::subject)
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

// Keep the original variable field spelling for rst-attribute-kind, even though Sphinx groups the fields together.
fn field_name(word: &str) -> (FieldKind, Option<&'static str>) {
    match word {
        "param" => (FieldKind::Param, None),
        "parameter" | "arg" | "argument" | "key" | "keyword" | "kwarg" | "kwparam" => (FieldKind::Param, Some("param")),
        "type" => (FieldKind::Type, None),
        "paramtype" => (FieldKind::Type, Some("type")),
        "raises" => (FieldKind::Raises, None),
        "raise" | "except" | "exception" => (FieldKind::Raises, Some("raises")),
        "var" | "ivar" | "cvar" => (FieldKind::Var, None),
        "vartype" => (FieldKind::VarType, None),
        "returns" => (FieldKind::Returns, None),
        "return" => (FieldKind::Returns, Some("returns")),
        "rtype" => (FieldKind::ReturnType, None),
        "yields" => (FieldKind::Yields, None),
        "yield" => (FieldKind::Yields, Some("yields")),
        "ytype" => (FieldKind::YieldType, None),
        "meta" => (FieldKind::Meta, None),
        _ => (FieldKind::Unknown, None),
    }
}

fn section(line: &str, next: Option<&str>, level: usize) -> Option<Section> {
    let next = next.filter(|next| !next.is_empty())?;
    let next_level = indent_width(next);
    if let Some(heading) = line.strip_suffix(':').filter(|heading| is_section(heading))
        && next_level > level
    {
        return Some(Section { heading: heading.to_owned(), numpy: false });
    }
    let underline = next.trim();
    (is_section(line) && underline.len() >= 3 && underline.chars().all(|char| char == '-'))
        .then(|| Section { heading: line.to_owned(), numpy: true })
}

fn is_section(heading: &str) -> bool {
    heading == "Attributes" || SIGNATURE_SECTIONS.contains(&heading)
}

pub(crate) const SIGNATURE_SECTIONS: &[&str] = &[
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
];
