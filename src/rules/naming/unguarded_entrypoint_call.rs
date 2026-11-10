use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::is_alone_on_lines;

pub fn unguarded_entrypoint_call(checker: &Checker) {
    if checker.path.file_name().is_some_and(|name| name == "__main__.py") {
        return;
    }
    let Some(stmt) = checker.body().last() else {
        return;
    };
    let expr = match stmt {
        Stmt::Expr(ast::StmtExpr { value, .. }) | Stmt::Raise(ast::StmtRaise { exc: Some(value), .. }) => {
            value.as_ref()
        }
        _ => return,
    };
    if let Some((range, name)) = entrypoint_call(expr) {
        let source = checker.source;
        let fix = (is_alone_on_lines(source, stmt.range()) && !source.contains_line_break(stmt.range())).then(|| {
            let lines = source.full_lines_range(stmt.range());
            let eol = checker.stylist.line_ending().as_str();
            let indent = checker.stylist.indentation().as_str();
            Fix::unsafe_edit(Edit::range_replacement(
                format!("if __name__ == \"__main__\":{eol}{indent}{}", &source[lines]),
                lines,
            ))
        });
        checker.report_fix(
            Rule::UnguardedEntrypointCall,
            range,
            format!("Call `{name}()` under `if __name__ == \"__main__\":` so importing the module does not run it"),
            fix,
            "Guard the entrypoint call",
        );
    }
}

fn entrypoint_call(expr: &Expr) -> Option<(TextRange, &str)> {
    let Expr::Call(call) = expr else {
        return None;
    };
    if let Some(name) =
        call.func.as_name_expr().map(|name| name.id.as_str()).filter(|name| matches!(*name, "main" | "run"))
    {
        return Some((call.range(), name));
    }
    call.arguments
        .args
        .iter()
        .chain(call.arguments.keywords.iter().map(|keyword| &keyword.value))
        .find_map(entrypoint_call)
}
