use std::collections::HashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::Visibility;
use crate::rules::docstrings::rest::{FieldKind, Owner, rank, report_missing};

pub fn rest_missing_attribute(checker: &Checker) {
    for target in checker.rest_targets().iter().filter(|target| target.public) {
        let Owner::Class(_) = target.owner else {
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
        let missing = target
            .attributes(checker)
            .iter()
            .filter(|attribute| {
                Visibility::of(attribute.name) == Visibility::Public
                    && !attribute.documented
                    && !documented.contains(attribute.name)
                    && !parameters.iter().any(|parameter| parameter.name == attribute.name)
            })
            .map(|attribute| {
                let (name, field) = (attribute.name, if attribute.on_instance { "ivar" } else { "cvar" });
                let message = format!(
                    "Docstring of `{}` does not document attribute `{name}`; add `:{field} {name}:`",
                    target.name()
                );
                (rank(FieldKind::Var, None, &parameters), format!(":{field} {name}:"), message)
            })
            .collect();
        let range = target.docstring.range();
        report_missing(checker, Rule::RestMissingAttribute, range, &target.docstring, &parameters, missing);
    }
}
