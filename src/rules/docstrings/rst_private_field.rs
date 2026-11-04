use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, field_deletion, is_internal};

pub fn rst_private_field(checker: &Checker) {
    // A public class's `__init__` docstring documents its constructor as the class docstring may.
    for target in checker.rst_targets().iter().filter(|target| target.public || target.public_constructor) {
        for field in &target.docstring.fields {
            let parameter = matches!(field.kind, FieldKind::Param | FieldKind::Type);
            if !parameter && !matches!(field.kind, FieldKind::Var | FieldKind::VarType) {
                continue;
            }
            let Some(name) = field.subject().filter(|name| is_internal(name, parameter)) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RstPrivateField,
                field.range_in(&target.docstring),
                format!("`{text}` documents the internal `{name}` in the public API; leave internal names out"),
                field_deletion(checker, &target.docstring, &[field]),
                format!("Remove `{text}`"),
            );
        }
    }
}
