use rustc_hash::FxHashSet;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use super::lazy_modules::{
    Declaration, LazyList, Module, ModuleImport, declaration, distinct_modules, enclosing_packages, imported_modules,
    lazy_keyword_removal, list_position, module_imports,
};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, statement_lines};
use crate::semantic::future_import;
use crate::syntax::{is_lazy, statement_index};

pub fn ineffective_lazy_declaration(checker: &Checker) {
    let module_path = checker.module_path();
    let enclosing = enclosing_packages(module_path);
    let imports = module_imports(checker.body());
    let mut listed = FxHashSet::default();
    if let Declaration::Listed(list) = declaration(checker) {
        listed = list
            .entries
            .iter()
            .filter(|entry| !entry.written_relative)
            .map(|entry| entry.module.clone().resolve(module_path))
            .collect();
        list_entries(checker, &list, &imports, &enclosing);
        late_list(checker, &list, &imports, &listed);
    }
    lazy_keywords(checker, &imports, &enclosing, &listed);
}

fn list_entries(checker: &Checker, list: &LazyList, imports: &[ModuleImport], enclosing: &[Module]) {
    let module_path = checker.module_path();
    let runs = || imports.iter().filter(|import| !import.dead);
    let keys = |imports: Vec<&ModuleImport>| -> FxHashSet<Module> {
        imports
            .into_iter()
            .flat_map(|import| imported_modules(import.stmt))
            .map(|imported| imported.module.resolve(module_path))
            .collect()
    };
    let deferred = keys(runs().filter(|import| import.governed && !is_lazy(import.stmt)).collect());
    let from_imported = keys(runs().filter(|import| import.governed && import.stmt.is_import_from_stmt()).collect());
    let lazy = keys(runs().filter(|import| is_lazy(import.stmt)).collect());
    let mut seen = FxHashSet::default();
    for (index, entry) in list.entries.iter().enumerate() {
        let text = checker.text(entry.expr);
        if entry.written_relative {
            let spelled = entry.module.entry(list.quote(checker));
            checker.report_fix(
                Rule::IneffectiveLazyDeclaration,
                entry.expr.range(),
                format!(
                    "`{text}` is relative, but CPython matches the absolute names of imports, so it defers nothing"
                ),
                Some(Fix::unsafe_edit(Edit::range_replacement(spelled.clone(), entry.expr.range()))),
                format!("Spell it `{spelled}`"),
            );
            continue;
        }
        let key = entry.module.clone().resolve(module_path);
        let message = if !seen.insert(key.clone()) {
            format!("`{key}` is listed twice")
        } else if enclosing.contains(&key) && !from_imported.contains(&key) {
            format!("`{key}` holds this file, so it has loaded before the file runs")
        } else if key.is_loaded_at_startup() {
            format!("`{key}` loads when Python starts, so listing it defers nothing")
        } else if deferred.contains(&key) {
            continue;
        } else if lazy.contains(&key) {
            format!("`{key}` is imported with `lazy` only, which defers it without the list")
        } else if matches!(key, Module::Relative { .. }) {
            // Outside a package a relative entry has no absolute name to compare with the imports.
            continue;
        } else {
            format!("No module-level import of `{key}` runs where the list can defer it")
        };
        let fix = list.items(checker).and_then(|mut items| {
            items.remove(index);
            list.replacement(checker, &items)
        });
        checker.report_fix(
            Rule::IneffectiveLazyDeclaration,
            entry.expr.range(),
            message,
            fix.map(Fix::safe_edit),
            format!("Remove `{text}`"),
        );
    }
}

// The list defers only the imports below it, so a listed module imported above it has loaded eagerly. The fix moves
// the list above the first import.
fn late_list(checker: &Checker, list: &LazyList, imports: &[ModuleImport], listed: &FxHashSet<Module>) {
    let module_path = checker.module_path();
    let Some(early) = imports
        .iter()
        .filter(|import| {
            import.governed && !import.dead && !is_lazy(import.stmt) && import.stmt.start() < list.stmt.start()
        })
        .flat_map(|import| imported_modules(import.stmt))
        .map(|imported| imported.module.resolve(module_path))
        .find(|key| listed.contains(key))
    else {
        return;
    };
    let body = checker.body();
    let first = imports
        .iter()
        .find(|import| future_import(import.stmt).is_none())
        .map(|import| &body[statement_index(body, import.stmt.start())]);
    let source = checker.source;
    let fix = first
        .filter(|_| checker.is_alone_on_lines(list.stmt.range()) && checker.comment_above(list.stmt).is_none())
        .and_then(|first| list_position(checker, first))
        .map(|position| {
            let lines = source.full_lines_range(list.stmt.range());
            let eol = checker.line_ending();
            Fix::applicable_edits(
                Edit::insertion(format!("{}{eol}", &source[lines]), position),
                [Edit::range_deletion(statement_lines(source, list.stmt.range(), Blanks::WiderSide))],
                Applicability::Unsafe,
            )
        });
    checker.report_fix(
        Rule::IneffectiveLazyDeclaration,
        list.stmt.range(),
        format!("`__lazy_modules__` comes after the import of `{early}`, which has loaded eagerly by then"),
        fix,
        "Move `__lazy_modules__` above the imports",
    );
}

fn lazy_keywords(checker: &Checker, imports: &[ModuleImport], enclosing: &[Module], listed: &FxHashSet<Module>) {
    let module_path = checker.module_path();
    let eager: FxHashSet<Module> = imports
        .iter()
        .filter(|import| import.top && !is_lazy(import.stmt))
        .flat_map(|import| imported_modules(import.stmt))
        .map(|imported| imported.module.resolve(module_path))
        // A package holding the file has loaded however it is imported, so its eager import loads nothing.
        .filter(|key| !listed.contains(key) && !enclosing.contains(key))
        .collect();
    for import in imports.iter().filter(|import| !import.dead && is_lazy(import.stmt)) {
        let modules = distinct_modules(import.stmt);
        let single = modules.len() == 1;
        for imported in modules {
            let key = imported.module.resolve(module_path);
            let (message, removable) = if import.stmt.is_import_stmt() && enclosing.contains(&key) {
                (format!("`{key}` holds this file, so it has loaded before `lazy` could defer it"), true)
            } else if key.is_loaded_at_startup() {
                (format!("`{key}` loads when Python starts, so `lazy` defers nothing"), true)
            } else if eager.contains(&key) {
                (format!("`{key}` is also imported eagerly at module level, so `lazy` defers nothing"), false)
            } else {
                continue;
            };
            let fix = (removable && single).then(|| Fix::safe_edit(lazy_keyword_removal(checker, import.stmt)));
            checker.report_fix(Rule::IneffectiveLazyDeclaration, imported.range, message, fix, "Remove `lazy`");
        }
    }
}
