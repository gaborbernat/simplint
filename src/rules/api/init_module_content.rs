use ruff_python_ast::helpers::{body_without_leading_docstring, collect_import_from_member, is_dunder};
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::is_type_checking;
use crate::semantic::future_import;
use crate::syntax::{Exports, alias_binding, all_binding, all_mutation, import_aliases, is_name};

pub(crate) fn init_module_content(checker: &Checker) {
    if !checker.is_init_module() || checker.is_test_module() || checker.settings.project.nested {
        return;
    }
    // A package can expose private implementation through a deliberate `__all__`; public library packages can also
    // use the implicit re-export forms PEP 8 describes.
    let facade = checker.project().role().public;
    let explicit_api = matches!(checker.dunder_all(), Exports::Listed(_))
        && checker.body().iter().any(|stmt| all_binding(stmt).is_some_and(|binding| binding.operator.is_none()));
    let public = facade || explicit_api;
    let message = if public {
        "`__init__.py` holds code; keep only its docstring and the re-exports of its public API"
    } else {
        "`__init__.py` holds code; keep only its docstring"
    };
    let body = body_without_leading_docstring(checker.body());
    for (index, stmt) in body.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| &body[previous]);
        let reexport = public && is_reexport(stmt, checker.dunder_all(), facade);
        if !reexport
            && !is_package_hook(stmt, checker)
            && !left_to_ruff(checker, stmt, previous)
            && !left_to_unused_import(checker, stmt, public, facade)
        {
            checker.report(Rule::InitModuleContent, stmt.range(), message);
        }
    }
}

// `from __future__` imports change no run-time behavior, and Python 3.15 reads `__lazy_modules__` from the package's
// globals, so neither is reported.
fn is_reexport(stmt: &Stmt, exports: &Exports, facade: bool) -> bool {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            future_import(stmt).is_some() || names.iter().all(|alias| is_reexported_alias(alias, exports, facade))
        }
        _ => all_binding(stmt).is_some() || all_mutation(stmt).is_some() || is_lazy_modules_binding(stmt),
    }
}

fn is_reexported_alias(alias: &ast::Alias, exports: &Exports, facade: bool) -> bool {
    facade
        && (alias.name.as_str() == "*"
            || alias.asname.as_ref().is_some_and(|asname| asname.as_str() == alias.name.as_str()))
        || exports.lists(alias_binding(alias))
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

fn is_package_hook(stmt: &Stmt, checker: &Checker) -> bool {
    match stmt {
        Stmt::FunctionDef(function) => matches!(function.name.as_str(), "__getattr__" | "__dir__"),
        Stmt::If(ast::StmtIf { test, .. }) => is_type_checking(test, checker),
        _ => false,
    }
}

// RUF067 reports the statements of an `__init__.py` that this rule would report, so the statements are skipped
// whether or not the run selects it or preview mode is on. Ruff allows the imports, dunder assignments and
// `if TYPE_CHECKING` blocks of a package unless the project bans all code, and an attribute docstring, which follows
// an assignment, is documentation rather than code.
fn left_to_ruff(checker: &Checker, stmt: &Stmt, previous: Option<&Stmt>) -> bool {
    !is_attribute_docstring(stmt, previous)
        && (checker.settings.from_ruff.strictly_empty_init_modules || !is_allowed_by_default(stmt, checker))
}

fn is_attribute_docstring(stmt: &Stmt, previous: Option<&Stmt>) -> bool {
    matches!(stmt, Stmt::Expr(ast::StmtExpr { value, .. }) if value.is_string_literal_expr())
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
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().all(is_dunder_name),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            is_dunder_name(target)
        }
        _ => is_package_hook(stmt, checker),
    }
}

fn is_dunder_name(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| is_dunder(name.id.as_str()))
}

// F401 reports an import that nothing reads, so the import is skipped whether or not the run selects F401. The
// statement stays here when one of its names is not one F401 reports, such as a name that a re-export keeps.
fn left_to_unused_import(checker: &Checker, stmt: &Stmt, public: bool, facade: bool) -> bool {
    let mut uncovered = import_aliases(stmt)
        .iter()
        .filter(|alias| {
            !(matches!(stmt, Stmt::ImportFrom(_)) && public && is_reexported_alias(alias, checker.dunder_all(), facade))
        })
        .peekable();
    uncovered.peek().is_some() && uncovered.all(|alias| reported_by_unused_import(checker, stmt, alias))
}

// Whether F401 reports the import of `alias`, which the semantic model finds unused, unless the isort or pyflakes
// settings exempt it.
fn reported_by_unused_import(checker: &Checker, stmt: &Stmt, alias: &ast::Alias) -> bool {
    let qualified = match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, .. }) => {
            collect_import_from_member(*level, module.as_deref(), alias.name.as_str())
        }
        _ => QualifiedName::user_defined(alias.name.as_str()),
    };
    let from_ruff = &checker.settings.from_ruff;
    checker.semantic().is_unused_import(alias.identifier())
        && !from_ruff
            .required_imports
            .iter()
            .any(|required| required.bound_name() == alias_binding(alias) && required.qualified_name() == qualified)
        && !from_ruff
            .allowed_unused_imports
            .iter()
            .any(|allowed| qualified.starts_with(&QualifiedName::user_defined(allowed)))
}
