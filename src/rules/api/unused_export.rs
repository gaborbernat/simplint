use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::settings::ProjectKind;
use crate::syntax::{Exports, alias_binding, all_binding, all_literal, all_mutation, module_statements};

pub fn unused_export(checker: &Checker) {
    if checker.in_nested_project() {
        return;
    }
    let project = checker.project();
    // The `__all__` of a public module of a library declares what users outside the run import, so no name in it needs
    // an importer in the run. An `import *` of the module hands every listed name to the importing module, which may use
    // it there. A package's `__init__.py` lists what it re-exports, and Ruff, which counts its imports as re-exports,
    // puts any it leaves out back into `__all__`.
    // A module-level `__getattr__` serves names `__all__` lists without the module binding them, as a deprecation shim or
    // a lazy loader does, so whether anything imports them shows only outside the run.
    if checker.is_test_module()
        || checker.is_init_module()
        || project.role().public
        || project.is_star_import_source()
        || module_statements(checker.body())
            .any(|stmt| stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__getattr__"))
    {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    // A name another exported name's signature reaches is part of the interface the module exports. A dotted path that
    // names it, as a `mock.patch("pkg.mod.name")` target does, uses it as an import does.
    let unused: Vec<&str> = exported
        .iter()
        .copied()
        .filter(|name| !project.is_imported(name) && !project.is_named_by_path(name) && !project.is_api_reached(name))
        .collect();
    if unused.is_empty() {
        return;
    }
    let statements: Vec<&Stmt> = checker.body().iter().filter(|stmt| builds_all(stmt)).collect();
    // A list built over several statements has no one place to remove a name from.
    let literal = match statements.as_slice() {
        [stmt] => all_literal(stmt).filter(|_| all_binding(stmt).is_some_and(|binding| binding.operator.is_none())),
        _ => None,
    };
    let elements = literal.map_or(&[][..], |(_, elements)| elements);
    // Ruff's fix for the import this leaves unused drops every binding of a name imported twice from one module, as in
    // `from .question import Question, Question as Alias`, the one still in use included.
    let fix = literal
        .filter(|_| !unused.iter().any(|name| is_double_import(checker, name)))
        .and_then(|(value, elements)| removal(checker, value, elements, &unused));
    let title = format!(
        "Remove {} from `__all__`",
        unused.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
    );
    for name in unused {
        let range = elements
            .iter()
            .find(|element| element.as_string_literal_expr().is_some_and(|text| text.value.to_str() == name))
            .map_or_else(|| statements.first().map_or_else(TextRange::default, Ranged::range), Ranged::range);
        checker.report_fix(
            Rule::UnusedExport,
            range,
            format!("`{name}` is in `__all__`, yet no other module imports it; list only what other modules use"),
            fix.clone(),
            title.clone(),
        );
    }
}

fn builds_all(stmt: &Stmt) -> bool {
    all_binding(stmt).is_some() || all_mutation(stmt).is_some()
}

// Each run of unused names goes with the separator after it, or, at the end of the list, with the separator before it,
// so the layout and comments around the names kept stay as they are, whatever the layout. A comment inside a run may
// belong to a removed name, so the fix leaves such a list alone.
fn removal(checker: &Checker, value: TextRange, elements: &[Expr], unused: &[&str]) -> Option<Fix> {
    let source = checker.source;
    let is_unused =
        |element: &Expr| element.as_string_literal_expr().is_some_and(|text| unused.contains(&text.value.to_str()));
    let (open, close) = if source[value].starts_with('[') { ("[", "]") } else { ("(", ")") };
    let kept = elements.iter().filter(|element| !is_unused(element)).count();
    if kept == 0 {
        return (!checker.has_comment_in(value)).then(|| {
            Fix::applicable_edit(Edit::range_replacement(format!("{open}{close}"), value), applicability(checker))
        });
    }
    // A tuple left with one name needs a comma after it.
    let lone = open == "(" && kept == 1;
    let mut edits = Vec::new();
    let mut index = 0;
    while index < elements.len() {
        if !is_unused(&elements[index]) {
            index += 1;
            continue;
        }
        let first = index;
        while index < elements.len() && is_unused(&elements[index]) {
            index += 1;
        }
        let edit = if let Some(next) = elements.get(index) {
            Edit::range_deletion(TextRange::new(elements[first].start(), next.start()))
        } else {
            let range = TextRange::new(elements[first - 1].end(), elements[index - 1].end());
            let has_comma = source[range.end().to_usize()..].trim_start().starts_with(',');
            if lone && !has_comma {
                Edit::range_replacement(",".to_owned(), range)
            } else {
                Edit::range_deletion(range)
            }
        };
        if checker.has_comment_in(edit.range()) {
            return None;
        }
        edits.push(edit);
    }
    if lone
        && !elements.last().is_some_and(is_unused)
        && let Some(last) = elements.last()
        && !source[last.end().to_usize()..].trim_start().starts_with(',')
    {
        edits.push(Edit::insertion(",".to_owned(), last.end()));
    }
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    Some(Fix::applicable_edits(first, edits, applicability(checker)))
}

// Code outside the run, such as a library's users, may import a name the run does not; an application has no such code.
fn applicability(checker: &Checker) -> Applicability {
    if checker.settings.project.api.kind == ProjectKind::Application {
        Applicability::Safe
    } else {
        Applicability::Unsafe
    }
}

// Whether `name` is bound by importing a name the module also imports under another binding from the same module.
fn is_double_import(checker: &Checker, name: &str) -> bool {
    let imports: Vec<&ast::StmtImportFrom> = checker.body().iter().filter_map(Stmt::as_import_from_stmt).collect();
    imports.iter().any(|import| {
        import.names.iter().any(|alias| {
            alias_binding(alias) == name
                && imports
                    .iter()
                    .filter(|other| import_source(other) == import_source(import))
                    .flat_map(|other| &other.names)
                    .filter(|other| other.name.as_str() == alias.name.as_str())
                    .count()
                    > 1
        })
    })
}

fn import_source(import: &ast::StmtImportFrom) -> (u32, Option<&str>) {
    (import.level, import.module.as_ref().map(ast::Identifier::as_str))
}
