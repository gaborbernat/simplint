use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::Visibility;

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
                check_forwarding(checker, function, &calls, None);
            }
            // Only positional bases open the class here: `metaclass=` and other class keywords add no caller of `self`.
            Stmt::ClassDef(class)
                if class.arguments.as_ref().is_none_or(|arguments| arguments.args.is_empty())
                    && !checker.project().is_base_class(class.name.as_str()) =>
            {
                for function in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    let calls = checker.semantic().member_references(class.range, function.name.as_str());
                    let receiver = function.parameters.posonlyargs.iter().chain(&function.parameters.args).next();
                    check_forwarding(checker, function, calls, receiver.map(|parameter| parameter.name().as_str()));
                }
            }
            _ => {}
        }
    }
}

fn check_forwarding(
    checker: &Checker,
    function: &ast::StmtFunctionDef,
    references: &[TextRange],
    receiver: Option<&str>,
) {
    if let Some(callee) = forward_target(checker, function, references, receiver) {
        let target = checker.text(callee.range());
        checker.report(
            Rule::ForwardingFunction,
            function.name.range(),
            format!("`{}` only forwards to `{target}`; call `{target}` at its call sites instead", function.name),
        );
    }
}

// The function a private wrapper only forwards to, when every reference to the wrapper calls it.
// single-caller-helper (SMP505) defers to this rule for such a wrapper, so both read this one definition.
pub(super) fn forward_target<'a>(
    checker: &Checker,
    function: &'a ast::StmtFunctionDef,
    references: &[TextRange],
    receiver: Option<&str>,
) -> Option<&'a ast::Expr> {
    let name = function.name.as_str();
    // A function other code patches by name is a seam the tests replace, so it must keep existing.
    if Visibility::of(name) != Visibility::Private
        || !function.decorator_list.is_empty()
        || references.is_empty()
        || checker.project().is_referenced(name)
    {
        return None;
    }
    // Defaults of its own make the wrapper configure the call, like an added `check=True`.
    if function.parameters.iter_non_variadic_params().any(|parameter| parameter.default.is_some()) {
        return None;
    }
    let [statement] = body_without_leading_docstring(&function.body) else {
        return None;
    };
    let (Stmt::Return(ast::StmtReturn { value: Some(value), .. }) | Stmt::Expr(ast::StmtExpr { value, .. })) =
        statement
    else {
        return None;
    };
    let call = value.as_call_expr()?;
    // A wrapper that adds its own values (`check=True`, a computed argument) configures the call, so it is not a pure
    // forward even when its body is a single call.
    let parameters: Vec<&str> = function.parameters.iter().map(|parameter| parameter.name().as_str()).collect();
    let is_parameter = |expr: &ast::Expr| {
        let expr = expr.as_starred_expr().map_or(expr, |starred| &starred.value);
        expr.as_name_expr().is_some_and(|name| parameters.contains(&name.id.as_str()))
    };
    // `make(x).strip()` is a call on a computed value, not a forward to a named function. `fn(x)` calls whatever the
    // caller passed in, so there is no fixed target to call instead. A method's receiver is the object it is called on,
    // so a target through its attributes, such as `self.client.get`, is a fixed one, while the receiver itself is the
    // method's own object and is not a target to call instead.
    if UnqualifiedName::from_expr(&call.func).as_ref().is_none_or(|target| {
        let segments = target.segments();
        segments
            .first()
            .is_some_and(|root| parameters.contains(root) && (Some(*root) != receiver || segments.len() == 1))
    }) || !call.arguments.args.iter().all(is_parameter)
        || !call.arguments.keywords.iter().all(|keyword| is_parameter(&keyword.value))
    {
        return None;
    }
    references.iter().all(|range| is_called(checker, *range)).then_some(&*call.func)
}

// Whether a reference is the callee of a call, wherever the call's `(` sits. A bare name that ends its statement is
// read there, even when the next line starts with `(`.
pub(super) fn is_called(checker: &Checker, reference: TextRange) -> bool {
    checker.state().call_ends(checker).contains(&reference.end())
}
