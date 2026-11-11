use std::borrow::Cow;

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::recognize::is_pytest_raises;
use crate::semantic::Semantic;
use crate::syntax::ReturnVisitor;

pub fn parametrize_without_id(checker: &Checker) {
    // A list a module shares by name is spread by several decorators, so each of its rows is reported once.
    let mut reported = FxHashSet::default();
    for function in checker.function_defs() {
        for decorator in &function.decorator_list {
            let Expr::Call(ast::ExprCall { func, arguments, .. }) = &decorator.expression else {
                continue;
            };
            if !checker.semantic().resolves_to(func, &["pytest", "mark", "parametrize"]) {
                continue;
            }
            let Some(values) = arguments.find_argument_value("argvalues", 1) else {
                continue;
            };
            if arguments.find_argument_value("ids", 3).is_some() {
                continue;
            }
            let (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = values else {
                continue;
            };
            let rows = rows(checker, elts);
            let mut id_counts = FxHashMap::default();
            for id in rows.iter().filter_map(|row| scalar_id(checker, row)) {
                *id_counts.entry(id).or_insert(0) += 1;
            }
            for row in rows.iter().filter(|row| !is_param_with_id(checker.semantic(), row)) {
                if scalar_id(checker, row).is_some_and(|id| id_counts.get(&id) == Some(&1))
                    || !reported.insert(row.range())
                {
                    continue;
                }
                checker.report(
                    Rule::ParametrizeWithoutId,
                    row.range(),
                    "Parametrize row has no id; wrap it in `pytest.param(..., id=...)`",
                );
            }
        }
    }
}

// The rows a list holds: its elements, and the elements of a starred list or tuple, written inline or a name bound once
// to one. Any other starred value, such as a generator, an imported name or a rebound one, has no rows this rule can
// read, and neither has a starred element inside such a list.
fn rows<'e>(checker: &'e Checker, elements: &'e [Expr]) -> Vec<&'e Expr> {
    let mut rows = Vec::new();
    for element in elements {
        let Expr::Starred(ast::ExprStarred { value, .. }) = element else {
            rows.extend(row(checker, element));
            continue;
        };
        let literal = match value.as_ref() {
            Expr::Name(name) => checker.semantic().only_binding_value(name),
            other => Some(other),
        };
        let Some(Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = literal else {
            continue;
        };
        rows.extend(elts.iter().filter(|elt| !elt.is_starred_expr()).filter_map(|elt| row(checker, elt)));
    }
    rows
}

// A name row is judged by every value its bindings give it. A row a binding gives an id, an import, or a value the rule
// cannot read leaves to whoever knows it. A name bound once to a `pytest.param(...)` call without an id is checked by
// that call, and any other name is reported where it is written. A call to a helper whose every path returns a row with
// an id is a row with an id, and any other call is reported where it is written.
fn row<'e>(checker: &'e Checker, element: &'e Expr) -> Option<&'e Expr> {
    let semantic = checker.semantic();
    let Expr::Name(name) = element else {
        return (!is_helper_with_ids(semantic, element)).then_some(element);
    };
    let bindings = semantic.resolved_bindings(name);
    let values: Vec<&Expr> = bindings
        .iter()
        .map(|binding| semantic.binding_values(binding))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    if values.iter().any(|value| is_param_with_id(semantic, value) || is_helper_with_ids(semantic, value)) {
        return None;
    }
    match (bindings.as_slice(), values.as_slice()) {
        ([_], [value]) if is_param(semantic, value) => Some(*value),
        _ => Some(element),
    }
}

// A helper returns its rows, so a call to a function the module defines once counts as a row with an id when every path
// ends in a `return` of `pytest.param(..., id=...)`. A coroutine or a generator returns no row, and a decorator may
// replace the function, so only the caches, which return the function's own result, keep the rule's view of it.
fn is_helper_with_ids(semantic: &Semantic, element: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = element else {
        return false;
    };
    let Expr::Name(name) = func.as_ref() else {
        return false;
    };
    let Some(function) = semantic.module_function(name) else {
        return false;
    };
    if function.is_async
        || !function.decorator_list.iter().all(|decorator| is_cache(semantic, decorator))
        || !always_returns(semantic, &function.body)
    {
        return false;
    }
    let mut returns = ReturnVisitor::default();
    returns.visit_body(&function.body);
    !returns.is_generator
        && returns
            .returns
            .iter()
            .all(|stmt| stmt.value.as_deref().is_some_and(|value| is_param_with_id(semantic, value)))
}

fn is_cache(semantic: &Semantic, decorator: &ast::Decorator) -> bool {
    let expression = match &decorator.expression {
        Expr::Call(ast::ExprCall { func, .. }) => func.as_ref(),
        other => other,
    };
    semantic.resolves_to(expression, &["functools", "cache"])
        || semantic.resolves_to(expression, &["functools", "lru_cache"])
}

// Only the last statement decides. A loop never counts, as it may run no times, and a `with` whose manager may swallow
// an exception may fall through after its body returns.
fn always_returns(semantic: &Semantic, body: &[Stmt]) -> bool {
    let Some(last) = body.last() else {
        return false;
    };
    match last {
        Stmt::Return(_) | Stmt::Raise(_) => true,
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            elif_else_clauses.last().is_some_and(|clause| clause.test.is_none())
                && always_returns(semantic, body)
                && elif_else_clauses.iter().all(|clause| always_returns(semantic, &clause.body))
        }
        Stmt::Match(ast::StmtMatch { cases, .. }) => {
            cases.last().is_some_and(|case| case.guard.is_none() && is_irrefutable(&case.pattern))
                && cases.iter().all(|case| always_returns(semantic, &case.body))
        }
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
            always_returns(semantic, finalbody)
                || (handlers
                    .iter()
                    .all(|ast::ExceptHandler::ExceptHandler(handler)| always_returns(semantic, &handler.body))
                    && always_returns(semantic, if orelse.is_empty() { body } else { orelse }))
        }
        Stmt::With(ast::StmtWith { items, body, .. }) => {
            always_returns(semantic, body) && !items.iter().any(|item| can_swallow(semantic, &item.context_expr))
        }
        _ => false,
    }
}

