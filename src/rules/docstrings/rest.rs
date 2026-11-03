use std::collections::HashMap;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt, StringFlags};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rules::docstrings::{PublicApi, is_documented_variable};
use crate::rules::{docstring, for_each_body, indentation, last_segment, module_statements};
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
    // Whether the field says anything after its name, on its line or on the lines that continue it.
    pub(crate) described: bool,
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
    content: Option<TextRange>,
    // Whether a comment follows the closing quotes, such as a `# noqa` for the docstring's line, which a fix moving the
    // quotes to another line would strand.
    commented: bool,
    // The indentation of the docstring's lines after the first, which a field must keep to belong to its field list;
    // `None` for a docstring of one line.
    indent: Option<String>,
}

impl<'a> Docstring<'a> {
    fn parse(checker: &Checker, literal: &'a ast::ExprStringLiteral) -> Self {
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
                let described = !trimmed[marker.len() + 2..].trim().is_empty();
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
                    joins_text,
                });
                open = Some((fields.len() - 1, level));
                text_since_field = false;
                continue;
            }
            if let Some((at, _)) = open.filter(|(_, field)| level > *field) {
                let field = &mut fields[at];
                field.described = true;
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

// A docstring the `SMP31` rules check, with what it documents and whether that is part of a library's public API, which
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
        .flat_map(|class| {
            module_statements(&class.body).filter_map(Stmt::as_function_def_stmt).map(move |def| (def.range, *class))
        })
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

// The `__init__` that runs: the last one, since `@overload` stubs come before the implementation.
fn constructor(class: &ast::StmtClassDef) -> Option<&ast::StmtFunctionDef> {
    module_statements(&class.body)
        .filter_map(Stmt::as_function_def_stmt)
        .filter(|def| {
            def.name.as_str() == "__init__"
                && !def.decorator_list.iter().any(|decorator| last_segment(&decorator.expression) == Some("overload"))
        })
        .last()
}

// A class's constructor parameters: its `__init__`'s, or the fields a dataclass or attrs class turns into them, those
// annotated in its body and those an `attr.ib()` or `field()` call declares, without `ClassVar`s, the `KW_ONLY`
// marker and fields that set `init=False`. `None` when they come from elsewhere: a base class, or a class that is not
// generated.
fn class_parameters(class: &ast::StmtClassDef) -> Option<Vec<Parameter<'_>>> {
    if let Some(init) = constructor(class) {
        return Some(parameters(init, true));
    }
    // A generator called with `init=False` writes no constructor, so the class keeps `object`'s.
    let generated = class.decorator_list.iter().any(|decorator| {
        let call = decorator.expression.as_call_expr();
        let name = last_segment(call.map_or(&decorator.expression, |call| &call.func));
        let writes_init = !call.is_some_and(|call| {
            call.arguments
                .find_keyword("init")
                .is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| !literal.value))
        });
        writes_init && matches!(name, Some("dataclass" | "define" | "frozen" | "mutable" | "attrs" | "s"))
    }) || class.bases().iter().any(|base| last_segment(base) == Some("NamedTuple"));
    if !generated || has_attribute_bases(class) {
        return None;
    }
    let skips_init = |value: Option<&Expr>| {
        value.and_then(Expr::as_call_expr).is_some_and(|call| {
            call.arguments
                .find_keyword("init")
                .is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| !literal.value))
        })
    };
    let fields = module_statements(&class.body).filter_map(|stmt| match stmt {
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) => {
            let annotation = annotation.as_subscript_expr().map_or(annotation.as_ref(), |subscript| &subscript.value);
            let marker = matches!(last_segment(annotation), Some("ClassVar" | "KW_ONLY"));
            target.as_name_expr().filter(|_| !marker && !skips_init(value.as_deref())).map(|name| (name, true))
        }
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match targets.as_slice() {
            [Expr::Name(name)]
                if value
                    .as_call_expr()
                    .is_some_and(|call| matches!(last_segment(&call.func), Some("ib" | "attrib" | "field")))
                    && !skips_init(Some(value)) =>
            {
                Some((name, false))
            }
            _ => None,
        },
        _ => None,
    });
    Some(
        fields
            .filter(|(name, _)| !name.id.as_str().chars().all(|char| char == '_'))
            .map(|(name, annotated)| Parameter { name: name.id.as_str(), annotated })
            .collect(),
    )
}

// The docstring of a class's `__init__`, where some projects document the constructor's parameters instead.
pub(crate) fn constructor_docstring<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Option<Docstring<'a>> {
    Some(Docstring::parse(checker, docstring(&constructor(class)?.body)?))
}

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

