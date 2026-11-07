use ruff_python_ast::Stmt;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner, field_deletion, has_attribute_bases};

pub fn rst_unknown_attribute(checker: &Checker) {
    if checker.in_nested_project() {
        return;
    }
    for target in checker.state().rst_targets(checker) {
        // A base class may define the attribute a field documents.
        let Owner::Class(class) = target.owner else {
            continue;
        };
        if has_attribute_bases(class) {
            continue;
        }
        // A field may document a property, which autodoc renders as an attribute.
        let methods = class.body.iter().filter_map(Stmt::as_function_def_stmt).map(|method| method.name.as_str());
        let known: FxHashSet<&str> =
            target.attributes(checker).iter().map(|attribute| attribute.name).chain(methods).collect();
        for field in target.docstring.fields(&[FieldKind::Var, FieldKind::VarType]) {
            let Some(subject) = field.subject().filter(|subject| !known.contains(subject)) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RstUnknownAttribute,
                field.range_in(&target.docstring),
                format!("`{text}` documents `{subject}`, which `{}` does not have", target.name()),
                field_deletion(checker, &target.docstring, &[field]),
                format!("Remove `{text}`"),
            );
        }
    }
}
