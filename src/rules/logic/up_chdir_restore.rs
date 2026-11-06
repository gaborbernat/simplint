use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::ensure_import;
use crate::rules::logic::scoped_block::{finally_call, identifier_uses, scope_of, sole_argument, wrap_edits};
use crate::syntax::for_each_body;

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
    let (_, edit) = ensure_import(checker, "contextlib", None)?;
    Some(edit.into_iter().collect())
}
