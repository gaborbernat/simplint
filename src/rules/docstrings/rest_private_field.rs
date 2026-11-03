use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, field_deletion, targets};

pub fn rest_private_field(checker: &Checker) {
    for target in targets(checker).into_iter().filter(|target| target.public) {
        let fields = target.docstring.fields.iter().filter(|field| {
            matches!(field.kind, FieldKind::Param | FieldKind::Type | FieldKind::Var | FieldKind::VarType)
        });
        for field in fields {
            let Some(name) = field.subject().filter(|name| name.starts_with('_')) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RestPrivateField,
                field.marker.unwrap_or_else(|| target.docstring.range()),
                format!("`{text}` documents the internal `{name}` in the public API; leave internal names out"),
                field_deletion(checker, &target.docstring, &[field]),
                format!("Remove `{text}`"),
            );
        }
    }
}
