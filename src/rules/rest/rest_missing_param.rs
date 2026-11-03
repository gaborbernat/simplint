use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::rest::{FieldKind, Owner, constructor_docstring, targets};

pub fn rest_missing_param(checker: &Checker) {
    for target in targets(checker).into_iter().filter(|target| target.public) {
        let Some(parameters) = target.parameters() else {
            continue;
        };
        let constructor = match target.owner {
            Owner::Class(class) => constructor_docstring(checker, class),
            Owner::Function { .. } => None,
        };
        let documented: HashSet<&str> = target
            .docstring
            .fields(FieldKind::Param)
            .chain(constructor.iter().flat_map(|docstring| docstring.fields(FieldKind::Param)))
            .filter_map(|field| field.subject())
            .collect();
        for parameter in parameters.iter().filter(|parameter| !documented.contains(parameter.name)) {
            checker.report(
                Rule::RestMissingParam,
                target.docstring.range(),
                format!(
                    "Docstring of `{}` does not document parameter `{}`; add `:param {}:`",
                    target.name(),
                    parameter.name,
                    parameter.name
                ),
            );
        }
    }
}
