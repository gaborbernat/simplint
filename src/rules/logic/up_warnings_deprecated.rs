use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::{Blanks, statement_removal};
use crate::syntax::argument;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let (body, name, kind) = match stmt {
        Stmt::FunctionDef(function) => (&function.body, &function.name, "function"),
        Stmt::ClassDef(class) => (&class.body, &class.name, "class"),
        _ => return,
    };
    let Some(statement @ Stmt::Expr(ast::StmtExpr { value, .. })) = body_without_leading_docstring(body).first() else {
        return;
    };
    let Expr::Call(call) = value.as_ref() else {
        return;
    };
    let checker = walker.checker;
    let semantic = checker.semantic();
    if !semantic.resolves_to(&call.func, &["warnings", "warn"]) {
        return;
    }
    let Some(message) = argument(call, 0, "message") else {
        return;
    };
    let Some(category) = argument(call, 1, "category") else {
        return;
    };
    if !semantic.is_builtin(category, "DeprecationWarning") {
        return;
    }
    let report = if kind == "class" {
        "`DeprecationWarning` is raised when the class is defined; `@warnings.deprecated` raises it when the class is \
         instantiated or subclassed"
    } else {
        "`DeprecationWarning` is raised on each call of the function; `@warnings.deprecated` marks it for \
         type checkers too"
    };
    checker.report_fix(
        Rule::WarningsDeprecated,
        call.range(),
        report,
        fix(checker, name, body, &Warning { statement, call, message }),
        "Mark it with `@warnings.deprecated`",
    );
}

// The decorator runs where the function is defined, so it takes a string literal on one line, which reads no name of
// the function's scope, and the module name the call reads `warn` through. It goes below other decorators, since
// `@warnings.deprecated` rejects the `property` or `classmethod` object they make. A warning with a comment keeps it,
// so it is reported only.
fn fix(checker: &Checker, name: &ast::Identifier, body: &[Stmt], warning: &Warning) -> Option<Fix> {
    let Warning { statement, call, message } = warning;
    let Expr::Attribute(func) = call.func.as_ref() else {
        return None;
    };
    let text = checker.text(message.range());
    let source = checker.source;
    if !message.is_string_literal_expr()
        || text.contains('\n')
        || checker.has_comment_in(source.full_lines_range(statement.range()))
        || !checker.is_alone_on_lines(statement.range())
    {
        return None;
    }
    // A body of only the warning keeps a statement, as the decorated function still has to run to its end.
    let removal = statement_removal(checker, statement.range(), body.len(), None, Blanks::Trailing)
        .expect("a statement alone on its lines is removable");
    let decorator = format!(
        "{}@{}.deprecated({text}){}",
        checker.indentation(name),
        checker.text(func.value.range()),
        checker.line_ending()
    );
    Some(Fix::unsafe_edits(Edit::insertion(decorator, source.line_start(name.start())), [removal]))
}

struct Warning<'a> {
    statement: &'a Stmt,
    call: &'a ast::ExprCall,
    message: &'a Expr,
}
