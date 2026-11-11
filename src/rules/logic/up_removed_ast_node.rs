use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

// The `ast` classes Python 3.14 removed, which were aliases of `ast.Constant` since Python 3.8.
const REMOVED: [&str; 5] = ["Num", "Str", "Bytes", "NameConstant", "Ellipsis"];

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Attribute(attribute) = expr else {
        return;
    };
    let checker = walker.checker;
    let Some(name) = checker.semantic().qualified_name(expr) else {
        return;
    };
    let Some(class) = (match name.segments() {
        ["ast", class] => REMOVED.iter().copied().find(|removed| removed == class),
        _ => None,
    }) else {
        return;
    };
    checker.report_fix(
        Rule::RemovedAstNode,
        expr.range(),
        format!("`ast.{class}` is removed in Python 3.14; use `ast.Constant`"),
        fix(walker, expr, attribute, class),
        "Use `ast.Constant`",
    );
}

// The fix covers the call the class is in: `Str(s=...)` builds a constant, and `isinstance(name, Str)` tests one.
fn fix(walker: &Walker, expr: &Expr, attribute: &ast::ExprAttribute, class: &str) -> Option<Fix> {
    let checker = walker.checker;
    let call = walker.expression_parent()?.as_call_expr()?;
    if checker.has_comment_in(call.range()) {
        return None;
    }
    let head = checker.text(attribute.value.range());
    if std::ptr::eq(call.func.as_ref(), expr) {
        let value = value_keyword(class)?;
        let [keyword] = call.arguments.keywords.as_slice() else {
            return None;
        };
        if !call.arguments.args.is_empty() || keyword.arg.as_ref()?.as_str() != value {
            return None;
        }
        let replacement = format!("{head}.Constant(value={})", checker.text(keyword.value.range()));
        return Some(Fix::safe_edit(Edit::range_replacement(replacement, call.range())));
    }
    // Only a name is repeated in the rewrite, so only a name is rewritten.
    let subject = call.arguments.args.first()?.as_name_expr()?;
    let kind = match class {
        "Str" => "str",
        "Bytes" => "bytes",
        _ => return None,
    };
    let is_test = call.func.as_name_expr().is_some_and(|name| name.id.as_str() == "isinstance");
    if !is_test
        || !checker.semantic().is_free_at("isinstance", call.range())
        || call.arguments.args.len() != 2
        || !call.arguments.keywords.is_empty()
        || !call.arguments.args.get(1).is_some_and(|argument| std::ptr::eq(argument, expr))
    {
        return None;
    }
    let test = format!("isinstance({0}, {head}.Constant) and isinstance({0}.value, {kind})", subject.id.as_str());
    // Inside a larger expression the two tests need their own parentheses to keep `and` apart from the rest.
    let replacement = if walker.expressions().len() > 2 { format!("({test})") } else { test };
    Some(Fix::safe_edit(Edit::range_replacement(replacement, call.range())))
}

// The keyword that holds the value of each class, which `ast.Constant` takes as `value`.
fn value_keyword(class: &str) -> Option<&'static str> {
    match class {
        "Num" => Some("n"),
        "Str" | "Bytes" => Some("s"),
        "NameConstant" => Some("value"),
        _ => None,
    }
}

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(function) = stmt else {
        return;
    };
    let Some(Scope::Class(class)) = walker.scopes().last() else {
        return;
    };
    let checker = walker.checker;
    let Some(name) = function.name.as_str().strip_prefix("visit_").filter(|name| REMOVED.contains(name)) else {
        return;
    };
    let semantic = checker.semantic();
    let visitor = class.arguments.as_deref().is_some_and(|arguments| {
        arguments.args.iter().any(|base| {
            semantic.resolves_to(base, &["ast", "NodeVisitor"])
                || semantic.resolves_to(base, &["ast", "NodeTransformer"])
        })
    });
    if visitor {
        checker.report(
            Rule::RemovedAstNode,
            function.name.range(),
            format!("`visit_{name}` is no longer called in Python 3.14; define `visit_Constant`"),
        );
    }
}
