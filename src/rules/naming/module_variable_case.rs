use ruff_python_ast::helpers::is_dunder;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::Exports;
use crate::rules::is_class_like;
use crate::rules::naming::rename_fix;
use crate::semantic::BindingKind;

pub fn module_variable_case(checker: &Checker) {
    const MAGIC: &[&str] = &["pytestmark", "pytest_plugins", "collect_ignore", "collect_ignore_glob", "TYPE_CHECKING"];
    // Sphinx reads its settings off the `conf.py` module by their lowercase names.
    if checker.file_name() == "conf.py" {
        return;
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => None,
        Exports::Listed(names) => Some(names),
        Exports::Dynamic => return,
    };
    let mut found: Vec<(&str, TextRange, String)> = Vec::new();
    for (name, bindings) in checker.semantic().all_bindings() {
        if is_dunder(name)
            || is_class_like(name)
            || MAGIC.contains(&name)
            || bindings
                .iter()
                .any(|binding| matches!(binding.kind, BindingKind::Import | BindingKind::Function | BindingKind::Class))
            || bindings.iter().all(|binding| binding.kind == BindingKind::Target)
        {
            continue;
        }
        let private = if checker.project().is_referenced(name) {
            name.starts_with('_')
        } else {
            !exported.is_some_and(|names| names.contains(&name))
        };
        let expected =
            format!("{}{}", if private { "_" } else { "" }, name.trim_start_matches('_').to_ascii_uppercase());
        if expected != name {
            found.push((name, bindings[0].range, expected));
        }
    }
    found.sort_by_key(|(_, range, _)| range.start());
    for (name, range, expected) in found {
        let fix = rename_fix(checker, name, &expected);
        checker.report_fix(
            Rule::ModuleVariableCase,
            range,
            format!("Module-scope variable `{name}` should be named `{expected}`"),
            fix,
            format!("Rename `{name}` to `{expected}`"),
        );
    }
}
