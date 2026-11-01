use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::PythonVersion;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{delete_statement, is_alone_on_lines};
use crate::semantic::future_import;

pub fn future_annotations(checker: &Checker) {
    if checker.settings.from_ruff.target_version < PythonVersion::PY314 {
        return;
    }
    let body = checker.body();
    for stmt in body {
        let Some(import) = future_import(stmt) else {
            continue;
        };
        let Some(alias) = import.names.iter().find(|alias| alias.name.as_str() == "annotations") else {
            continue;
        };
        let edit = if import.names.len() > 1 {
            let kept: Vec<&str> = import
                .names
                .iter()
                .filter(|other| other.name.as_str() != "annotations")
                .map(|other| checker.text(other.range()))
                .collect();
            Some(Edit::range_replacement(format!("from __future__ import {}", kept.join(", ")), import.range()))
        } else {
            is_alone_on_lines(checker.source, stmt.range()).then(|| delete_statement(checker.source, stmt, body))
        };
        // Lazy annotations still evaluate on access, so a name imported only for type checking raises there.
        let applicability =
            if checker.name_token_count("TYPE_CHECKING") > 0 { Applicability::Unsafe } else { Applicability::Safe };
        checker.report_fix(
            Rule::FutureAnnotations,
            alias.range(),
            "Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`",
            edit.map(|edit| Fix::applicable_edit(edit, applicability)),
            "Remove the `annotations` future import",
        );
    }
}
