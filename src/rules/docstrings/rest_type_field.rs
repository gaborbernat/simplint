use ruff_diagnostics::{Edit, Fix};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, Owner, field_deletion, targets};

pub fn rest_type_field(checker: &Checker) {
    for target in targets(checker) {
        let parameters = target.parameters().unwrap_or_default();
        let annotated = |name: &str| parameters.iter().any(|parameter| parameter.name == name && parameter.annotated);
        let returns_annotated = matches!(target.owner, Owner::Function { def, .. } if def.returns.is_some());
        for field in &target.docstring.fields {
            let range = field.marker.unwrap_or_else(|| target.docstring.range());
            let text = field.text();
            // The fix drops a type only where the annotation already holds it; elsewhere the docstring's text, such as
            // `list of str`, may not be an annotation at all.
            let (message, fix, title) = match (field.kind, field.subject()) {
                (FieldKind::Type, Some(name)) => (
                    format!("`{text}` puts a type in the docstring; annotate `{name}` in the signature instead"),
                    annotated(name).then(|| field_deletion(checker, &target.docstring, &[field])).flatten(),
                    format!("Remove `{text}`"),
                ),
                (FieldKind::VarType, Some(name)) => (
                    format!("`{text}` puts a type in the docstring; annotate `{name}` where it is defined instead"),
                    None,
                    String::new(),
                ),
                (FieldKind::ReturnType | FieldKind::YieldType, _) => (
                    format!("`{text}` puts a type in the docstring; annotate the return type in the signature instead"),
                    returns_annotated.then(|| field_deletion(checker, &target.docstring, &[field])).flatten(),
                    format!("Remove `{text}`"),
                ),
                (FieldKind::Param | FieldKind::Var, Some(name)) if field.words.len() > 2 => {
                    let short = format!("{} {name}", field.words[0]);
                    let fix = (field.kind == FieldKind::Param && annotated(name))
                        .then(|| {
                            field.marker.map(|marker| Fix::unsafe_edit(Edit::range_replacement(short.clone(), marker)))
                        })
                        .flatten();
                    (
                        format!(
                            "`{text}` puts a type in the docstring; write `:{short}:` and annotate `{name}` instead"
                        ),
                        fix,
                        format!("Write `:{short}:`"),
                    )
                }
                _ => continue,
            };
            checker.report_fix(Rule::RestTypeField, range, message, fix, title);
        }
    }
}
