use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::ExprCollector;
use crate::syntax::{for_each_body, is_name};

// The `str` methods that read the text of an exception to tell it apart from others.
const TEXT_METHODS: [&str; 5] = ["startswith", "endswith", "find", "index", "count"];

pub(crate) fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    let checker = walker.checker;
    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), body, .. }) = handler
    else {
        return;
    };
    // Tests may assert on the messages they expect.
    if checker.is_test_module() {
        return;
    }
    let exc = name.as_str();
    let mut tests = ExprCollector::new(|expr: &Expr| by_text(checker, exc, expr));
    tests.visit_body(body);
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Match(ast::StmtMatch { subject, .. }) = stmt
                && is_message(subject, exc)
            {
                tests.found.push(subject.range());
            }
        }
    });
    for range in tests.found {
        checker.report(
            Rule::ErrorMessageMatching,
            range,
            "Telling exceptions apart by their message breaks when the text changes; catch a more specific type",
        );
    }
}

// The range of a comparison, or of a `str` method or `re` function, that reads the text of the exception `exc`.
fn by_text(checker: &Checker, exc: &str, expr: &Expr) -> Option<TextRange> {
    let found = match expr {
        Expr::Compare(ast::ExprCompare { operands, .. }) => operands.iter().any(|operand| is_message(operand, exc)),
        Expr::Call(ast::ExprCall { func, arguments, .. }) => {
            let text_method = func.as_attribute_expr().is_some_and(|attribute| {
                TEXT_METHODS.contains(&attribute.attr.as_str()) && is_str_of(&attribute.value, exc)
            });
            let pattern_search = ["search", "match", "fullmatch"]
                .into_iter()
                .any(|function| checker.semantic().resolves_to(func, &["re", function]))
                && arguments.args.iter().any(|argument| is_str_of(argument, exc));
            text_method || pattern_search
        }
        _ => false,
    };
    found.then(|| expr.range())
}

// `str(exc)`, `exc.args`, `exc.args[N]` or `exc.message`: the text the exception carries.
fn is_message(expr: &Expr, exc: &str) -> bool {
    match expr {
        Expr::Call(_) => is_str_of(expr, exc),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            matches!(attr.as_str(), "args" | "message") && is_name(value, exc)
        }
        Expr::Subscript(ast::ExprSubscript { value, .. }) => value
            .as_attribute_expr()
            .is_some_and(|attribute| attribute.attr.as_str() == "args" && is_name(&attribute.value, exc)),
        _ => false,
    }
}

fn is_str_of(expr: &Expr, exc: &str) -> bool {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return false;
    };
    is_name(func, "str") && matches!(&*arguments.args, [argument] if is_name(argument, exc))
}
