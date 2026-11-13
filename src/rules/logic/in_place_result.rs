use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Container {
    List,
    Dict,
    Set,
}

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    if !is_result_used(walker, expr) {
        return;
    }
    let checker = walker.checker;
    let Some(name) = in_place_call(checker, call) else {
        return;
    };
    checker.report(
        Rule::InPlaceResult,
        expr.range(),
        format!("`{name}` changes its object in place and returns `None`, so this value is always `None`"),
    );
}

// The value is stored, returned, passed on or compared. A lambda's value is often thrown away on purpose, as in
// `on_done=lambda value: bool(seen.append(value))`, and so is the return of a function whose whole body is
// `return calls.append(kwargs)`, a callback that records its calls.
fn is_result_used(walker: &Walker, expr: &Expr) -> bool {
    if walker.scopes().iter().any(|scope| matches!(scope, Scope::Lambda(_))) {
        return false;
    }
    match walker.expression_parent() {
        Some(Expr::Call(_) | Expr::Named(_) | Expr::Compare(_)) => true,
        Some(_) => false,
        None => match walker.statement() {
            Some(
                Stmt::Assign(ast::StmtAssign { value, .. })
                | Stmt::AnnAssign(ast::StmtAnnAssign { value: Some(value), .. }),
            ) => value.range() == expr.range(),
            Some(Stmt::Return(ast::StmtReturn { value: Some(value), .. })) => {
                value.range() == expr.range()
                    && !matches!(walker.scopes().last(), Some(Scope::Function(function)) if function.body.len() == 1)
            }
            _ => false,
        },
    }
}

fn in_place_call(checker: &Checker, call: &ast::ExprCall) -> Option<String> {
    if checker.semantic().resolves_to(&call.func, &["random", "shuffle"]) {
        return Some("random.shuffle()".to_owned());
    }
    let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = call.func.as_ref() else {
        return None;
    };
    let method = attr.as_str();
    let container = container(checker, value)?;
    let in_place = match container {
        Container::List => ["append", "extend", "insert", "remove", "reverse", "clear", "sort"].as_slice(),
        Container::Dict => ["update", "clear"].as_slice(),
        Container::Set => [
            "add",
            "update",
            "discard",
            "remove",
            "clear",
            "difference_update",
            "intersection_update",
            "symmetric_difference_update",
        ]
        .as_slice(),
    };
    in_place.contains(&method).then(|| format!("{}.{method}()", checker.text(value.as_ref())))
}

// What the receiver provably is: a display, a comprehension, a builtin constructor call, a name whose only binding is
// one of those, or a parameter annotated with the builtin type.
fn container(checker: &Checker, value: &Expr) -> Option<Container> {
    let semantic = checker.semantic();
    match value {
        Expr::List(_) | Expr::ListComp(_) => Some(Container::List),
        Expr::Dict(_) | Expr::DictComp(_) => Some(Container::Dict),
        Expr::Set(_) | Expr::SetComp(_) => Some(Container::Set),
        Expr::Call(ast::ExprCall { func, .. }) => builtin(checker, func),
        Expr::Name(name) => {
            if let Some(bound) = semantic.only_binding_value(name) {
                return container(checker, bound);
            }
            let bindings = semantic.resolved_bindings(name);
            let [binding] = bindings.as_slice() else {
                return None;
            };
            let annotation = semantic.parameter_annotation(binding)?;
            builtin(checker, map_subscript(annotation))
        }
        _ => None,
    }
}

fn builtin(checker: &Checker, expr: &Expr) -> Option<Container> {
    let semantic = checker.semantic();
    [("list", Container::List), ("dict", Container::Dict), ("set", Container::Set)]
        .into_iter()
        .find(|(name, _)| semantic.is_builtin(expr, name))
        .map(|(_, container)| container)
}
