use std::collections::BTreeMap;

use ruff_diagnostics::Edit;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Stmt, StringFlags};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::syntax::Nesting;

// Rewrites the edits that change a name read in an f-string field with a `=` debug specifier, so the field prints
// what it printed before. The field prints its source text, `x=`, then the value of the expression, so the rewrite
// keeps that text as written and moves the edits into a plain field: `{x=}` becomes `x={x!r}`, and a conversion or
// format spec carries over. `None` when an edit lands where that cannot hold, such as in a t-string field, which keeps
// its source text in the template, or in a field nested in another one.
pub(crate) fn keep_debug_text(checker: &Checker, edits: Vec<Edit>) -> Option<Vec<Edit>> {
    let finder = checker.state().debug_fields(checker);
    let mut groups: BTreeMap<usize, Vec<Edit>> = BTreeMap::new();
    let mut rewritten = Vec::new();
    for edit in edits {
        if finder.blocked.holders(edit.range()).next().is_some() {
            return None;
        }
        let mut containing = finder.field_ranges.holders(edit.range());
        match (containing.next(), containing.next()) {
            (None, _) => rewritten.push(edit),
            (Some(index), None) => groups.entry(index).or_default().push(edit),
            (Some(_), Some(_)) => return None,
        }
    }
    for (index, edits) in groups {
        let field = &finder.fields[index];
        rewritten.push(Edit::range_replacement(debug_field(checker.source, field, &edits)?, field.range));
    }
    Some(rewritten)
}

fn debug_field(source: &str, field: &DebugField, edits: &[Edit]) -> Option<String> {
    let debug = field.debug.as_str();
    if debug.contains(['\n', '\r', '\\']) {
        return None;
    }
    let quote = field.flags.quote_style().as_char();
    // A raw f-string keeps the backslash that an escaped quote would need, so the quote cannot go in its literal text.
    if field.flags.prefix().is_raw() && debug.contains(quote) {
        return None;
    }
    let start = usize::from(field.range.start()) + 1;
    let equals = start + debug.rfind('=')?;
    let (mut expression_edits, mut remainder_edits): (Vec<&Edit>, Vec<&Edit>) =
        edits.iter().partition(|edit| usize::from(edit.start()) >= start && usize::from(edit.end()) <= equals);
    let expression = splice(source, start, equals, &mut expression_edits)?;
    let expression = expression.trim_end();
    let remainder = splice(source, start + debug.len(), usize::from(field.range.end()) - 1, &mut remainder_edits)?;
    let conversion = if remainder.is_empty() { "!r" } else { &remainder };
    let literal = escape_braces(debug).replace(quote, &format!("\\{quote}"));
    Some(format!("{literal}{{{}{expression}{conversion}}}", if expression.starts_with('{') { " " } else { "" }))
}

// The source between `from` and `to` with the edits inside it applied. `None` when an edit lies outside the span.
fn splice(source: &str, from: usize, to: usize, edits: &mut [&Edit]) -> Option<String> {
    edits.sort_by_key(|edit| usize::from(edit.start()));
    let mut text = String::new();
    let mut cursor = from;
    for edit in edits.iter() {
        text.push_str(source.get(cursor..usize::from(edit.start()))?);
        text.push_str(edit.content().unwrap_or_default());
        cursor = usize::from(edit.end());
    }
    text.push_str(source.get(cursor..to)?);
    Some(text)
}

// The text with each brace doubled, as an f-string spells a literal brace.
pub(crate) fn escape_braces(text: &str) -> String {
    text.replace('{', "{{").replace('}', "}}")
}

#[derive(Default)]
pub(in crate::rules) struct DebugFields<'a> {
    fields: Vec<DebugField<'a>>,
    // Where the fields lie, so that the edits of a field are found by a search.
    field_ranges: Nesting,
    // The debug fields inside a t-string, which the rewrite cannot change.
    blocked_ranges: Vec<TextRange>,
    blocked: Nesting,
}

impl<'a> DebugFields<'a> {
    pub(crate) fn of(body: &'a [Stmt]) -> Self {
        let mut collected = Self::default();
        collected.visit_body(body);
        collected.field_ranges = Nesting::new(collected.fields.iter().map(|field| field.range).collect());
        collected.blocked = Nesting::new(std::mem::take(&mut collected.blocked_ranges));
        collected
    }
}

struct DebugField<'a> {
    range: TextRange,
    debug: &'a ast::DebugText,
    flags: ast::FStringFlags,
}

impl<'a> Visitor<'a> for DebugFields<'a> {
    fn visit_f_string(&mut self, f_string: &'a ast::FString) {
        self.fields.extend(f_string.elements.iter().filter_map(|element| debug_field_of(element, f_string.flags)));
        visitor::walk_f_string(self, f_string);
    }

    fn visit_t_string(&mut self, t_string: &'a ast::TString) {
        self.blocked_ranges.extend(t_string.elements.iter().filter_map(|element| match element {
            ast::InterpolatedStringElement::Interpolation(interpolation) => {
                interpolation.debug_text.as_ref().map(|_| interpolation.range)
            }
            ast::InterpolatedStringElement::Literal(_) => None,
        }));
        visitor::walk_t_string(self, t_string);
    }
}

fn debug_field_of(element: &ast::InterpolatedStringElement, flags: ast::FStringFlags) -> Option<DebugField<'_>> {
    let ast::InterpolatedStringElement::Interpolation(interpolation) = element else {
        return None;
    };
    Some(DebugField { range: interpolation.range, debug: interpolation.debug_text.as_ref()?, flags })
}
