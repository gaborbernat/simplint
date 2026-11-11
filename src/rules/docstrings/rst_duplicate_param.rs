use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner};

pub fn rst_duplicate_param(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let Owner::Class(class) = target.owner else {
            continue;
        };
        let Some(constructor) = &target.constructor else {
            continue;
        };
        let documented: FxHashSet<&str> = target.docstring.subjects(&[FieldKind::Param]).collect();
        for field in constructor
            .fields(&[FieldKind::Param])
            .filter(|field| field.subject().is_some_and(|name| documented.contains(name)))
        {
            let text = field.text();
            checker.report_fix(
                Rule::RstDuplicateParam,
                field.range_in(constructor),
                format!(
                    "`{text}` repeats the docstring of `{}`, which autodoc shows; document each constructor parameter \
                     once",
                    class.name
                ),
                field.removal(checker, constructor),
                format!("Remove `{text}` from `__init__`"),
            );
        }
    }
}
