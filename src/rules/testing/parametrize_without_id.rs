use std::borrow::Cow;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::fix_of;
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
            if let Some(ids) = arguments.find_argument_value("ids", 3) {
                if let Some(id_values) = id_strings(ids) {
                    let fix = ids_fix(checker, func, arguments, values, id_values);
                    report_ids_list(checker, ids, fix);
                } else if is_module_id_strings(checker.semantic(), ids) {
                    report_ids_list(checker, ids, None);
                }
                continue;
            }
            let (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = values else {
                continue;
            };
            let rows = rows(checker, elts);
            let names = argnames(arguments);
            let mut id_counts = FxHashMap::default();
            for id in rows.iter().filter_map(|row| row_id(checker, row, names)) {
                *id_counts.entry(id).or_insert(0) += 1;
            }
            for row in rows.iter().filter(|row| !is_param_with_id(checker.semantic(), row)) {
                if row_id(checker, row, names).is_some_and(|id| id_counts.get(&id) == Some(&1))
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

// Callable IDs can describe a case from its values.
fn id_strings(ids: &Expr) -> Option<&[Expr]> {
    let (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = ids else {
        return None;
    };
    elts.iter().all(Expr::is_string_literal_expr).then_some(elts.as_slice())
}

// Only a single module-level binding gives the rule a local list to inspect here.
fn is_module_id_strings(semantic: &Semantic, ids: &Expr) -> bool {
    let Expr::Name(name) = ids else {
        return false;
    };
    let bindings = semantic.resolved_bindings(name);
    let [binding] = bindings.as_slice() else {
        return false;
    };
    binding.scope.is_global() && semantic.only_binding_value(name).is_some_and(|value| id_strings(value).is_some())
}

fn report_ids_list(checker: &Checker, ids: &Expr, fix: Option<Fix>) {
    checker.report_fix(
        Rule::ParametrizeWithoutId,
        ids.range(),
        "Parametrize `ids=` list keeps each id apart from its row; wrap each row in `pytest.param(..., id=...)`",
        fix,
        "Wrap each row in `pytest.param(..., id=...)` and drop `ids=`",
    );
}

fn ids_fix(
    checker: &Checker,
    func: &Expr,
    arguments: &ast::Arguments,
    values: &Expr,
    id_values: &[Expr],
) -> Option<Fix> {
    let keyword = arguments.find_keyword("ids")?;
    let argnames = argnames(arguments)?;
    let (Expr::List(ast::ExprList { elts: rows, .. }) | Expr::Tuple(ast::ExprTuple { elts: rows, .. })) = values else {
        return None;
    };
    if rows.len() != id_values.len() {
        return None;
    }
    let callee = param_callee(checker, func)?;
    let mut edits = Vec::with_capacity(rows.len() + 1);
    for (row, id) in rows.iter().zip(id_values) {
        let row_arguments = row_arguments(checker, row, argnames)?;
        let replacement = format!("{callee}({row_arguments}, id={})", checker.text(id));
        edits.push(Edit::range_replacement(replacement, row.range()));
    }
    let removal = keyword_removal(arguments, keyword);
    edits.push(Edit::range_deletion(removal));
    let commented = checker.has_comment_in(removal.cover(values.range()));
    fix_of(edits, if commented { Applicability::Unsafe } else { Applicability::Safe })
}

// Keep a single argument's value intact. A sequence of argument names, or a trailing comma in the name string, asks
// pytest to unpack the row.
#[derive(Clone, Copy)]
enum Argnames {
    Whole,
    Unpacked(usize),
}

fn argnames(arguments: &ast::Arguments) -> Option<Argnames> {
    match arguments.find_argument_value("argnames", 0)? {
        Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
            let text = value.to_str();
            let count = text.split(',').filter(|name| !name.trim().is_empty()).count();
            Some(if count == 1 && !text.trim_end().ends_with(',') {
                Argnames::Whole
            } else {
                Argnames::Unpacked(count)
            })
        }
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().all(Expr::is_string_literal_expr).then_some(Argnames::Unpacked(elts.len()))
        }
        _ => None,
    }
}

// The module's own name for pytest, as the decorator spells it: `import pytest as pt` gives `pt.param`.
fn param_callee(checker: &Checker, func: &Expr) -> Option<String> {
    let Expr::Attribute(ast::ExprAttribute { value: mark, .. }) = func else {
        return None;
    };
    let Expr::Attribute(ast::ExprAttribute { value: module, .. }) = mark.as_ref() else {
        return None;
    };
    Some(format!("{}.param", checker.text(module.as_ref())))
}

fn row_arguments(checker: &Checker, row: &Expr, argnames: Argnames) -> Option<String> {
    let Argnames::Unpacked(names) = argnames else {
        return is_literal_row(row).then(|| checker.text(row).to_owned());
    };
    let (Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. })) = row else {
        return None;
    };
    (!elts.is_empty() && elts.len() == names && !elts.iter().any(Expr::is_starred_expr))
        .then(|| elts.iter().map(|elt| checker.text(elt)).collect::<Vec<_>>().join(", "))
}

