use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner, rank, report_missing};
use crate::syntax::Visibility;
use rustc_hash::FxHashSet;

pub fn rst_missing_attribute(checker: &Checker) {
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public) {
        let Owner::Class(_) = target.owner else {
            continue;
        };
        let parameters = target.parameters().unwrap_or_default();
        let documented: FxHashSet<&str> = target
            .docstring
            .fields(&[FieldKind::Var, FieldKind::VarType])
            .filter_map(|field| field.subject())
            .collect();
        // An attribute the constructor takes is documented by its `:param:` field, which `rst-missing-param` asks for.
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
        report_missing(checker, Rule::RstMissingAttribute, range, &target.docstring, &parameters, missing);
    }
}
