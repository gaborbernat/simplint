use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, field_deletion};

pub fn rest_unknown_param(checker: &Checker) {
    for target in checker.rest_targets() {
        // `**kwargs` takes any keyword, which `:keyword name:` or `:param name:` fields document.
        let Some(parameters) = target.parameters().filter(|_| !target.takes_keywords()) else {
            continue;
        };
        let names: HashSet<&str> = parameters.iter().map(|parameter| parameter.name).collect();
        for field in target.docstring.fields(&[FieldKind::Param, FieldKind::Type]) {
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
