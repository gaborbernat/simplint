use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, targets};

pub fn rest_field_syntax(checker: &Checker) {
    for target in targets(checker) {
        let docstring = &target.docstring;
        for section in &docstring.sections {
            let message = if section.numpy {
                format!(
                    "numpydoc section `{}` in a reStructuredText docstring; write fields such as `:param name:`",
                    section.heading
                )
            } else {
                format!(
                    "Google-style section `{}:` in a reStructuredText docstring; write fields such as `:param name:`",
                    section.heading
                )
            };
            checker.report(Rule::RestFieldSyntax, docstring.range(), message);
        }
        let mut seen: HashSet<(u8, Option<&str>)> = HashSet::new();
        let mut reported_placement = false;
        for field in &docstring.fields {
            let range = field.marker.unwrap_or_else(|| docstring.range());
            let text = field.text();
            let word = &field.words[0];
            if field.after_text && !reported_placement {
                reported_placement = true;
                checker.report(
                    Rule::RestFieldSyntax,
                    range,
                    format!("Field `{text}` follows text after the field list; keep the fields together at the end"),
                );
            }
            let problem = match field.kind {
                FieldKind::Unknown => Some(format!(
                    "Unknown field `{text}`; Sphinx reads `param`, `type`, `raises`, `returns`, `rtype`, `var`, `vartype` \
                     and their aliases"
                )),
                FieldKind::Param | FieldKind::Type | FieldKind::Var | FieldKind::VarType
                    if field.subject().is_none() =>
                {
                    Some(format!("Field `{text}` names nothing; write `:{word} name:`"))
                }
                FieldKind::Raises if field.subject().is_none() => {
                    Some(format!("Field `{text}` names no exception; write `:{word} Error:`"))
                }
                FieldKind::Returns | FieldKind::ReturnType | FieldKind::Yields | FieldKind::YieldType
                    if field.words.len() > 1 =>
                {
                    Some(format!("Field `{text}` takes no name; write `:{word}:`"))
                }
                _ => None,
            };
            if let Some(problem) = problem {
                checker.report(Rule::RestFieldSyntax, range, problem);
                continue;
            }
            // A field that names what it documents but says nothing about it, as the fixes of the other rules leave it.
            let needs_description = matches!(
                field.kind,
                FieldKind::Param | FieldKind::Raises | FieldKind::Returns | FieldKind::Yields | FieldKind::Var
            );
            if needs_description && !field.described {
                checker.report(
                    Rule::RestFieldSyntax,
                    range,
                    format!("Field `{text}` has no description; say what it documents"),
                );
            }
            let repeatable = matches!(field.kind, FieldKind::Raises | FieldKind::Meta | FieldKind::Unknown);
            if !repeatable && !seen.insert((field.kind as u8, field.subject())) {
                checker.report(Rule::RestFieldSyntax, range, format!("Field `{text}` repeats an earlier field"));
            }
        }
    }
}
