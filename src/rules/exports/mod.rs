mod all_format;
mod all_not_last;
mod missing_from_all;

pub(crate) use all_format::all_format;
pub(crate) use all_not_last::all_not_last;
pub(crate) use missing_from_all::missing_from_all;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::rules::for_each_body;

pub enum Exports<'a> {
    Absent,
    Listed(Vec<&'a str>),
    Dynamic,
}

pub fn dunder_all(body: &[Stmt]) -> Exports<'_> {
    // An `__all__` bound inside an `if` or `try` depends on how the module runs, so its contents are unknown.
    let mut nested = false;
    for_each_body(body, &mut |statements, owner| {
        nested |= owner.is_some() && statements.iter().any(|stmt| all_binding(stmt).is_some());
    });
    if nested {
        return Exports::Dynamic;
    }
    // `from ._impl import __all__` re-exports a list this module cannot see.
    let imported = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str() == "__all__")
        }
        _ => false,
    });
    if imported {
        return Exports::Dynamic;
    }
    let mut exports = Exports::Absent;
    for stmt in body {
        let (resets, elements): (bool, Option<Vec<&Expr>>) = if let Some(binding) = all_binding(stmt) {
            (binding.operator.is_none(), all_literal(stmt).map(|(_, elts)| elts.iter().collect()))
        } else {
            match stmt {
                Stmt::Expr(ast::StmtExpr { value, .. }) => match value.as_ref() {
                    Expr::Call(ast::ExprCall { func, arguments, .. })
                        if func.as_attribute_expr().is_some_and(|attribute| is_all_target(&attribute.value)) =>
                    {
                        let method = func.as_attribute_expr().map(|attribute| attribute.attr.as_str());
                        let elements = match (method, &*arguments.args) {
                            (Some("append"), [element]) => Some(vec![element]),
                            (
                                Some("extend"),
                                [Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })],
                            ) => Some(elts.iter().collect()),
                            _ => None,
                        };
                        (false, elements)
                    }
                    _ => continue,
                },
                _ => continue,
            }
        };
        let names: Option<Vec<&str>> = elements.and_then(|elements| {
            elements.iter().map(|element| Some(element.as_string_literal_expr()?.value.to_str())).collect()
        });
        exports = match (exports, names) {
            (_, None) | (Exports::Dynamic, _) => Exports::Dynamic,
            (Exports::Listed(mut listed), Some(names)) if !resets => {
                listed.extend(names);
                Exports::Listed(listed)
            }
            (_, Some(names)) => Exports::Listed(names),
        };
    }
    exports
}

pub struct AllBinding<'a> {
    pub value: Option<&'a Expr>,
    // Set for `__all__ += ...`, which extends the list instead of replacing it.
    pub operator: Option<ast::Operator>,
}

pub fn all_binding(stmt: &Stmt) -> Option<AllBinding<'_>> {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.iter().any(is_all_target) => {
            Some(AllBinding { value: Some(value), operator: None })
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: value.as_deref(), operator: None })
        }
        Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: Some(value), operator: Some(*op) })
        }
        _ => None,
    }
}

pub fn all_literal(stmt: &Stmt) -> Option<(TextRange, &[Expr])> {
    // In `alias = __all__ = [...]` the list has a second name that can change it out of sight.
    if let Stmt::Assign(ast::StmtAssign { targets, .. }) = stmt
        && !targets.first().is_some_and(is_all_target)
    {
        return None;
    }
    match all_binding(stmt)? {
        AllBinding {
            value: Some(value @ (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }))),
            operator: None | Some(ast::Operator::Add),
        } => Some((value.range(), elts)),
        _ => None,
    }
}

pub fn is_all_target(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| name.id.as_str() == "__all__")
}

// The one-name-per-line layout `ruff format` gives a list with a trailing comma, so the fix needs no formatter pass.
fn exploded(checker: &Checker, base: &str, items: &[String]) -> String {
    let indent = checker.stylist.indentation().as_str();
    let lines = items.iter().fold(String::new(), |text, item| text + base + indent + item + ",\n");
    format!("[\n{lines}{base}]")
}
