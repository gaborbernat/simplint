use rustc_hash::FxHashSet;

use ruff_diagnostics::{Applicability, Edit};
use ruff_text_size::Ranged;

use super::lazy_modules::{Declaration, Module, declaration, imported_modules, module_imports};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, fix_of, statement_lines};
use crate::syntax::is_lazy;

pub fn up_lazy_modules_list(checker: &Checker) {
    let Declaration::Listed(list) = declaration(checker) else {
        return;
    };
    let module_path = checker.module_path();
    let modules: Vec<Option<Module>> = list
        .entries
        .iter()
        .map(|entry| (!entry.written_relative).then(|| entry.module.clone().resolve(module_path)))
        .collect();
    let listed: FxHashSet<&Module> = modules.iter().flatten().collect();
    let imports = module_imports(checker.body());
    let mut lazy_imports = Vec::new();
    let mut converted = FxHashSet::default();
    let mut mixed = false;
    for import in imports.iter().filter(|import| import.top) {
        let names: Vec<Module> =
            imported_modules(import.stmt).into_iter().map(|imported| imported.module.resolve(module_path)).collect();
        let listed_names: Vec<&Module> = names.iter().filter(|name| listed.contains(name)).collect();
        if listed_names.is_empty() {
            continue;
        }
        // `lazy` covers the whole statement, so a statement mixing listed and unlisted modules keeps its entries.
        if is_lazy(import.stmt) {
            converted.extend(listed_names.into_iter().cloned());
        } else if listed_names.len() == names.len() {
            lazy_imports.push(Edit::insertion("lazy ".to_owned(), import.stmt.start()));
            converted.extend(listed_names.into_iter().cloned());
        } else {
            mixed = true;
        }
    }
    if converted.is_empty() && !mixed {
        return;
    }
    // An import the list defers in a block keeps no `lazy` from the fix, so it would load eagerly once the list goes,
    // while one above the list loads eagerly already and must not gain one.
    let stays_eager = imports
        .iter()
        .filter(|import| {
            (!import.top || import.stmt.start() < list.stmt.start())
                && import.governed
                && !import.dead
                && !is_lazy(import.stmt)
        })
        .any(|import| {
            imported_modules(import.stmt)
                .into_iter()
                .any(|imported| listed.contains(&imported.module.resolve(module_path)))
        });
    let fix = (modules.iter().all(|module| module.as_ref().is_some_and(|module| converted.contains(module)))
        && !mixed
        && !stays_eager
        && checker.is_alone_on_lines(list.stmt.range()))
    .then(|| {
        let mut edits = lazy_imports;
        edits.push(Edit::range_deletion(statement_lines(checker.source, list.stmt.range(), Blanks::Keep)));
        fix_of(edits, Applicability::Safe)
    })
    .flatten();
    checker.report_fix(
        Rule::LazyModulesList,
        list.stmt.range(),
        "`__lazy_modules__` makes the listed imports lazy; spell each one as `lazy import` from Python 3.15",
        fix,
        "Use `lazy` on the listed imports and drop the list",
    );
}