fn is_literal_row(row: &Expr) -> bool {
    match row {
        Expr::Tuple(_) | Expr::List(_) => true,
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub, operand, .. }) => operand.is_number_literal_expr(),
        _ => row.is_literal_expr(),
    }
}

// Remove the adjacent comma with the keyword so the remaining argument list is valid.
fn keyword_removal(arguments: &ast::Arguments, keyword: &ast::Keyword) -> TextRange {
    let before = arguments
        .args
        .iter()
        .map(Ranged::end)
        .chain(arguments.keywords.iter().map(Ranged::end))
        .filter(|end| *end <= keyword.start())
        .max();
    if let Some(start) = before {
        return TextRange::new(start, keyword.end());
    }
    let after = arguments
        .args
        .iter()
        .map(Ranged::start)
        .chain(arguments.keywords.iter().map(Ranged::start))
        .filter(|start| *start >= keyword.end())
        .min()
        .expect("the argument names lie before or after `ids=`");
    TextRange::new(keyword.start(), after)
}

// Expand a starred list or tuple once if its elements are available in this file.
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

// A named row can have several bindings. Any binding with an explicit ID can supply the case. For a single
// pytest.param binding, report the call where the ID belongs.
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

// A local helper can already supply IDs. Coroutines and generators return other objects, and decorators other than the
// supported caches may replace its result.
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
    semantic.is_decorator(decorator, &[&["functools", "cache"], &["functools", "lru_cache"]])
}

// A loop may run zero times, and a context manager may suppress an exception before the return.
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

// The id pytest builds for a row: a whole row's own id, or the ids of an unpacked row's values joined with `-`.
fn row_id<'e>(checker: &Checker, row: &'e Expr, names: Option<Argnames>) -> Option<Cow<'e, str>> {
    let Some(Argnames::Unpacked(count)) = names else {
        return scalar_id(checker, row);
    };
    let (Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. })) = row else {
        return None;
    };
    if elts.len() != count {
        return None;
    }
    let ids = elts.iter().map(|value| scalar_id(checker, value)).collect::<Option<Vec<_>>>()?;
    Some(Cow::Owned(ids.join("-")))
}

// Compare the IDs pytest would display so equal numeric values and duplicate rows count together.
fn scalar_id<'e>(checker: &Checker, element: &'e Expr) -> Option<Cow<'e, str>> {
    match element {
        Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
            let text = value.to_str();
            text.bytes().all(|byte| matches!(byte, b' '..=b'~') && byte != b'\\').then_some(Cow::Borrowed(text))
        }
        Expr::BytesLiteral(ast::ExprBytesLiteral { value, .. }) => {
            let bytes: Vec<u8> = value.bytes().collect();
            bytes
                .iter()
                .all(|byte| matches!(byte, b' '..=b'~') && *byte != b'\\')
                .then(|| Cow::Owned(bytes.iter().map(|byte| char::from(*byte)).collect()))
        }
        Expr::BooleanLiteral(_) | Expr::NoneLiteral(_) => Some(Cow::Owned(checker.text(element).to_owned())),
        Expr::NumberLiteral(number) => Some(Cow::Owned(number_id(checker, number))),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::USub, operand, .. }) => {
            let id = number_id(checker, operand.as_number_literal_expr()?);
            Some(Cow::Owned(if id == "0" { id } else { format!("-{id}") }))
        }
        _ => None,
    }
}

// Use the source spelling as a fallback for complex numbers and integers larger than u64.
fn number_id(checker: &Checker, number: &ast::ExprNumberLiteral) -> String {
    match &number.value {
        ast::Number::Int(int) => int.as_u64().map_or_else(|| checker.text(number).to_owned(), |v| v.to_string()),
        ast::Number::Float(float) => float_id(*float),
        ast::Number::Complex { .. } => checker.text(number).to_owned(),
    }
}

// Python's `repr` of a float: the shortest digits that read back as the same value, in fixed notation from 1e-4 up to
// 1e16 and in exponent notation beyond.
fn float_id(value: f64) -> String {
    if value.is_infinite() {
        return "inf".to_owned();
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("a finite float formats with an exponent");
    let exponent: i32 = exponent.parse().expect("the exponent is an integer");
    let digits: String = mantissa.chars().filter(|digit| *digit != '.').collect();
    let point = exponent + 1;
    if !(-4..16).contains(&exponent) {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() { String::new() } else { format!(".{rest}") };
        let exponent_sign = if exponent < 0 { '-' } else { '+' };
        return format!("{first}{fraction}e{exponent_sign}{:02}", exponent.unsigned_abs());
    }
    match usize::try_from(point) {
        Ok(whole) if whole >= digits.len() => format!("{digits}{}.0", "0".repeat(whole - digits.len())),
        Ok(whole) if whole > 0 => {
            let (whole, fraction) = digits.split_at(whole);
            format!("{whole}.{fraction}")
        }
        _ => format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize)),
    }
}
