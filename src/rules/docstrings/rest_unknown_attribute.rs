use std::collections::HashSet;

use ruff_python_ast::Stmt;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, Owner, attributes, field_deletion, has_attribute_bases, targets};

pub fn rest_unknown_attribute(checker: &Checker) {
    for target in targets(checker) {
        // A base class may define the attribute a field documents.
        let Owner::Class(class) = target.owner else {
            continue;
        };
        if has_attribute_bases(class) {
            continue;
        }
        // A field may document a property, which autodoc renders as an attribute.
        let methods = class.body.iter().filter_map(Stmt::as_function_def_stmt).map(|method| method.name.as_str());
        let known: HashSet<&str> =
            attributes(checker, class).iter().map(|attribute| attribute.name).chain(methods).collect();
        let fields =
            target.docstring.fields.iter().filter(|field| matches!(field.kind, FieldKind::Var | FieldKind::VarType));
        for field in fields {
            let Some(subject) = field.subject().filter(|subject| !known.contains(subject)) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RestUnknownAttribute,
                field.marker.unwrap_or_else(|| target.docstring.range()),
                format!("`{text}` documents `{subject}`, which `{}` does not have", target.name()),
                field_deletion(checker, &target.docstring, &[field]),
                format!("Remove `{text}`"),
            );
        }
    }
}
