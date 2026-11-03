use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{Field, FieldKind, Owner, constructor_docstring, field_deletion, targets};

pub fn rest_duplicate_param(checker: &Checker) {
    for target in targets(checker) {
        let Owner::Class(class) = target.owner else {
            continue;
        };
        let Some(constructor) = constructor_docstring(checker, class) else {
            continue;
        };
        let documented: HashSet<&str> = target.docstring.fields(FieldKind::Param).filter_map(Field::subject).collect();
        let repeated = constructor
            .fields(FieldKind::Param)
            .filter(|field| field.subject().is_some_and(|name| documented.contains(name)));
        for field in repeated {
            let text = field.text();
            checker.report_fix(
                Rule::RestDuplicateParam,
                field.marker.unwrap_or_else(|| constructor.range()),
                format!(
                    "`{text}` repeats the docstring of `{}`, which autodoc shows; document each constructor parameter once",
                    class.name
                ),
                field_deletion(checker, &constructor, &[field]),
                format!("Remove `{text}` from `__init__`"),
            );
        }
    }
}
