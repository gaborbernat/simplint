use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::Semantic;
use crate::syntax::last_segment;

pub fn sleep_in_test(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let mut finder = Sleeps { checker, calls: Vec::new(), patches_sleep: false };
    finder.visit_body(checker.body());
    // A test that patches `time.sleep` already controls the wait, so its sleeps are not the problem.
    if finder.patches_sleep {
        return;
    }
    for range in finder.calls {
        checker.report(
            Rule::SleepInTest,
            range,
            "Sleeping makes the test slow and flaky; wait on an event the code sets instead",
        );
    }
}

struct Sleeps<'a, 'b> {
    checker: &'a Checker<'b>,
    calls: Vec<TextRange>,
    patches_sleep: bool,
}

impl<'a> Visitor<'a> for Sleeps<'_, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            if is_sleep(self.checker, call) {
                self.calls.push(expr.range());
            }
            self.patches_sleep |= patches_sleep(self.checker.semantic(), call);
        }
        visitor::walk_expr(self, expr);
    }
}

// `sleep(0)` only yields to the event loop, so it does not wait.
fn is_sleep(checker: &Checker, call: &ast::ExprCall) -> bool {
    let argument = if checker.semantic().resolves_to(&call.func, &["time", "sleep"]) {
        "secs"
    } else if checker.semantic().resolves_to(&call.func, &["asyncio", "sleep"]) {
        "delay"
    } else {
        return false;
    };
    call.arguments.find_argument_value(argument, 0).is_none_or(|value| checker.text(value) != "0")
}

fn patches_sleep(semantic: &Semantic, call: &ast::ExprCall) -> bool {
    let monkeypatch_setattr = UnqualifiedName::from_expr(&call.func)
        .is_some_and(|name| matches!(name.segments(), [.., "monkeypatch", "setattr"]));
    let args = call.arguments.args.as_ref();
    if (last_segment(&call.func) == Some("patch") || monkeypatch_setattr)
        && args.iter().any(|arg| is_string(arg, "time.sleep"))
    {
        return true;
    }
    monkeypatch_setattr
        && matches!(args, [target, name, ..] if semantic.resolves_to(target, &["time"]) && is_string(name, "sleep"))
}

fn is_string(expr: &Expr, value: &str) -> bool {
    matches!(expr, Expr::StringLiteral(literal) if literal.value.to_str() == value)
}