// An attribute of a class as autodoc documents it: one its body assigns, or one a method sets on `self`.
pub(crate) struct Attribute<'a> {
    pub(crate) name: &'a str,
    // Whether it belongs to the class rather than its instances: a `ClassVar`, or a name the body assigns without an
    // annotation; and whether instances hold it, as an annotated field or a name a method sets on `self`, which a class
    // default may be too.
    pub(crate) class_var: bool,
    pub(crate) on_instance: bool,
    // Whether a `#:` comment or a string right after it documents it, as autodoc reads.
    pub(crate) documented: bool,
}

pub(crate) fn attributes<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Vec<Attribute<'a>> {
    let mut found: Vec<Attribute> = Vec::new();
    let mut add = |name: &'a str, class_var: bool, documented: bool| match found
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        Some(attribute) => {
            attribute.documented |= documented;
            attribute.on_instance |= !class_var;
        }
        None => found.push(Attribute { name, class_var, on_instance: !class_var, documented }),
    };
    for stmt in module_statements(&class.body) {
        let documented = is_documented_variable(checker, &class.body, stmt);
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) => {
                let annotation =
                    annotation.as_subscript_expr().map_or(annotation.as_ref(), |subscript| &subscript.value);
                if let Some(name) = target.as_name_expr() {
                    add(name.id.as_str(), last_segment(annotation) == Some("ClassVar"), documented);
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                // An attrs or dataclass field declared by a call holds a value per instance.
                let field = value
                    .as_call_expr()
                    .is_some_and(|call| matches!(last_segment(&call.func), Some("ib" | "attrib" | "field")));
                for name in targets.iter().filter_map(Expr::as_name_expr) {
                    add(name.id.as_str(), !field, documented);
                }
            }
            _ => {}
        }
    }
    for method in module_statements(&class.body).filter_map(Stmt::as_function_def_stmt) {
        // A static method receives nothing; a class method receives the class, whose attributes are class variables.
        let decorators: Vec<&str> =
            method.decorator_list.iter().filter_map(|decorator| last_segment(&decorator.expression)).collect();
        if decorators.contains(&"staticmethod") {
            continue;
        }
        let on_class = decorators.contains(&"classmethod");
        let parameters = &method.parameters;
        let Some(receiver) =
            parameters.posonlyargs.iter().chain(&parameters.args).next().map(|parameter| parameter.name().as_str())
        else {
            continue;
        };
        for_each_body(&method.body, &mut |body, _| {
            for stmt in body {
                // A frozen dataclass sets its own attributes with `object.__setattr__(self, "name", value)`.
                if let Some(name) = set_through_object(stmt, receiver) {
                    add(name, on_class, is_documented_variable(checker, body, stmt));
                }
                let mut targets: Vec<&Expr> = match stmt {
                    Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().collect(),
                    Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })
                    | Stmt::AugAssign(ast::StmtAugAssign { target, .. })
                    | Stmt::For(ast::StmtFor { target, .. }) => vec![target.as_ref()],
                    Stmt::With(ast::StmtWith { items, .. }) => {
                        items.iter().filter_map(|item| item.optional_vars.as_deref()).collect()
                    }
                    _ => Vec::new(),
                };
                // `self.left, self.right = pair` sets both, in the order written.
                let mut next = 0;
                while let Some(&target) = targets.get(next) {
                    next += 1;
                    match target {
                        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                            targets.extend(elts);
                        }
                        Expr::Starred(ast::ExprStarred { value, .. }) => targets.push(value),
                        Expr::Attribute(attribute)
                            if attribute.value.as_name_expr().is_some_and(|name| name.id.as_str() == receiver) =>
                        {
                            add(attribute.attr.as_str(), on_class, is_documented_variable(checker, body, stmt));
                        }
                        _ => {}
                    }
                }
            }
        });
    }
    found
}

// The attribute an `object.__setattr__(receiver, "name", value)` call sets.
fn set_through_object<'a>(stmt: &'a Stmt, receiver: &str) -> Option<&'a str> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr()?;
    let is_object = attribute.value.as_name_expr().is_some_and(|name| name.id.as_str() == "object");
    match call.arguments.args.as_ref() {
        [Expr::Name(target), Expr::StringLiteral(name), ..]
            if is_object && attribute.attr.as_str() == "__setattr__" && target.id.as_str() == receiver =>
        {
            Some(name.value.to_str())
        }
        _ => None,
    }
}

// Whether the class takes attributes from a base that may define them, as a field list cannot tell.
pub(crate) fn has_attribute_bases(class: &ast::StmtClassDef) -> bool {
    class.bases().iter().any(|base| {
        let base = base.as_subscript_expr().map_or(base, |subscript| &subscript.value);
        !matches!(last_segment(base), Some("object" | "ABC" | "Protocol" | "Generic" | "NamedTuple" | "TypedDict"))
    })
}
