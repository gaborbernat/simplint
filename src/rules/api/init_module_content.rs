use ruff_python_ast::helpers::{body_without_leading_docstring, collect_import_from_member, is_dunder};
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::is_type_checking;
use crate::semantic::future_import;
use crate::syntax::{Exports, alias_binding, all_binding, builds_all, import_aliases, is_name};

// Future imports select language features for this file. Python and packaging tools also read some initializer globals.
pub fn init_module_content(checker: &Checker) {
    if !checker.is_init_module() || checker.is_test_module() || checker.settings.project.nested {
        return;
    }
    // A literal export list makes re-exports explicit in any package. Public library packages can also use star imports
    // and redundant aliases.
    let facade = checker.project().role().public;
    let explicit_api = matches!(checker.dunder_all(), Exports::Listed(_))
        && checker.body().iter().any(|stmt| all_binding(stmt).is_some_and(|binding| binding.operator.is_none()));
    let public = facade || explicit_api;
    let message = if public {
        "`__init__.py` holds code; keep only its docstring and the re-exports of its public API"
    } else {
        "`__init__.py` holds code; keep only its docstring"
    };
    let readers = &checker.settings.project.version_readers;
    let version_read = readers.any_init || readers.files.iter().any(|file| file.as_path() == checker.path);
    let body = body_without_leading_docstring(checker.body());
    for (index, stmt) in body.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| &body[previous]);
        let reexport = public && is_reexport(stmt, checker.dunder_all(), facade);
        if !reexport
            && future_import(stmt).is_none()
            && !is_lazy_modules_binding(stmt)
            && (!version_read || !is_version_binding(stmt))
            && !is_package_hook(stmt, checker)
            && !left_to_ruff(checker, stmt, previous)
            && !left_to_unused_import(checker, stmt, public, facade)
        {
            checker.report(Rule::InitModuleContent, stmt.range(), message);
        }
    }
}

fn is_reexport(stmt: &Stmt, exports: &Exports, facade: bool) -> bool {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().all(|alias| is_reexported_alias(alias, exports, facade))
        }
        _ => builds_all(stmt),
    }
}

fn is_version_binding(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().any(|target| is_name(target, "__version__")),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => is_name(target, "__version__"),
        _ => false,
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

// Leave statements covered by RUF067 to Ruff even when the rule is disabled. Ruff allows an attribute docstring
// after an assignment, so simplint can still report the docstring.
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

// Skip an import when F401 covers every name that is not a re-export. A mixed import may still need a report.
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
