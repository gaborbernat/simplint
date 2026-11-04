use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, is_internal, rank, report_missing};
use rustc_hash::FxHashSet;

pub fn rst_missing_param(checker: &Checker) {
    for target in checker.rst_targets().iter().filter(|target| target.public) {
        let Some(parameters) = target.parameters() else {
            continue;
        };
        let constructor = target.constructor.as_deref();
        let documented: FxHashSet<&str> = target
            .docstring
            .fields(&[FieldKind::Param])
            .chain(constructor.iter().flat_map(|docstring| docstring.fields(&[FieldKind::Param])))
            .filter_map(|field| field.subject())
            .collect();
        // A parameter with a leading underscore is internal, a knob the documentation leaves out.
        let missing = parameters
            .iter()
            .map(|parameter| parameter.name)
            .filter(|name| !is_internal(name, true) && !documented.contains(name))
            .map(|name| {
                let message = format!(
                    "Docstring of `{}` does not document parameter `{name}`; add `:param {name}:`",
                    target.name()
                );
                (rank(FieldKind::Param, Some(name), &parameters), format!(":param {name}:"), message)
            })
            .collect();
        // The fields go where the constructor's parameters are documented already, the class docstring otherwise.
        let into = constructor.filter(|docstring| docstring.has(FieldKind::Param)).unwrap_or(&target.docstring);
        report_missing(checker, Rule::RstMissingParam, target.docstring.range(), into, &parameters, missing);
    }
}
