use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::FieldKind;

pub fn rest_attribute_kind(checker: &Checker) {
    for target in checker.rest_targets() {
        let attributes = target.attributes(checker);
        for field in target.docstring.fields(FieldKind::Var) {
            let Some(attribute) =
                field.subject().and_then(|name| attributes.iter().find(|attribute| attribute.name == name))
            else {
                continue;
            };
            // `:var:` suits either, and a class-level default that methods set on `self` is both.
            let (word, name) = (field.words[0].as_str(), attribute.name);
            let (expected, what) = match word {
                "ivar" if !attribute.on_instance => ("cvar", "a class variable"),
                "cvar" if !attribute.class_var => ("ivar", "an instance attribute"),
                _ => continue,
            };
            let fix = field.marker.map(|marker| {
                Fix::unsafe_edit(Edit::range_replacement(
                    expected.to_owned(),
                    TextRange::at(marker.start(), TextSize::of(word)),
                ))
            });
            checker.report_fix(
                Rule::RestAttributeKind,
                field.marker.unwrap_or_else(|| target.docstring.range()),
                format!("`{}` documents {what}, `{name}`; write `:{expected} {name}:`", field.text()),
                fix,
                format!("Write `:{expected} {name}:`"),
            );
        }
    }
}
