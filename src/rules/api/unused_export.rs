use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::api::{AllList, is_alone_on_line};
use crate::rules::edits::fix_of;
use crate::settings::ProjectKind;
use crate::syntax::{Exports, alias_binding, all_binding, all_mutation, module_statements};

pub fn unused_export(checker: &Checker) {
    let project = checker.project();
    // The `__all__` of a public module of a library declares what users outside the run import, so no name in it needs
    // an importer in the run. An `import *` of the module hands every listed name to the importing module, which may
    // use it there, and so does a module that imports `__all__` from it or reads it off the module, as it iterates the
    // list. A package's `__init__.py` lists what it re-exports, and Ruff, which counts its imports as re-exports, puts
    // any it leaves out back into `__all__`.
    //
    // A module-level `__getattr__` serves names `__all__` lists without the module binding them, as a deprecation shim
    // or a lazy loader does, so whether anything imports them shows only outside the run.
    if checker.is_test_module()
        || checker.is_init_module()
        || project.role().public
        || project.is_star_import_source()
        || project.is_all_read()
        || project.is_read_whole()
        || module_statements(checker.body())
            .any(|stmt| stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__getattr__"))
    {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    // A name another exported name's signature reaches is part of the interface the module exports, and a dotted path
    // that names it, as a `mock.patch("pkg.mod.name")` target does, uses it as an import does. Code the index cannot
    // follow may still read a name, so the removal below is withheld for it, while the report stays.
    let unused: Vec<&str> = exported.names().iter().copied().filter(|name| !project.is_used_elsewhere(name)).collect();
    if unused.is_empty() {
        return;
    }
    let statements: Vec<&Stmt> = checker.body().iter().filter(|stmt| builds_all(stmt)).collect();
    // A list built over several statements has no one place to remove a name from.
    let list = match statements.as_slice() {
        [stmt] => AllList::of(stmt).filter(|_| all_binding(stmt).is_some_and(|binding| binding.operator.is_none())),
        _ => None,
    };
    // Ruff's fix for the import this leaves unused drops every binding of a name imported twice from one module, as in
    // `from .question import Question, Question as Alias`, the one still in use included.
    let doubled = doubled_bindings(checker);
    let title = format!(
        "Remove {} from `__all__`",
        unused.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
    );
    let mut offer = list
        .as_ref()
        .filter(|_| !unused.iter().any(|name| doubled.contains(name) || project.is_read_opaquely(name)))
        .and_then(|list| removal(checker, list, &unused.iter().copied().collect()))
        .map(|fix| (fix, title));
    let mut ranges: FxHashMap<&str, TextRange> = FxHashMap::default();
    for element in list.as_ref().map_or(&[][..], AllList::elements) {
        if let Some(text) = element.as_string_literal_expr() {
            ranges.entry(text.value.to_str()).or_insert(element.range());
        }
    }
    for name in unused {
        let range = ranges
            .get(name)
            .copied()
            .unwrap_or_else(|| statements.first().map_or_else(TextRange::default, Ranged::range));
        let (fix, title) = offer.take().map_or((None, String::new()), |(fix, title)| (Some(fix), title));
        checker.report_fix(
            Rule::UnusedExport,
            range,
            format!("`{name}` is in `__all__`, yet no other module imports it; list only what other modules use"),
            fix,
            title,
        );
    }
}

fn builds_all(stmt: &Stmt) -> bool {
    all_binding(stmt).is_some() || all_mutation(stmt).is_some()
}

fn doubled_bindings<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    let imports: Vec<&ast::StmtImportFrom> = checker.body().iter().filter_map(Stmt::as_import_from_stmt).collect();
    let mut counts: FxHashMap<((u32, Option<&str>), &str), usize> = FxHashMap::default();
    for import in &imports {
        for alias in &import.names {
            *counts.entry((import_source(import), alias.name.as_str())).or_default() += 1;
        }
    }
    imports
        .iter()
        .flat_map(|import| import.names.iter().map(move |alias| (import_source(import), alias)))
        .filter(|(source, alias)| counts[&(*source, alias.name.as_str())] > 1)
        .map(|(_, alias)| alias_binding(alias))
        .collect()
}

fn import_source(import: &ast::StmtImportFrom) -> (u32, Option<&str>) {
    (import.level, import.module.as_ref().map(ast::Identifier::as_str))
}

// Each run of unused names goes with the separator after it, or, at the end of the list, with the separator before it,
// so the layout and comments around the kept names stay as they are. A comment inside a run, or after a removed name
// that sits alone on its line, may belong to it, so the fix leaves such a list alone.
fn removal(checker: &Checker, list: &AllList, unused: &FxHashSet<&str>) -> Option<Fix> {
    let elements = list.elements();
    let is_unused =
        |element: &Expr| element.as_string_literal_expr().is_some_and(|text| unused.contains(text.value.to_str()));
    let (open, close) = list.brackets(checker);
    let kept = elements.iter().filter(|element| !is_unused(element)).count();
    if kept == 0 {
        return (!list.has_comment(checker)).then(|| {
            Fix::applicable_edit(
                Edit::range_replacement(format!("{open}{close}"), list.value()),
                applicability(checker),
            )
        });
    }
    // A parenthesized element's range starts at its parenthesis, which the deletion must take with it.
    let ranges: Vec<TextRange> = elements.iter().map(|element| list.element_range(checker, element)).collect();
    // A tuple left with one name needs a comma after it.
    let lone = open == "(" && kept == 1;
    let source = checker.source;
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
        if (first..index).any(|removed| owns_comment(checker, ranges[removed])) {
            return None;
        }
        let edit = if let Some(next) = ranges.get(index) {
            Edit::range_deletion(TextRange::new(ranges[first].start(), next.start()))
        } else {
            let range = TextRange::new(ranges[first - 1].end(), ranges[index - 1].end());
            if lone && !followed_by_comma(source, range.end()) {
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
        && let Some(last) = ranges.last()
        && !followed_by_comma(source, last.end())
    {
        edits.push(Edit::insertion(",".to_owned(), last.end()));
    }
    fix_of(edits, applicability(checker))
}

// Code outside the run, such as a library's users, may import a name the run does not; an application has no such code.
fn applicability(checker: &Checker) -> Applicability {
    if checker.settings.project.api.kind == ProjectKind::Application {
        Applicability::Safe
    } else {
        Applicability::Unsafe
    }
}

fn owns_comment(checker: &Checker, name: TextRange) -> bool {
    let Some(comment) = checker.trailing_comment(name) else {
        return false;
    };
    let alone = is_alone_on_line(checker, name);
    let before_comment = checker.text(TextRange::new(name.end(), comment.start())).trim();
    alone && matches!(before_comment, "" | ",")
}

fn followed_by_comma(source: &str, end: TextSize) -> bool {
    source[end.to_usize()..].trim_start().starts_with(',')
}
