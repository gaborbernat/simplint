use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::scoped_block::{finally_call, identifier_uses, scope_of, sole_argument, wrap_edits};
use crate::semantic::future_import;
use crate::syntax::for_each_body;

// Since Python 3.11 `contextlib.chdir` enters a directory and returns to the one it left on exit.
pub fn up_chdir_restore(checker: &Checker) {
    if checker.settings.from_ruff.target_version < PythonVersion::PY311 {
        return;
    }
    for_each_body(checker.body(), &mut |body, _| {
        for window in body.windows(3) {
            if let [Stmt::Assign(assign), Stmt::Expr(ast::StmtExpr { value, .. }), Stmt::Try(try_stmt)] = window {
                report_restore(checker, assign, value, try_stmt);
            }
        }
    });
}

fn report_restore(checker: &Checker, assign: &ast::StmtAssign, change: &Expr, try_stmt: &ast::StmtTry) {
    let semantic = checker.semantic();
    let [Expr::Name(old)] = assign.targets.as_slice() else {
        return;
    };
    let Expr::Call(get) = assign.value.as_ref() else {
        return;
    };
    let Expr::Call(chdir) = change else {
        return;
    };
    let Some(target) = sole_argument(chdir) else {
        return;
    };
    let Some(reset) = finally_call(try_stmt) else {
        return;
    };
    let restores_old =
        sole_argument(reset).is_some_and(|argument| matches!(argument, Expr::Name(name) if name.id == old.id));
    if !get.arguments.args.is_empty()
        || !get.arguments.keywords.is_empty()
        || !semantic.resolves_to(&get.func, &["os", "getcwd"])
        || !semantic.resolves_to(&chdir.func, &["os", "chdir"])
        || !semantic.resolves_to(&reset.func, &["os", "chdir"])
        || !restores_old
    {
        return;
    }
    // The assignment and the `chdir` argument of the `finally` are the name's only spellings in its scope.
    if identifier_uses(checker, scope_of(checker, assign.range()), old.id.as_str()) != 2 {
        return;
    }
    let header = format!("with contextlib.chdir({}):\n", checker.text(target));
    let fix = contextlib_import(checker).and_then(|imports| {
        let [head, tail] = wrap_edits(checker, assign.range(), try_stmt, &header)?;
        let mut edits = vec![tail];
        edits.extend(imports);
        Some(Fix::safe_edits(head, edits))
    });
    checker.report_fix(
        Rule::ChdirRestore,
        assign.range(),
        "Restoring the directory in a `finally` is what `contextlib.chdir` does",
        fix,
        "Enter the directory with `contextlib.chdir`",
    );
}

// The edits that make `contextlib` available: none when the module imports it, one inserting the import when the name
// is free, and nothing, which leaves the fix off, when another binding holds the name.
fn contextlib_import(checker: &Checker) -> Option<Vec<Edit>> {
    let body = checker.body();
    let imported = body.iter().any(|stmt| {
        matches!(stmt, Stmt::Import(ast::StmtImport { names, .. })
            if names.iter().any(|alias| alias.name.as_str() == "contextlib" && alias.asname.is_none()))
    });
    if imported {
        return Some(Vec::new());
    }
    let whole = TextRange::up_to(checker.source.text_len());
    if identifier_uses(checker, whole, "contextlib") > 0 {
        return None;
    }
    // The import goes before the first import the module has, after `from __future__` imports, which must come first.
    let first_import =
        body.iter().find(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)) && future_import(stmt).is_none());
    let edit = if let Some(stmt) = first_import {
        Edit::insertion("import contextlib\n".to_owned(), checker.source.line_start(stmt.start()))
    } else {
        let preamble = body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count();
        Edit::insertion("import contextlib\n\n".to_owned(), checker.source.line_start(body[preamble].start()))
    };
    Some(vec![edit])
}
