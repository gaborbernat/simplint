use ruff_diagnostics::{Applicability, Edit};
use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, fix_of, statement_lines};
use crate::rules::{is_alone_on_lines, is_lazy};

pub fn up_lazy_modules_list(checker: &Checker) {
    let body = checker.body();
    let Some((list_stmt, modules)) = body.iter().find_map(|stmt| lazy_modules(stmt).map(|modules| (stmt, modules)))
    else {
        return;
    };
    let mut lazy_imports = Vec::new();
    let mut converted = Vec::new();
    let mut matched = false;
    for stmt in body {
        let Some(names) = plain_import_names(stmt) else {
            continue;
        };
        let listed: Vec<&str> = names.iter().copied().filter(|name| modules.contains(name)).collect();
        matched |= !listed.is_empty();
        // `lazy` covers the whole statement, so a statement mixing listed and unlisted modules keeps its entries.
        if is_lazy(stmt) {
            converted.extend(listed);
        } else if !listed.is_empty() && listed.len() == names.len() {
            lazy_imports.push(Edit::insertion("lazy ".to_owned(), stmt.start()));
            converted.extend(listed);
        }
    }
    if !matched {
        return;
    }
    let source = checker.source;
    let fix = (modules.iter().all(|module| converted.contains(module))
        && !stays_eager(checker, body, false, &modules)
        && is_alone_on_lines(source, list_stmt.range()))
    .then(|| {
        let mut edits = lazy_imports;
        edits.push(Edit::range_deletion(statement_lines(source, list_stmt.range(), Blanks::Keep)));
        fix_of(edits, Applicability::Safe)
    })
    .flatten();
    checker.report_fix(
        Rule::LazyModulesList,
        list_stmt.range(),
        "`__lazy_modules__` makes the listed imports lazy; spell each one as `lazy import` from Python 3.15",
        fix,
        "Use `lazy` on the listed imports and drop the list",
    );
}

// The modules of a module-level `__lazy_modules__` literal of strings, or `None` for any other value.
fn lazy_modules(stmt: &Stmt) -> Option<Vec<&str>> {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return None;
    };
    let [Expr::Name(target)] = targets.as_slice() else {
        return None;
    };
    if target.id.as_str() != "__lazy_modules__" {
        return None;
    }
    let elements: &[Expr] = match value.as_ref() {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        Expr::Set(set) => &set.elts,
        _ => return None,
    };
    elements.iter().map(|element| element.as_string_literal_expr().map(|literal| literal.value.to_str())).collect()
}

// Whether an import of a listed module stays eager once the list goes, which the fix must not allow. CPython keeps an
// import lazy under the list unless it is compiled inside a `try`, so the imports the fix leaves lazy are the ones under
// an `if`, `with`, `for`, `while` or `match`, the `else` of a `try` without a `finally`, and the `finally` of a `try`,
// which runs lazily on its normal path. The body and the handlers of a `try` are eager, and so is a function body.
fn stays_eager(checker: &Checker, body: &[Stmt], nested: bool, modules: &[&str]) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            nested && names.iter().any(|alias| modules.contains(&alias.name.as_str()))
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
            nested && modules.contains(&module.as_str())
        }
        // A relative import names the module after `from`, whatever it imports, so only that module is matched.
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, .. }) => {
            resolve_imported_module_path(*level, module.as_ref().map(ast::Identifier::as_str), checker.module_path())
                .is_some_and(|base| modules.contains(&base.as_ref()))
        }
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            stays_eager(checker, body, true, modules)
                || elif_else_clauses.iter().any(|clause| stays_eager(checker, &clause.body, true, modules))
        }
        Stmt::With(ast::StmtWith { body, .. }) => stays_eager(checker, body, true, modules),
        Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
            stays_eager(checker, body, true, modules) || stays_eager(checker, orelse, true, modules)
        }
        Stmt::Match(ast::StmtMatch { cases, .. }) => {
            cases.iter().any(|case| stays_eager(checker, &case.body, true, modules))
        }
        Stmt::Try(ast::StmtTry { orelse, finalbody, .. }) => {
            (finalbody.is_empty() && stays_eager(checker, orelse, true, modules))
                || stays_eager(checker, finalbody, true, modules)
        }
        _ => false,
    })
}

// The module names of an import that `lazy` can apply to, which are every name it binds. A relative import has no
// absolute name to match, and `from x import *` has no `lazy` form.
fn plain_import_names(stmt: &Stmt) -> Option<Vec<&str>> {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => Some(names.iter().map(|alias| alias.name.as_str()).collect()),
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. })
            if names.iter().all(|alias| alias.name.as_str() != "*") =>
        {
            Some(vec![module.as_str()])
        }
        _ => None,
    }
}
