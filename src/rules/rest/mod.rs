mod rest_field_syntax;
mod rest_missing_param;
mod rest_missing_raises;
mod rest_param_order;
mod rest_redundant_type;
mod rest_returns;
mod rest_unknown_param;

pub(crate) use rest_field_syntax::rest_field_syntax;
pub(crate) use rest_missing_param::rest_missing_param;
pub(crate) use rest_missing_raises::rest_missing_raises;
pub(crate) use rest_param_order::rest_param_order;
pub(crate) use rest_redundant_type::rest_redundant_type;
pub(crate) use rest_returns::rest_returns;
pub(crate) use rest_unknown_param::rest_unknown_param;

use std::collections::HashMap;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rules::docstrings::PublicApi;
use crate::rules::{docstring, last_segment};
use crate::settings::DocstringStyle;

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
}

impl<'a> Docstring<'a> {
    fn parse(checker: &Checker, literal: &'a ast::ExprStringLiteral) -> Self {
        let text = literal.value.to_str();
        // Offsets into the source hold only while the text is the source as written, one part without escapes.
        let start = match literal.value.as_slice() {
            [part] if checker.text(part.content_range()) == text => Some(part.content_range().start()),
            _ => None,
        };
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
        let mut fields: Vec<Field> = Vec::new();
        let mut sections = Vec::new();
        // The indentation of the field whose body a deeper line continues.
        let mut open: Option<usize> = None;
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
                });
                open = Some(level);
                text_since_field = false;
                continue;
            }
            if open.is_some_and(|field| level > field) {
                if let (Some(field), Some(start)) = (fields.last_mut(), start)
                    && let Some(lines) = field.lines
                {
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
        Self { literal, fields, sections }
    }

    pub(crate) fn fields(&self, kind: FieldKind) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(move |field| field.kind == kind)
    }

    pub(crate) fn has(&self, kind: FieldKind) -> bool {
        self.fields(kind).next().is_some()
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

pub(crate) enum Owner<'a> {
    Function { def: &'a ast::StmtFunctionDef, class: Option<&'a ast::StmtClassDef> },
    Class(&'a ast::StmtClassDef),
}

// A docstring the `SMP3` rules check, with what it documents and whether that is part of a library's public API, which
// must document all of its parameters, return value and exceptions; any other docstring must only be accurate.
pub(crate) struct Target<'a> {
    pub(crate) owner: Owner<'a>,
    pub(crate) docstring: Docstring<'a>,
    pub(crate) public: bool,
}

impl<'a> Target<'a> {
    pub(crate) fn name(&self) -> &'a str {
        match self.owner {
            Owner::Function { def, .. } => def.name.as_str(),
            Owner::Class(class) => class.name.as_str(),
        }
    }

    // The parameters the docstring may document: a function's, or the constructor's for a class, `None` when the class
    // takes them from elsewhere, such as a base class.
    pub(crate) fn parameters(&self) -> Option<Vec<Parameter<'a>>> {
        match self.owner {
            Owner::Function { def, class } => Some(parameters(def, class.is_some())),
            Owner::Class(class) => class_parameters(class),
        }
    }

    pub(crate) fn takes_keywords(&self) -> bool {
        match self.owner {
            Owner::Function { def, .. } => def.parameters.kwarg.is_some(),
            Owner::Class(class) => constructor(class).is_some_and(|init| init.parameters.kwarg.is_some()),
        }
    }
}

pub(crate) fn targets<'a>(checker: &Checker<'a>) -> Vec<Target<'a>> {
    if checker.settings.project.docstring_style != Some(DocstringStyle::Rest) {
        return Vec::new();
    }
    let api = PublicApi::of(checker, checker.body());
    let methods: HashMap<TextRange, &ast::StmtClassDef> = checker
        .class_defs()
        .iter()
        .flat_map(|class| class.body.iter().filter_map(Stmt::as_function_def_stmt).map(move |def| (def.range, *class)))
        .collect();
    let functions = checker.function_defs().iter().filter_map(|def| {
        let docstring = Docstring::parse(checker, docstring(&def.body)?);
        let owner = Owner::Function { def, class: methods.get(&def.range).copied() };
        Some(Target { owner, docstring, public: api.is_public(def.range) })
    });
    let classes = checker.class_defs().iter().filter_map(|class| {
        let docstring = Docstring::parse(checker, docstring(&class.body)?);
        Some(Target { owner: Owner::Class(class), docstring, public: api.is_public(class.range) })
    });
    let mut targets: Vec<Target> = functions.chain(classes).collect();
    targets.sort_by_key(|target| target.docstring.range().start());
    targets
}

pub(crate) struct Parameter<'a> {
    pub(crate) name: &'a str,
    pub(crate) annotated: bool,
}

// A function's parameters in order, without the instance or class a method receives first and without names made of
// underscores only, which take a value nobody reads.
fn parameters(def: &ast::StmtFunctionDef, is_method: bool) -> Vec<Parameter<'_>> {
    let receives = is_method
        && !def.decorator_list.iter().any(|decorator| last_segment(&decorator.expression) == Some("staticmethod"));
    let parameters = &def.parameters;
    let positional = parameters.posonlyargs.iter().chain(&parameters.args).map(|parameter| &parameter.parameter);
    let all = positional
        .chain(parameters.vararg.as_deref())
        .chain(parameters.kwonlyargs.iter().map(|parameter| &parameter.parameter))
        .chain(parameters.kwarg.as_deref());
    all.skip(usize::from(receives))
        .filter(|parameter| !parameter.name.as_str().chars().all(|char| char == '_'))
        .map(|parameter| Parameter { name: parameter.name.as_str(), annotated: parameter.annotation.is_some() })
        .collect()
}

fn constructor(class: &ast::StmtClassDef) -> Option<&ast::StmtFunctionDef> {
    class.body.iter().filter_map(Stmt::as_function_def_stmt).find(|def| def.name.as_str() == "__init__")
}

// A class's constructor parameters: its `__init__`'s, or the annotated fields a dataclass or attrs class turns into
// them, `ClassVar`s aside.
fn class_parameters(class: &ast::StmtClassDef) -> Option<Vec<Parameter<'_>>> {
    if let Some(init) = constructor(class) {
        return Some(parameters(init, true));
    }
    let generated = class.decorator_list.iter().any(|decorator| {
        let decorator = decorator.expression.as_call_expr().map_or(&decorator.expression, |call| &call.func);
        matches!(last_segment(decorator), Some("dataclass" | "define" | "frozen" | "mutable" | "attrs" | "s"))
    });
    generated.then(|| {
        class
            .body
            .iter()
            .filter_map(Stmt::as_ann_assign_stmt)
            .filter(|field| last_segment(field.annotation.as_ref()) != Some("ClassVar"))
            .filter_map(|field| field.target.as_name_expr())
            .map(|name| Parameter { name: name.id.as_str(), annotated: true })
            .collect()
    })
}

// The docstring of a class's `__init__`, where some projects document the constructor's parameters instead.
pub(crate) fn constructor_docstring<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Option<Docstring<'a>> {
    Some(Docstring::parse(checker, docstring(&constructor(class)?.body)?))
}

// Deletes whole fields, each with the lines that continue it.
pub(crate) fn field_deletion(fields: &[&Field]) -> Option<Fix> {
    let mut edits = fields.iter().map(|field| field.lines.map(Edit::range_deletion));
    let first = edits.next()??;
    let rest: Option<Vec<Edit>> = edits.collect();
    Some(Fix::unsafe_edits(first, rest?))
}
