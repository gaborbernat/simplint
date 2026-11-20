use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, is_internal};

pub fn rst_missing_param(checker: &Checker) {
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public(checker)) {
        let Some(parameters) = target.parameters() else {
            continue;
        };
        let documented: FxHashSet<&str> = target
            .docstring
            .subjects(&[FieldKind::Param])
            .chain(target.constructor.iter().flat_map(|docstring| docstring.subjects(&[FieldKind::Param])))
            .collect();
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
