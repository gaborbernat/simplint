use std::cmp::Ordering;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::{Truthiness, any_over_expr, contains_effect};
use ruff_python_ast::{self as ast, BoolOp, CmpOp, Expr, Operator, Stmt, UnaryOp};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, statement_removal};
use crate::syntax::for_each_body;

pub fn constant_assert(checker: &Checker) {
    let is_builtin = |name: &str| !checker.semantic().is_bound(name);
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Assert(assert) = stmt
                && is_truthy_constant(&assert.test, is_builtin)
            {
                report_constant_assert(checker, stmt, assert, statements);
            }
        }
    });
}

fn is_truthy_constant(test: &Expr, is_builtin: impl Fn(&str) -> bool) -> bool {
    match Truthiness::from_expr(test, is_builtin) {
        Truthiness::True | Truthiness::Truthy => true,
        Truthiness::Unknown => formats_to_text(test) || evaluate(test).is_some_and(|value| value.is_truthy()),
        _ => false,
    }
}

fn report_constant_assert(checker: &Checker, stmt: &Stmt, assert: &ast::StmtAssert, statements: &[Stmt]) {
    let ast::StmtAssert { test, msg, .. } = assert;
    if matches!(test.as_ref(), Expr::Tuple(_)) && checker.settings.from_ruff.ruff_runs(RuffRule::AssertTuple)
        || matches!(test.as_ref(), Expr::StringLiteral(_) | Expr::BytesLiteral(_) | Expr::FString(_))
            && checker.settings.from_ruff.ruff_runs(RuffRule::AssertOnStringLiteral)
    {
        return;
    }
    let fix = match (test.as_ref(), msg) {
        // Rewriting or deleting the statement would drop a comment inside it.
        _ if checker.has_comment_in(stmt.range()) => None,
        // `assert (check, "message")` meant `assert check, "message"`; deleting it would drop the check.
        // A starred element has no meaning outside the tuple, so there is no single-argument form to restore.
        (Expr::Tuple(tuple @ ast::ExprTuple { elts, .. }), None)
            if elts.len() == 2 && !elts.iter().any(Expr::is_starred_expr) =>
        {
            // Out of the tuple's brackets, a walrus or a multi-line element needs parentheses of its own.
            let [check, message] = [&elts[0], &elts[1]].map(|elt| {
                let text = checker.parenthesized_text(elt, tuple.into());
                if (text.contains('\n') || matches!(elt, Expr::Named(_))) && text == checker.text(elt.range()) {
                    format!("({text})")
                } else {
                    text.to_owned()
                }
            });
            let replacement = format!("assert {check}, {message}");
            Some(Fix::unsafe_edit(Edit::range_replacement(replacement, stmt.range())))
        }
        (Expr::Tuple(_), _) => None,
        // `assert (x for x in xs)` meant `assert all(x for x in xs)`: the generator is always truthy, and its first
        // iterable is evaluated when it is built, so deleting it could hide an error.
        // An `async for` or an `await` makes it an asynchronous generator, which `all()` cannot iterate, and a
        // rebound `all` is not the builtin.
        (Expr::Generator(_), _) => (!any_over_expr(test, |expr| match expr {
            Expr::Generator(generator) => generator.generators.iter().any(|comprehension| comprehension.is_async),
            _ => expr.is_await_expr(),
        }) && !checker.semantic().is_bound("all"))
        .then(|| {
            // The generator's range includes the parentheses that an assert test must have, so they become the call's.
            let test = test.as_ref();
            Fix::unsafe_edit(Edit::range_replacement(format!("all{}", checker.text(test)), test.range()))
        }),
        _ => is_deletable(checker, stmt, test)
            .then(|| statement_removal(checker, stmt.range(), statements.len(), None, Blanks::Keep))
            .flatten()
            .map(|edit| {
                // A template raises when its fields do not fit its arguments, and deleting it would hide that error.
                Fix::applicable_edit(
                    edit,
                    if formats_to_text(test) { Applicability::Unsafe } else { Applicability::Safe },
                )
            }),
    };
    let title = match test.as_ref() {
        Expr::Tuple(_) => "Split the tuple into a test and a message",
        Expr::Generator(_) => "Check every item with `all()`",
        _ => "Remove the assertion",
    };
    checker.report_fix(
        Rule::ConstantAssert,
        stmt.range(),
        "Assertion on a constant always passes; assert on the result under test",
        fix,
        title,
    );
}

