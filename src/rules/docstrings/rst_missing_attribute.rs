use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner};
use crate::syntax::Visibility;

pub fn rst_missing_attribute(checker: &Checker) {
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public(checker)) {
        let Owner::Class(_) = target.owner else {
            continue;
        };
        let parameters = target.parameters().unwrap_or_default();
        let documented: FxHashSet<&str> = target.docstring.subjects(&[FieldKind::Var, FieldKind::VarType]).collect();
        // An attribute the constructor takes is documented by its `:param:` field, which `rst-missing-param` asks for.
        // The description of an attribute is for a person to write, so the report carries no fix.
        for attribute in target.attributes(checker).iter().filter(|attribute| {
            Visibility::of(attribute.name) == Visibility::Public
                && !attribute.documented
                && !documented.contains(attribute.name)
                && !parameters.iter().any(|parameter| parameter.name == attribute.name)
        }) {
            let (name, field) = (attribute.name, if attribute.on_instance { "ivar" } else { "cvar" });
            checker.report(
                Rule::RstMissingAttribute,
                target.docstring.range(),
                format!(
                    "Docstring of `{}` does not document attribute `{name}`; add `:{field} {name}:`",
                    target.name()
                ),
            );
        }
    }
}
