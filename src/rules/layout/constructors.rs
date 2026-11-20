use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use super::definition_time::bound_names;
use crate::syntax::{ReturnVisitor, first_positional, is_classmethod};

// Returning an instance through another constructor still counts as construction. A unique assignment outside
// branches identifies a returned local even when a branch later transforms it.
pub(super) fn alternative_constructors(class: &ast::StmtClassDef) -> FxHashSet<&str> {
    let mut factories = FxHashSet::default();
    let mut delegates: Vec<(&str, Vec<&str>)> = Vec::new();
    for stmt in &class.body {
        let Stmt::FunctionDef(function) = stmt else { continue };
        if !is_classmethod(function) {
            continue;
        }
        let Some(receiver) = first_positional(&function.parameters) else { continue };
        let mut returns = ReturnVisitor::default();
        returns.visit_body(&function.body);
        let bound_once = |name: &str| {
            let mut unconditional = function.body.iter().filter(|stmt| {
                !matches!(stmt, Stmt::If(_) | Stmt::For(_) | Stmt::While(_) | Stmt::Try(_) | Stmt::Match(_))
                    && bound_names(stmt).contains(&name)
            });
            match (unconditional.next(), unconditional.next()) {
                (Some(Stmt::Assign(ast::StmtAssign { targets, value, .. })), None)
                    if matches!(targets.as_slice(), [Expr::Name(_)]) =>
                {
                    Some(value.as_ref())
                }
                _ => None,
            }
        };
        let mut constructs = false;
        let mut targets = Vec::new();
        for value in returns.returns.iter().filter_map(|stmt| stmt.value.as_deref()) {
            let value = match value {
                Expr::Name(name) => match bound_once(name.id.as_str()) {
                    Some(bound) => bound,
                    None => continue,
                },
                other => other,
            };
            match receiver_call(value, receiver) {
                Some(ReceiverCall::Class) => constructs = true,
                Some(ReceiverCall::Method(target)) => targets.push(target),
                None => {}
            }
        }
        if constructs || function.returns.as_deref().is_some_and(|returns| names_class(returns, class)) {
            factories.insert(function.name.as_str());
        } else {
            delegates.push((function.name.as_str(), targets));
        }
    }
    // A delegation cycle with no known constructor has no factory to propagate.
    while let Some(position) =
        delegates.iter().position(|(_, targets)| targets.iter().any(|target| factories.contains(target)))
    {
        factories.insert(delegates.swap_remove(position).0);
    }
    factories
}

enum ReceiverCall<'a> {
    Class,
    Method(&'a str),
}

fn receiver_call<'a>(expr: &'a Expr, receiver: &str) -> Option<ReceiverCall<'a>> {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else { return None };
    match func.as_ref() {
        Expr::Name(name) if name.id.as_str() == receiver => Some(ReceiverCall::Class),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. })
            if value.as_name_expr().is_some_and(|name| name.id.as_str() == receiver) =>
        {
            Some(ReceiverCall::Method(attr.as_str()))
        }
        _ => None,
    }
}

fn names_class(annotation: &Expr, class: &ast::StmtClassDef) -> bool {
    match annotation {
        Expr::Name(name) => name.id.as_str() == "Self" || name.id.as_str() == class.name.as_str(),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            attr.as_str() == "Self"
                && value
                    .as_name_expr()
                    .is_some_and(|module| matches!(module.id.as_str(), "typing" | "typing_extensions"))
        }
        Expr::StringLiteral(literal) => literal.value.to_str() == class.name.as_str(),
        _ => false,
    }
}
