use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::FieldKind;

pub fn rst_empty_field(checker: &Checker) {
    if checker.in_nested_project() {
        return;
    }
    for target in checker.state().rst_targets(checker) {
        let fields = target.docstring.fields(&[
            FieldKind::Param,
            FieldKind::Raises,
            FieldKind::Returns,
            FieldKind::Yields,
            FieldKind::Var,
        ]);
        // A malformed field is for rst-field-syntax to fix first, and it reports the field once.
        for field in fields.filter(|field| !field.described && field.problem().is_none()) {
            checker.report(
                Rule::RstEmptyField,
                field.range_in(&target.docstring),
                format!("Field `{}` has no description; say what it documents", field.text()),
            );
        }
    }
}
