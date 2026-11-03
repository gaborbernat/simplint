use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::{Exports, all_binding, all_literal, is_all_target};
use crate::settings::ProjectKind;

pub fn unused_export(checker: &Checker) {
    let project = checker.project();
    // The `__all__` of a library's top-level package, or of a module `api-modules` names, declares what users outside the
    // run import; an `import *` of the module hands every listed name to the importing module, which may use it there.
    if checker.is_test_module() || project.role().entry || project.is_star_import_source() {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    let unused: Vec<&str> = exported.iter().copied().filter(|name| !project.is_imported(name)).collect();
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
    let fix = literal.and_then(|(value, elements)| removal(checker, value, elements, &unused));
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
    all_binding(stmt).is_some()
        || stmt.as_expr_stmt().is_some_and(|expr| {
            expr.value.as_call_expr().is_some_and(|call| {
                call.func.as_attribute_expr().is_some_and(|attribute| is_all_target(&attribute.value))
            })
        })
}

// Names listed one per line lose their lines; a list on one line is written again without them. A comment inside the
// list may belong to a name, so the fix leaves such a list alone unless each name has a line of its own.
fn removal(checker: &Checker, value: TextRange, elements: &[Expr], unused: &[&str]) -> Option<Fix> {
    let source = checker.source;
    let is_unused =
        |element: &Expr| element.as_string_literal_expr().is_some_and(|text| unused.contains(&text.value.to_str()));
    let (open, close) = if source[value].starts_with('[') { ("[", "]") } else { ("(", ")") };
    let kept: Vec<&str> =
        elements.iter().filter(|element| !is_unused(element)).map(|element| checker.text(element)).collect();
    let one_per_line = elements.iter().all(|element| {
        source[TextRange::new(source.line_start(element.start()), element.start())].trim().is_empty()
            && source[TextRange::new(element.end(), source.line_end(element.end()))]
                .trim()
                .trim_start_matches(',')
                .trim()
                .is_empty()
    });
    let edit = if kept.is_empty() {
        Edit::range_replacement(format!("{open}{close}"), value)
    } else if source.contains_line_break(value) && one_per_line {
        let mut edits = elements
            .iter()
            .filter(|element| is_unused(element))
            .map(|element| Edit::range_deletion(source.full_lines_range(element.range())));
        let first = edits.next()?;
        return Some(applicability(checker, Fix::safe_edits(first, edits)));
    } else if checker.has_comment_in(value) {
        return None;
    } else {
        // A tuple of one needs its trailing comma.
        let comma = if open == "(" && kept.len() == 1 { "," } else { "" };
        Edit::range_replacement(format!("{open}{}{comma}{close}", kept.join(", ")), value)
    };
    Some(applicability(checker, Fix::safe_edit(edit)))
}

// Code outside the run, such as a library's users, may import a name the run does not; an application has no such code.
fn applicability(checker: &Checker, fix: Fix) -> Fix {
    if checker.settings.project.api.kind == ProjectKind::Application {
        fix
    } else {
        Fix::unsafe_edits(fix.edits()[0].clone(), fix.edits()[1..].iter().cloned())
    }
}
