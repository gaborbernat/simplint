use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::is_name;

pub fn unguarded_main_call(checker: &Checker) {
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
    if let Some(range) = main_call(expr) {
        checker.report(
            Rule::UnguardedMainCall,
            range,
            "Call `main()` under `if __name__ == \"__main__\":` so importing the module does not run it",
        );
    }
}

fn main_call(expr: &Expr) -> Option<TextRange> {
    let Expr::Call(call) = expr else {
        return None;
    };
    if is_name(&call.func, "main") {
        return Some(call.range());
    }
    call.arguments.args.iter().chain(call.arguments.keywords.iter().map(|keyword| &keyword.value)).find_map(main_call)
}
