use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::{body_without_leading_docstring, is_dunder};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::future_import;
use crate::syntax::{Exports, alias_binding, all_binding, all_mutation, is_name};

pub fn init_module_content(checker: &Checker) {
    if !checker.is_init_module() {
        return;
    }
    // A public package of a library may present the API its private modules define, as PEP 8 describes for a package's
    // `__init__` that exposes functionality from submodules.
    let facade = checker.project().role().public;
    let message = if facade {
        "`__init__.py` holds code; keep only its docstring and the re-exports of its public API"
    } else {
        "`__init__.py` holds code; keep only its docstring"
    };
    let body = body_without_leading_docstring(checker.body());
    for (index, stmt) in body.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| &body[previous]);
        let reexport = facade && is_reexport(stmt, checker.dunder_all());
        if !reexport && !left_to_ruff(checker, stmt, previous) {
            checker.report(Rule::InitModuleContent, stmt.range(), message);
        }
    }
}

// RUF067 reports the statements of an `__init__.py` that this rule reports, when its preview mode is on and the project
// selects it. Ruff allows the imports, dunder assignments and `if TYPE_CHECKING` blocks of a package unless the project
// bans all code, and an attribute docstring, which follows an assignment, is documentation rather than code.
fn left_to_ruff(checker: &Checker, stmt: &Stmt, previous: Option<&Stmt>) -> bool {
    let from_ruff = &checker.settings.from_ruff;
    from_ruff.preview
        && from_ruff.ruff_runs(RuffRule::NonEmptyInitModule)
        && !is_attribute_docstring(stmt, previous)
        && (from_ruff.strictly_empty_init_modules || !is_allowed_by_default(stmt, checker))
}

fn is_attribute_docstring(stmt: &Stmt, previous: Option<&Stmt>) -> bool {
    let is_string = matches!(stmt, Stmt::Expr(ast::StmtExpr { value, .. }) if value.is_string_literal_expr());
    is_string
        && match previous {
            Some(Stmt::Assign(ast::StmtAssign { targets, .. })) => matches!(targets.as_slice(), [Expr::Name(_)]),
            Some(Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })) => target.is_name_expr(),
            _ => false,
        }
}

// The statements RUF067 allows without `strictly-empty-init-modules`: imports, the PEP 562 `__getattr__` and `__dir__`,
// `if TYPE_CHECKING` blocks, and assignments whose targets are all dunder names.
fn is_allowed_by_default(stmt: &Stmt, checker: &Checker) -> bool {
    match stmt {
        Stmt::Import(_) | Stmt::ImportFrom(_) => true,
        Stmt::FunctionDef(function) => matches!(function.name.as_str(), "__getattr__" | "__dir__"),
        Stmt::If(ast::StmtIf { test, .. }) => is_type_checking(test, checker),
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().all(is_dunder_name),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            is_dunder_name(target)
        }
        _ => false,
    }
}

fn is_dunder_name(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| is_dunder(name.id.as_str()))
}

// Ruff's test: a name `TYPE_CHECKING` by its own spelling or through `typing`, or any `.TYPE_CHECKING` attribute.
fn is_type_checking(test: &Expr, checker: &Checker) -> bool {
    match test {
        Expr::Name(name) => {
            name.id.as_str() == "TYPE_CHECKING" || checker.semantic().match_typing_expr(test, "TYPE_CHECKING")
        }
        Expr::Attribute(attribute) => attribute.attr.as_str() == "TYPE_CHECKING",
        _ => false,
    }
}

// An import whose every name the package exports, through `__all__`, a redundant alias or `import *`, the `__all__`
// that lists them, `from __future__` imports, which change no behavior at run time, and the `__lazy_modules__` that
// Python 3.15 reads from the package's globals to load the imports after it lazily.
fn is_reexport(stmt: &Stmt, exports: &Exports) -> bool {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            future_import(stmt).is_some()
                || names.iter().all(|alias| {
                    let bound = alias_binding(alias);
                    alias.name.as_str() == "*"
                        || alias.asname.as_ref().is_some_and(|asname| asname.as_str() == alias.name.as_str())
                        || matches!(exports, Exports::Listed(names) if names.contains(&bound))
                })
        }
        _ => all_binding(stmt).is_some() || all_mutation(stmt).is_some() || is_lazy_modules_binding(stmt),
    }
}

fn is_lazy_modules_binding(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().any(|target| is_name(target, "__lazy_modules__"))
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            is_name(target, "__lazy_modules__")
        }
        _ => false,
    }
}
