use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, Owner, constructor_docstring, field_insertion, is_internal, rank};

pub fn rest_missing_param(checker: &Checker) {
    for target in checker.rest_targets().iter().filter(|target| target.public) {
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
        // A parameter with a leading underscore is internal, a knob the documentation leaves out.
        let missing: Vec<&str> = parameters
            .iter()
            .map(|parameter| parameter.name)
            .filter(|name| !is_internal(name, true) && !documented.contains(name))
            .collect();
        // The fields go where the constructor's parameters are documented already, the class docstring otherwise.
        let into =
            constructor.as_ref().filter(|docstring| docstring.has(FieldKind::Param)).unwrap_or(&target.docstring);
        let added: Vec<(usize, String)> = missing
            .iter()
            .map(|name| (rank(FieldKind::Param, Some(name), &parameters), format!(":param {name}:")))
            .collect();
        let fix = field_insertion(checker, into, &parameters, &added);
        let title = format!("Add {}", added.iter().map(|(_, text)| format!("`{text}`")).collect::<Vec<_>>().join(", "));
        for name in missing {
            checker.report_fix(
                Rule::RestMissingParam,
                target.docstring.range(),
                format!("Docstring of `{}` does not document parameter `{name}`; add `:param {name}:`", target.name()),
                fix.clone(),
                title.clone(),
            );
        }
    }
}
