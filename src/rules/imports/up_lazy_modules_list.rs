use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::is_alone_on_lines;

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
        if !listed.is_empty() && listed.len() == names.len() {
            lazy_imports.push(Edit::insertion("lazy ".to_owned(), stmt.start()));
            converted.extend(listed);
        }
    }
    if !matched {
        return;
    }
    let source = checker.source;
    let fix = (modules.iter().all(|module| converted.contains(module)) && is_alone_on_lines(source, list_stmt.range()))
        .then(|| {
            let mut edits = lazy_imports;
            edits.push(Edit::range_deletion(source.full_lines_range(list_stmt.range())));
            let rest = edits.split_off(1);
            Fix::applicable_edits(edits.remove(0), rest, Applicability::Safe)
        });
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