fn is_deletable(checker: &Checker, stmt: &Stmt, test: &Expr) -> bool {
    checker.is_alone_on_lines(stmt.range()) && !has_effect(checker, test)
}

// A computed value and a format or `%` template read only literals, so only the arguments of such a template can
// have an effect.
fn has_effect(checker: &Checker, test: &Expr) -> bool {
    let is_builtin = |name: &str| !checker.semantic().is_bound(name);
    match test {
        Expr::Call(ast::ExprCall { arguments, .. }) if formats_to_text(test) => arguments
            .args
            .iter()
            .chain(arguments.keywords.iter().map(|keyword| &keyword.value))
            .any(|argument| contains_effect(argument, is_builtin)),
        Expr::BinOp(ast::ExprBinOp { right, .. }) if formats_to_text(test) => contains_effect(right, is_builtin),
        _ if evaluate(test).is_some() => false,
        _ => contains_effect(test, is_builtin),
    }
}

// A `"...".format(...)` or `"..." % ...` whose template has text outside its fields, so the result is never empty
// whatever the fields hold.
fn formats_to_text(expr: &Expr) -> bool {
    match expr {
        Expr::Call(ast::ExprCall { func, .. }) => match func.as_ref() {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr == "format" => {
                string_literal(value).is_some_and(has_format_text)
            }
            _ => false,
        },
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::Mod, .. }) => {
            string_literal(left).is_some_and(has_percent_text)
        }
        _ => false,
    }
}

fn string_literal(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::StringLiteral(string) => Some(string.value.to_str()),
        _ => None,
    }
}

// `{{` and `}}` stand for braces, and any other character outside a `{...}` field is copied as written. A malformed
// template raises instead of passing, so it has no text this rule can rely on.
fn has_format_text(template: &str) -> bool {
    let mut has_text = false;
    let mut depth = 0_usize;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' if depth == 0 && chars.next_if_eq(&'{').is_some() => has_text = true,
            '{' => depth += 1,
            '}' if depth == 0 => {
                if chars.next_if_eq(&'}').is_none() {
                    return false;
                }
                has_text = true;
            }
            '}' => depth -= 1,
            _ if depth == 0 => has_text = true,
            _ => {}
        }
    }
    depth == 0 && has_text
}

// `%%` stands for a percent sign. A directive is `%`, an optional `(key)`, flags, width and precision, and a
// conversion, which can produce nothing; any other character is copied as written.
fn has_percent_text(template: &str) -> bool {
    let mut has_text = false;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' || chars.next_if_eq(&'%').is_some() {
            has_text = true;
            continue;
        }
        if chars.next_if_eq(&'(').is_some() && !chars.any(|key_char| key_char == ')') {
            return false;
        }
        while chars.next_if(|flag| "#0-+ .*hlL123456789".contains(*flag)).is_some() {}
        if !chars.next().is_some_and(|conversion| "diouxXeEfFgGcrsa".contains(conversion)) {
            return false;
        }
    }
    has_text
}

// The value of a computation over literals, enough to tell whether an assertion on it always passes.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Bool(bool),
    Int(i64),
    Str(String),
    None,
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Int(value) => *value != 0,
            Self::Str(value) => !value.is_empty(),
            Self::None => false,
        }
    }

    // `bool` is an `int` in arithmetic and comparisons.
    fn int(&self) -> Option<i64> {
        match self {
            Self::Bool(value) => Some(i64::from(*value)),
            Self::Int(value) => Some(*value),
            Self::Str(_) | Self::None => None,
        }
    }
}

