use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::Visibility;
use crate::rules::docstrings::rest::{FieldKind, Owner, attributes, field_insertion, rank};

pub fn rest_missing_attribute(checker: &Checker) {
    for target in checker.rest_targets().iter().filter(|target| target.public) {
        let Owner::Class(class) = target.owner else {
            continue;
        };
        let parameters = target.parameters().unwrap_or_default();
        let documented: HashSet<&str> = target
            .docstring
            .fields(FieldKind::Var)
            .chain(target.docstring.fields(FieldKind::VarType))
            .filter_map(|field| field.subject())
            .collect();
        // An attribute the constructor takes is documented by its `:param:` field, which `rest-missing-param` asks for.
        let missing: Vec<(&str, &str)> = attributes(checker, class)
            .into_iter()
            .filter(|attribute| {
                Visibility::of(attribute.name) == Visibility::Public
                    && !attribute.documented
                    && !documented.contains(attribute.name)
                    && !parameters.iter().any(|parameter| parameter.name == attribute.name)
            })
            .map(|attribute| (attribute.name, if attribute.on_instance { "ivar" } else { "cvar" }))
            .collect();
        let added: Vec<(usize, String)> = missing
            .iter()
            .map(|(name, field)| (rank(FieldKind::Var, None, &parameters), format!(":{field} {name}:")))
            .collect();
        let fix = field_insertion(checker, &target.docstring, &parameters, &added);
        let title = format!("Add {}", added.iter().map(|(_, text)| format!("`{text}`")).collect::<Vec<_>>().join(", "));
        for (name, field) in missing {
            checker.report_fix(
                Rule::RestMissingAttribute,
                target.docstring.range(),
                format!(
                    "Docstring of `{}` does not document attribute `{name}`; add `:{field} {name}:`",
                    target.name()
                ),
                fix.clone(),
                title.clone(),
            );
        }
    }
}
