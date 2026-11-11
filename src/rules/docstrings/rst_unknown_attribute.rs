use ruff_python_ast::Stmt;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner, has_attribute_bases};
use crate::syntax::module_statements;

pub fn rst_unknown_attribute(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let Owner::Class(class) = target.owner else {
            continue;
        };
        // A base class may define the attribute a field documents.
        if has_attribute_bases(class) {
            continue;
        }
        let known: FxHashSet<&str> = target
            .attributes(checker)
            .iter()
            .map(|attribute| attribute.name)
            // A field may document a property, which autodoc renders as an attribute.
            .chain(
                module_statements(&class.body)
                    .filter_map(Stmt::as_function_def_stmt)
                    .map(|method| method.name.as_str()),
            )
            .collect();
        for field in target.docstring.fields(&[FieldKind::Var, FieldKind::VarType]) {
            let Some(subject) = field.subject().filter(|subject| !known.contains(subject)) else {
                continue;
            };
            let text = field.text();
            checker.report_fix(
                Rule::RstUnknownAttribute,
                field.range_in(&target.docstring),
                format!("`{text}` documents `{subject}`, which `{}` does not have", target.name()),
                field.removal(checker, &target.docstring),
                format!("Remove `{text}`"),
            );
        }
    }
}
