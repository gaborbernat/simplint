use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, is_internal};
use rustc_hash::FxHashSet;

pub fn rst_missing_param(checker: &Checker) {
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public(checker)) {
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
        // A parameter with a leading underscore is internal, a knob the documentation leaves out. The description
        // of a parameter is for a person to write, so the report carries no fix.
        for name in parameters
            .iter()
            .map(|parameter| parameter.name)
            .filter(|name| !is_internal(name, true) && !documented.contains(name))
        {
            checker.report(
                Rule::RstMissingParam,
                target.docstring.range(),
                format!("Docstring of `{}` does not document parameter `{name}`; add `:param {name}:`", target.name()),
            );
        }
    }
}
