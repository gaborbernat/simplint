use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, field_deletion, targets};

pub fn rest_unknown_param(checker: &Checker) {
    for target in targets(checker) {
        // `**kwargs` takes any keyword, which `:keyword name:` or `:param name:` fields document.
        let Some(parameters) = target.parameters().filter(|_| !target.takes_keywords()) else {
            continue;
        };
        let names: HashSet<&str> = parameters.iter().map(|parameter| parameter.name).collect();
        let fields =
            target.docstring.fields.iter().filter(|field| matches!(field.kind, FieldKind::Param | FieldKind::Type));
        for field in fields {
            let Some(subject) = field.subject().filter(|subject| !names.contains(subject)) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RestUnknownParam,
                field.marker.unwrap_or_else(|| target.docstring.range()),
                format!("`{text}` documents `{subject}`, which `{}` does not take", target.name()),
                field_deletion(checker, &target.docstring, &[field]),
                format!("Remove `{text}`"),
            );
        }
    }
}
