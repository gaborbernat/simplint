use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Arguments, Expr};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::testing::in_test_case;

pub fn expression(walker: &mut Walker, expr: &Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(negated) = boolean_assert(&call.func) else {
        return;
    };
    let Some(message) = message_argument(&call.arguments) else {
        return;
    };
    let Some(Expr::Call(inner)) = call.arguments.args.first() else {
        return;
    };
    let checker = walker.checker;
    if !in_test_case(checker, call.range()) {
        return;
    }
    let Some((positive, operands)) = helper_for(inner) else {
        return;
    };
    let name = if negated { format!("assertNot{}", &positive["assert".len()..]) } else { positive.to_owned() };
    let fix = (!checker.has_comment_in(call.range())).then(|| {
        let operands: Vec<&str> = operands.iter().map(|operand| checker.text(operand.range())).collect();
        let message = match message {
            Message::Absent => String::new(),
            Message::Given(message) => format!(", msg={}", checker.text(message.range())),
        };
        let replacement = format!("self.{name}({}{message})", operands.join(", "));
        Fix::safe_edit(Edit::range_replacement(replacement, call.range()))
    });
    checker.report_fix(
        Rule::UnittestAssertHelper,
        call.range(),
        format!("`self.{name}` reports both operands on failure, which the boolean assert hides"),
        fix,
        format!("Use `self.{name}`"),
    );
}

// Only `self` calls are the test case's own methods; another receiver may be a different `assertTrue`.
fn boolean_assert(func: &Expr) -> Option<bool> {
    let attribute = func.as_attribute_expr()?;
    if attribute.value.as_name_expr().is_none_or(|receiver| receiver.id != "self") {
        return None;
    }
    match attribute.attr.as_str() {
        "assertTrue" => Some(false),
        "assertFalse" => Some(true),
        _ => None,
    }
}

// The message of `assertTrue(expr, msg)` or `assertTrue(expr, msg=msg)`, the one argument beyond `expr` the helper
// takes. Any other keyword or unpacked argument is not ours to move.
enum Message<'a> {
    Absent,
    Given(&'a Expr),
}

fn message_argument(arguments: &Arguments) -> Option<Message<'_>> {
    if arguments.args.iter().any(Expr::is_starred_expr) {
        return None;
    }
    match (arguments.args.len(), arguments.keywords.as_slice()) {
        (1, []) => Some(Message::Absent),
        (2, []) => arguments.args.get(1).map(Message::Given),
        (1, [keyword]) if keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == "msg") => {
            Some(Message::Given(&keyword.value))
        }
        _ => None,
    }
}

// The helper that asserts what `call` checks, with the operands it takes. `startswith` and `endswith` take the
// receiver as the first operand; `hasattr` and `issubclass` are plain builtins with two arguments.
fn helper_for(call: &ast::ExprCall) -> Option<(&'static str, Vec<&Expr>)> {
    if !call.arguments.keywords.is_empty() || call.arguments.args.iter().any(Expr::is_starred_expr) {
        return None;
    }
    let args: Vec<&Expr> = call.arguments.args.iter().collect();
    match (call.func.as_ref(), args.as_slice()) {
        (Expr::Attribute(attribute), [prefix]) => {
            let helper = match attribute.attr.as_str() {
                "startswith" => "assertStartsWith",
                "endswith" => "assertEndsWith",
                _ => return None,
            };
            Some((helper, vec![attribute.value.as_ref(), *prefix]))
        }
        (Expr::Name(function), [object, other]) => {
            let helper = match function.id.as_str() {
                "hasattr" => "assertHasAttr",
                "issubclass" => "assertIsSubclass",
                _ => return None,
            };
            Some((helper, vec![*object, *other]))
        }
        _ => None,
    }
}
