use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use super::lazy_modules::{
    Declaration, LazyList, Module, declaration, distinct_modules, imported_modules, lazy_keyword_removal,
    module_imports,
};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::is_lazy;

const IMPORT_ERRORS: [&str; 4] = ["ImportError", "ModuleNotFoundError", "Exception", "BaseException"];

pub fn suppressed_lazy_import(checker: &Checker) {
    let declaration = declaration(checker);
    let list = match &declaration {
        Declaration::Listed(list) => Some(list),
        Declaration::Absent | Declaration::Unknown => None,
    };
    for stmt in checker.body() {
        let Stmt::With(ast::StmtWith { items, body, .. }) = stmt else {
            continue;
        };
        let Some(guard) = items
            .iter()
            .find(|item| suppresses_import_error(checker, &item.context_expr))
            .map(|item| checker.text(&item.context_expr))
        else {
            continue;
        };
        for import in body {
            if is_lazy(import) {
                checker.report_fix(
                    Rule::SuppressedLazyImport,
                    import.range(),
                    format!("`lazy` defers this import past `{guard}`, so a missing module raises at its first use"),
                    Some(Fix::unsafe_edit(lazy_keyword_removal(checker, import))),
                    "Remove `lazy`",
                );
            } else if let Some(list) = list {
                listed_import(checker, list, import, guard);
            }
        }
    }
}

fn listed_import(checker: &Checker, list: &LazyList, import: &Stmt, guard: &str) {
    let module_path = checker.module_path();
    let keys: Vec<Module> = list.entries.iter().map(|entry| entry.module.clone().resolve(module_path)).collect();
    for imported in distinct_modules(import) {
        let key = imported.module.resolve(module_path);
        let mut matching =
            keys.iter().enumerate().filter(|(index, listed)| !list.entries[*index].written_relative && **listed == key);
        let Some((index, _)) = matching.next() else {
            continue;
        };
        // The entry defers every other import of the module too, so it goes only when this import is the one it defers.
        let alone = matching.next().is_none()
            && module_imports(checker.body())
                .iter()
                .filter(|other| other.governed && !other.dead && !is_lazy(other.stmt))
                .filter(|other| {
                    imported_modules(other.stmt).into_iter().any(|other| other.module.resolve(module_path) == key)
                })
                .count()
                == 1;
        let fix = alone
            .then(|| {
                let mut items = list.items(checker)?;
                items.remove(index);
                list.replacement(checker, &items)
            })
            .flatten()
            .map(Fix::unsafe_edit);
        checker.report_fix(
            Rule::SuppressedLazyImport,
            imported.range,
            format!(
                "`__lazy_modules__` defers `{key}` past `{guard}`, so a missing module raises at its first use on Python 3.15"
            ),
            fix,
            format!("Remove `{}` from `__lazy_modules__`", checker.text(list.entries[index].expr)),
        );
    }
}

fn suppresses_import_error(checker: &Checker, expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return false;
    };
    let semantic = checker.semantic();
    semantic.resolves_to(func, &["contextlib", "suppress"])
        && arguments.args.iter().any(|argument| IMPORT_ERRORS.iter().any(|name| semantic.is_builtin(argument, name)))
}
