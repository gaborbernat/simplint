use ruff_diagnostics::Fix;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Import, ensure_import};
use crate::rules::logic::scoped_block::{finally_call, is_exported_module_name, wrap_edits};
use crate::rules::logic::sole_name_target;
use crate::syntax::{for_each_body, sole_argument};

// Since Python 3.11 `contextlib.chdir` enters a directory and returns to the one it left on exit.
pub fn up_chdir_restore(checker: &Checker) {
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
    let Some(old) = sole_name_target(assign) else {
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
    if !get.arguments.is_empty()
        || !semantic.resolves_to(&get.func, &["os", "getcwd"])
        || !semantic.resolves_to(&chdir.func, &["os", "chdir"])
        || !semantic.resolves_to(&reset.func, &["os", "chdir"])
        || !restores_old
    {
        return;
    }
    // The assignment and the `chdir` argument of the `finally` are the name's only spellings in its scope.
    if checker.identifier_count_in(checker.enclosing_scope(assign.range()), old.id.as_str()) != 2 {
        return;
    }
    let header = format!("with contextlib.chdir({}):", checker.text(target));
    let fix = ensure_import(checker, "contextlib", None, assign.range())
        .filter(|_| !is_exported_module_name(checker, old))
        .and_then(|import| {
            let [head, tail] = wrap_edits(checker, assign.range(), try_stmt, &header)?;
            let mut edits = vec![tail];
            if let Import::Add(edit) = import {
                edits.push(edit);
            }
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
