use ruff_diagnostics::{Edit, Fix};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner, field_deletion};

pub fn rst_type_field(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let parameters = target.parameters().unwrap_or_default();
        let annotated: FxHashSet<&str> =
            parameters.iter().filter(|parameter| parameter.annotated).map(|parameter| parameter.name).collect();
        let returns_annotated = matches!(target.owner, Owner::Function { def, .. } if def.returns.is_some());
        for field in &target.docstring.fields {
            let range = field.range_in(&target.docstring);
            let text = field.text();
            // The fix drops a type only where the annotation already holds it; elsewhere the docstring's text, such as
            // `list of str`, may not be an annotation at all.
            let (message, fix, title) = match (field.kind, field.subject()) {
                (FieldKind::Type, Some(name)) => (
                    format!("`{text}` puts a type in the docstring; annotate `{name}` in the signature instead"),
                    annotated.contains(name).then(|| field_deletion(checker, &target.docstring, &[field])).flatten(),
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
                    let fix = (field.kind == FieldKind::Param && annotated.contains(name))
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
            checker.report_fix(Rule::RstTypeField, range, message, fix, title);
        }
    }
}