// Longest string a repetition or concatenation builds; a longer one is not evaluated, so the rule never allocates a
// large string.
const MAX_STR_LEN: usize = 4096;

// Only literals, operators on them and `not` are evaluated; a name or a call is unknown, as its value can change.
fn evaluate(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::BooleanLiteral(ast::ExprBooleanLiteral { value, .. }) => Some(Value::Bool(*value)),
        Expr::NoneLiteral(_) => Some(Value::None),
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(int), .. }) => {
            int.as_i64().map(Value::Int)
        }
        Expr::StringLiteral(string) => Some(Value::Str(string.value.to_str().to_owned())),
        Expr::UnaryOp(ast::ExprUnaryOp { op, operand, .. }) => {
            let value = evaluate(operand)?;
            match op {
                UnaryOp::Not => Some(Value::Bool(!value.is_truthy())),
                UnaryOp::USub => value.int()?.checked_neg().map(Value::Int),
                _ => None,
            }
        }
        Expr::BinOp(ast::ExprBinOp { left, op, right, .. }) => binary(evaluate(left)?, *op, evaluate(right)?),
        Expr::BoolOp(ast::ExprBoolOp { op, values, .. }) => {
            let mut last = None;
            for value in values {
                let value = evaluate(value)?;
                let decides = match op {
                    BoolOp::And => !value.is_truthy(),
                    BoolOp::Or => value.is_truthy(),
                };
                if decides {
                    return Some(value);
                }
                last = Some(value);
            }
            last
        }
        Expr::Compare(ast::ExprCompare { ops, operands, .. }) => {
            let mut previous = evaluate(operands.first()?)?;
            let mut all_hold = true;
            for (op, operand) in ops.iter().zip(&operands[1..]) {
                let next = evaluate(operand)?;
                all_hold &= compare(&previous, *op, &next)?;
                previous = next;
            }
            Some(Value::Bool(all_hold))
        }
        _ => None,
    }
}

fn binary(left: Value, op: Operator, right: Value) -> Option<Value> {
    match (op, left, right) {
        (Operator::Add, Value::Str(left), Value::Str(right)) => {
            (left.len().saturating_add(right.len()) <= MAX_STR_LEN).then(|| Value::Str(left + &right))
        }
        (Operator::Mult, Value::Str(text), count) | (Operator::Mult, count, Value::Str(text)) => {
            let count = usize::try_from(count.int()?).unwrap_or(0);
            (text.len().saturating_mul(count) <= MAX_STR_LEN).then(|| Value::Str(text.repeat(count)))
        }
        (op, left, right) => {
            let (left, right) = (left.int()?, right.int()?);
            match op {
                Operator::Add => left.checked_add(right),
                Operator::Sub => left.checked_sub(right),
                Operator::Mult => left.checked_mul(right),
                _ => None,
            }
            .map(Value::Int)
        }
    }
}

// Python orders only operands of one type, while `==` and `!=` accept any two values, which are equal only when
// they are the same value.
fn compare(left: &Value, op: CmpOp, right: &Value) -> Option<bool> {
    let ordering = match (left, right) {
        (Value::Str(left), Value::Str(right)) => left.cmp(right),
        (Value::None, Value::None) => return equality(op, true),
        _ => match (left.int(), right.int()) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => return equality(op, false),
        },
    };
    match op {
        CmpOp::Eq => Some(ordering == Ordering::Equal),
        CmpOp::NotEq => Some(ordering != Ordering::Equal),
        CmpOp::Lt => Some(ordering == Ordering::Less),
        CmpOp::LtE => Some(ordering != Ordering::Greater),
        CmpOp::Gt => Some(ordering == Ordering::Greater),
        CmpOp::GtE => Some(ordering != Ordering::Less),
        _ => None,
    }
}

fn equality(op: CmpOp, equal: bool) -> Option<bool> {
    match op {
        CmpOp::Eq => Some(equal),
        CmpOp::NotEq => Some(!equal),
        _ => None,
    }
}
