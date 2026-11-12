use ruff_python_ast::{self as ast, StringFlags};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rst::{docstring_lines, field_list, field_marker, indent_width};

// What a field documents, by its name: the info fields of Sphinx's Python domain and their aliases, and the `yields`
// and `ytype` fields documentation tools accept for generators.
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

// One field of a field list, as `:param int count: How many.` with the lines that continue it.
pub(crate) struct Field {
    pub(crate) kind: FieldKind,
    // The words between the colons: `param`, `int` and `count` above.
    pub(crate) words: Vec<String>,
    // The name an alias of the field stands for, such as `param` for `:arg:`.
    pub(crate) primary: Option<&'static str>,
    // Where the field and its continuation lines sit in the source, whole lines with their line break, and where the
    // text between its colons sits; `None` when escapes make the docstring's text differ from its source, or the field
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
    // The problem with the field's name or arguments, as Sphinx reads them; rst-empty-field skips a field that has one.
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

    // The name a field documents, its last word without the `*` or `\*` of `*args` and the backticks of interpreted
    // text, which Sphinx renders without them; `None` for a field of one word.
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
                // The first line shares the line of the opening quotes, so its field has no whole line of its own to
                // edit.
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

// The kind of a field by its first word, and the name Sphinx reads it as when the word is an alias. `var`, `ivar` and
// `cvar` have no name of their own: Sphinx renders them alike, yet `rst-attribute-kind` tells instance attributes from
// class variables by them.
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

// A Google section is `Args:` over indented lines; a numpydoc one is `Parameters` over a line of dashes.
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

// The Google and numpydoc sections documenting a function's parameters and results.
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
