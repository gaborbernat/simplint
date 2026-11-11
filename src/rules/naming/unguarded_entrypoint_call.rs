use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::naming::script_targets;
use crate::syntax::is_name;

pub fn unguarded_entrypoint_call(checker: &Checker) {
    let Some(stmt) = checker.body().last() else {
        return;
    };
    let call = match stmt {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_call_expr(),
        Stmt::Raise(ast::StmtRaise { exc: Some(value), .. }) => {
            value.as_call_expr().filter(|call| is_name(&call.func, "SystemExit"))
        }
        _ => return,
    };
    let Some(call) = call else {
        return;
    };
    if !script_file(checker) && !calls_script_target(call, &script_targets(checker)) {
        return;
    }
    let source = checker.source;
    let fix = (checker.is_alone_on_lines(stmt.range()) && !source.contains_line_break(stmt.range())).then(|| {
        let lines = source.full_lines_range(stmt.range());
        let eol = checker.line_ending();
        let indent = checker.stylist.indentation().as_str();
        Fix::unsafe_edit(Edit::range_replacement(
            format!("if __name__ == \"__main__\":{eol}{indent}{}", &source[lines]),
            lines,
        ))
    });
    checker.report_fix(
        Rule::UnguardedEntrypointCall,
        call.range(),
        "Call the entrypoint under `if __name__ == \"__main__\":` so importing the module does not run it",
        fix,
        "Guard the entrypoint call",
    );
}

fn script_file(checker: &Checker) -> bool {
    if checker.file_name() == "__main__.py" || checker.source.starts_with("#!") {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        checker.path.metadata().is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn calls_script_target(call: &ast::ExprCall, targets: &[&str]) -> bool {
    if UnqualifiedName::from_expr(&call.func)
        .is_some_and(|name| targets.iter().any(|target| name.segments().iter().copied().eq(target.split('.'))))
    {
        return true;
    }
    call.arguments
        .args
        .iter()
        .chain(call.arguments.keywords.iter().map(|keyword| &keyword.value))
        .filter_map(Expr::as_call_expr)
        .any(|call| calls_script_target(call, targets))
}