fn is_irrefutable(pattern: &ast::Pattern) -> bool {
    matches!(pattern, ast::Pattern::MatchAs(ast::PatternMatchAs { pattern: None, .. }))
}

fn can_swallow(semantic: &Semantic, expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return true;
    };
    semantic.resolves_to(func, &["contextlib", "suppress"])
        || is_pytest_raises(semantic, func)
        || semantic.resolves_to(func, &["contextlib", "ExitStack"])
        || semantic.resolves_to(func, &["contextlib", "AsyncExitStack"])
}

fn is_param(semantic: &Semantic, element: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = element else {
        return false;
    };
    semantic.resolves_to(func, &["pytest", "param"])
}

fn is_param_with_id(semantic: &Semantic, element: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { arguments, .. }) = element else {
        return false;
    };
    is_param(semantic, element)
        && arguments.find_keyword("id").is_some_and(|keyword| !keyword.value.is_none_literal_expr())
}

// Pytest uses a scalar's value as its id, but escapes non-printable ASCII and backslashes in strings, prints a number
// as Python does, so `0x10` and `16` share an id, and suffixes ids that several rows share.
fn scalar_id<'e>(checker: &Checker, element: &'e Expr) -> Option<Cow<'e, str>> {
    match element {
        Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
            let text = value.to_str();
            text.bytes().all(|byte| matches!(byte, b' '..=b'~') && byte != b'\\').then_some(Cow::Borrowed(text))
        }
        Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) => Some(Cow::Owned(checker.text(element).to_owned())),
        Expr::NumberLiteral(number) => number_id(checker, number).map(Cow::Owned),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub, operand, .. }) => {
            let id = number_id(checker, operand.as_number_literal_expr()?)?;
            Some(Cow::Owned(if id == "0" { id } else { format!("-{id}") }))
        }
        _ => None,
    }
}

// The id pytest gives a number: an integer in decimal, a float as Python's `repr` prints it, and a complex or an
// integer too large for `u64` as its source text.
fn number_id(checker: &Checker, number: &ast::ExprNumberLiteral) -> Option<String> {
    match &number.value {
        ast::Number::Int(int) => Some(int.as_u64().map_or_else(|| checker.text(number).to_owned(), |v| v.to_string())),
        ast::Number::Float(float) => float_id(*float),
        ast::Number::Complex { .. } => Some(checker.text(number).to_owned()),
    }
}

// Python's `repr` of a float: the shortest digits that read back as the same value, in fixed notation from 1e-4 up to
// 1e16 and in exponent notation beyond.
fn float_id(value: f64) -> Option<String> {
    if value.is_infinite() {
        return Some("inf".to_owned());
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e')?;
    let exponent: i32 = exponent.parse().ok()?;
    let digits: String = mantissa.chars().filter(|digit| *digit != '.').collect();
    let point = exponent + 1;
    if !(-4..16).contains(&exponent) {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() { String::new() } else { format!(".{rest}") };
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        return Some(format!("{first}{fraction}e{exponent_sign}{:02}", exponent.unsigned_abs()));
    }
    let fixed = match usize::try_from(point) {
        Ok(whole) if whole >= digits.len() => format!("{digits}{}.0", "0".repeat(whole - digits.len())),
        Ok(whole) if whole > 0 => {
            let (whole, fraction) = digits.split_at(whole);
            format!("{whole}.{fraction}")
        }
        _ => format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize)),
    };
    Some(fixed)
}
