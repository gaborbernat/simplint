use ruff_python_ast::helpers::{body_without_leading_docstring, is_dunder};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn forwarding_function(checker: &Checker) {
    for stmt in checker.body() {
        match stmt {
            Stmt::FunctionDef(function) => {
                let calls: Vec<TextRange> = checker
                    .semantic()
                    .references_to(function.name.as_str())
                    .iter()
                    .map(|reference| reference.range)
                    .collect();
                check_forwarding(checker, function, &calls);
            }
            Stmt::ClassDef(class) if class.arguments.as_ref().is_none_or(|arguments| arguments.args.is_empty()) => {
                for function in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    let calls = checker.semantic().member_references(class.range, function.name.as_str());
                    check_forwarding(checker, function, calls);
                }
            }
            _ => {}
        }
    }
}

fn check_forwarding(checker: &Checker, function: &ast::StmtFunctionDef, references: &[TextRange]) {
    let name = function.name.as_str();
    // A function other code patches by name is a seam the tests replace, so it must keep existing.
    if !name.starts_with('_')
        || is_dunder(name)
        || !function.decorator_list.is_empty()
        || references.is_empty()
        || checker.project().is_referenced(name)
    {
        return;
    }
    // Defaults of its own make the wrapper configure the call, like an added `check=True`.
    if function.parameters.iter_non_variadic_params().any(|parameter| parameter.default.is_some()) {
        return;
    }
    let [statement] = body_without_leading_docstring(&function.body) else {
        return;
    };
    let (Stmt::Return(ast::StmtReturn { value: Some(value), .. }) | Stmt::Expr(ast::StmtExpr { value, .. })) =
        statement
    else {
        return;
    };
    let Some(call) = value.as_call_expr() else {
        return;
    };
    // A wrapper that adds its own values (`check=True`, a computed argument) configures the call, so it is not a pure
    // forward even when its body is a single call.
    let parameters: Vec<&str> = function.parameters.iter().map(|parameter| parameter.name().as_str()).collect();
    let is_parameter = |expr: &ast::Expr| {
        let expr = expr.as_starred_expr().map_or(expr, |starred| &starred.value);
        expr.as_name_expr().is_some_and(|name| parameters.contains(&name.id.as_str()))
    };
    // `make(x).strip()` is a call on a computed value, not a forward to a named function.
    let target = UnqualifiedName::from_expr(&call.func);
    // `fn(x)` calls whatever the caller passed in, so there is no fixed target to call instead.
    if target.as_ref().is_none_or(|target| target.segments().first().is_some_and(|root| parameters.contains(root)))
        || !call.arguments.args.iter().all(is_parameter)
        || !call.arguments.keywords.iter().all(|keyword| is_parameter(&keyword.value))
    {
        return;
    }
    if references.iter().all(|range| checker.source[range.end().to_usize()..].trim_start().starts_with('(')) {
        let target = checker.text(call.func.range());
        checker.report(
            Rule::ForwardingFunction,
            function.name.range(),
            format!("`{name}` only forwards to `{target}`; call `{target}` at its call sites instead"),
        );
    }
}
