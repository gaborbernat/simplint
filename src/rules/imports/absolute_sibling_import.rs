use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{for_each_body, is_main_guard};

pub fn absolute_sibling_import(checker: &Checker) {
    let Some((_, package)) = checker.module_path().and_then(<[String]>::split_last) else {
        return;
    };
    let current = package.join(".");
    // A module with a main guard may run as `python path/to/module.py`, where relative imports fail.
    let applicability =
        if checker.body().iter().any(is_main_guard) { Applicability::Unsafe } else { Applicability::Safe };
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements {
            if let Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) = stmt {
                let name = module.as_str();
                let relative = if name == current {
                    Some(".".to_owned())
                } else {
                    name.strip_prefix(&current).and_then(|rest| rest.strip_prefix('.')).map(|rest| format!(".{rest}"))
                };
                if let Some(relative) = relative {
                    let title = format!("Import from `{relative}`");
                    checker.report_fix(
                        Rule::AbsoluteSiblingImport,
                        module.range(),
                        format!("Import from the same package relatively: `from {relative} import ...`"),
                        Some(Fix::applicable_edit(Edit::range_replacement(relative, module.range()), applicability)),
                        title,
                    );
                }
            }
        }
    });
}
