use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::{Exports, all_binding, all_mutation};
use crate::semantic::future_import;

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
    for stmt in body_without_leading_docstring(checker.body()) {
        if !(facade && is_reexport(stmt, checker.dunder_all())) {
            checker.report(Rule::InitModuleContent, stmt.range(), message);
        }
    }
}

// An import whose every name the package exports, through `__all__`, a redundant alias or `import *`, the `__all__`
// that lists them, and `from __future__` imports, which change no behavior at run time.
fn is_reexport(stmt: &Stmt, exports: &Exports) -> bool {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            future_import(stmt).is_some()
                || names.iter().all(|alias| {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                    alias.name.as_str() == "*"
                        || alias.asname.as_ref().is_some_and(|asname| asname.as_str() == alias.name.as_str())
                        || matches!(exports, Exports::Listed(names) if names.contains(&bound))
                })
        }
        _ => all_binding(stmt).is_some() || all_mutation(stmt).is_some(),
    }
}
