use ruff_diagnostics::{Edit, Fix};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::rest::{FieldKind, Owner, field_deletion, targets};

pub fn rest_redundant_type(checker: &Checker) {
    for target in targets(checker) {
        let parameters = target.parameters().unwrap_or_default();
        let annotated = |name: &str| parameters.iter().any(|parameter| parameter.name == name && parameter.annotated);
        let returns_annotated = matches!(target.owner, Owner::Function { def, .. } if def.returns.is_some());
        for field in &target.docstring.fields {
            let range = field.marker.unwrap_or_else(|| target.docstring.range());
            let text = field.text();
            match field.kind {
                FieldKind::Type if field.subject().is_some_and(annotated) => checker.report_fix(
                    Rule::RestRedundantType,
                    range,
                    format!(
                        "`{text}` repeats the annotation of `{}`; the signature gives its type",
                        field.subject().unwrap_or_default()
                    ),
                    field_deletion(&[field]),
                    format!("Remove `{text}`"),
                ),
                FieldKind::ReturnType if returns_annotated => checker.report_fix(
                    Rule::RestRedundantType,
                    range,
                    format!("`{text}` repeats the return annotation; the signature gives the type"),
                    field_deletion(&[field]),
                    format!("Remove `{text}`"),
                ),
                FieldKind::Param if field.words.len() > 2 && field.subject().is_some_and(annotated) => {
                    let short = format!("{} {}", field.words[0], field.subject().unwrap_or_default());
                    checker.report_fix(
                        Rule::RestRedundantType,
                        range,
                        format!(
                            "`{text}` repeats the annotation of `{}`; the signature gives its type",
                            field.subject().unwrap_or_default()
                        ),
                        field.marker.map(|marker| Fix::unsafe_edit(Edit::range_replacement(short.clone(), marker))),
                        format!("Write `:{short}:`"),
                    );
                }
                _ => {}
            }
        }
    }
}
