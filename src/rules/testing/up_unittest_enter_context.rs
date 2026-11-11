use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_case;
use crate::rules::{indentation, is_alone_on_lines};
use crate::semantic::Semantic;
use crate::syntax::for_each_body;

const PATCHERS: &[&[&str]] = &[
    &["unittest", "mock", "patch"],
    &["unittest", "mock", "patch", "object"],
    &["unittest", "mock", "patch", "dict"],
    &["mock", "patch"],
    &["mock", "patch", "object"],
    &["mock", "patch", "dict"],
];

pub fn up_unittest_enter_context(checker: &Checker) {
    let mut spellings = FxHashMap::default();
    for class in checker.class_defs().iter().filter(|class| is_test_case(checker, class)) {
        for method in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
            for_each_body(&method.body, &mut |statements, _| {
                for window in statements.windows(3) {
                    if let [start, bind, cleanup] = window {
                        check_sequence(checker, &mut spellings, method.range(), start, bind, cleanup);
                    }
                }
            });
        }
    }
}

// The three statements a manual start and stop take: `patcher = patch(...)`, `value = patcher.start()` and
// `self.addCleanup(patcher.stop)`.
fn check_sequence<'a>(
    checker: &Checker<'a>,
    spellings: &mut FxHashMap<TextRange, FxHashMap<&'a str, usize>>,
    method: TextRange,
    start: &Stmt,
    bind: &Stmt,
    cleanup: &Stmt,
) {
    let Some((patcher, call)) = patcher_assignment(checker.semantic(), start) else {
        return;
    };
    let Some(value) = started_value(bind, patcher) else {
        return;
    };
    if !is_cleanup_of(cleanup, patcher) {
        return;
    }
    // The fix drops the patcher's name, so the method may use it only in the three statements, each once.
    let counts = spellings.entry(method).or_insert_with(|| identifier_counts(checker, method));
    let used_elsewhere = counts.get(patcher).is_some_and(|&count| count > 3);
    let source = checker.source;
    let range = TextRange::new(start.start(), cleanup.end());
    let fix = (!used_elsewhere
        && !checker.has_comment_in(range)
        && [start, bind, cleanup].iter().all(|stmt| is_alone_on_lines(source, stmt.range())))
    .then(|| {
        let replacement = format!(
            "{}{value} = self.enterContext({}){}",
            indentation(source, start),
            checker.text(call.range()),
            checker.line_ending()
        );
        Fix::safe_edit(Edit::range_replacement(replacement, source.full_lines_range(range)))
    });
    checker.report_fix(
        Rule::UnittestEnterContext,
        start.range(),
        "A patch started and stopped by hand; `self.enterContext` starts it and registers the stop",
        fix,
        "Enter the patch with `self.enterContext`",
    );
}

// How often each name is spelled as an identifier within a method.
fn identifier_counts<'a>(checker: &Checker<'a>, method: TextRange) -> FxHashMap<&'a str, usize> {
    let mut counts = FxHashMap::default();
    for token in checker.parsed.tokens().in_range(method).iter().filter(|token| token.kind() == TokenKind::Identifier) {
        *counts.entry(checker.text(token.range())).or_default() += 1;
    }
    counts
}

fn patcher_assignment<'a>(semantic: &Semantic, stmt: &'a Stmt) -> Option<(&'a str, &'a ast::ExprCall)> {
    let (target, value) = single_name_assignment(stmt)?;
    let Expr::Call(call) = value else {
        return None;
    };
    PATCHERS.iter().any(|path| semantic.resolves_to(&call.func, path)).then_some((target, call))
}

// The name the started patch's value is bound to, when the statement is `value = patcher.start()`.
fn started_value<'a>(stmt: &'a Stmt, patcher: &str) -> Option<&'a str> {
    let (target, value) = single_name_assignment(stmt)?;
    let Expr::Call(call) = value else {
        return None;
    };
    (call.arguments.is_empty() && is_method_on(&call.func, patcher, "start")).then_some(target)
}

// `self.addCleanup(patcher.stop)`, with no other argument.
fn is_cleanup_of(stmt: &Stmt, patcher: &str) -> bool {
    let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
        return false;
    };
    let Expr::Call(call) = value.as_ref() else {
        return false;
    };
    let [stop] = call.arguments.args.as_ref() else {
        return false;
    };
    call.arguments.keywords.is_empty()
        && is_method_on(&call.func, "self", "addCleanup")
        && is_method_on(stop, patcher, "stop")
}

fn single_name_assignment(stmt: &Stmt) -> Option<(&str, &Expr)> {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return None;
    };
    let [Expr::Name(target)] = targets.as_slice() else {
        return None;
    };
    Some((target.id.as_str(), value))
}

// Whether `expr` is `receiver.member`, with `receiver` a plain name.
fn is_method_on(expr: &Expr, receiver: &str, member: &str) -> bool {
    expr.as_attribute_expr().is_some_and(|attribute| {
        attribute.attr.as_str() == member && attribute.value.as_name_expr().is_some_and(|name| name.id == receiver)
    })
}
