use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::FieldKind;

pub fn rst_attribute_kind(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let attributes = target.attributes(checker);
        for field in target.docstring.fields(&[FieldKind::Var]) {
            let Some(attribute) =
                field.subject().and_then(|name| attributes.iter().find(|attribute| attribute.name == name))
            else {
                continue;
            };
            // `:var:` suits either, and a class-level default that methods set on `self` is both.
            let name = attribute.name;
            let (expected, what) = match field.words[0].as_str() {
                "ivar" if !attribute.on_instance => ("cvar", "a class variable"),
                "cvar" if !attribute.class_var => ("ivar", "an instance attribute"),
                _ => continue,
            };
            checker.report_fix(
                Rule::RstAttributeKind,
                field.range_in(&target.docstring),
                format!("`{}` documents {what}, `{name}`; write `:{expected} {name}:`", field.text()),
                field.rename(expected),
                format!("Write `:{expected} {name}:`"),
            );
        }
    }
}
