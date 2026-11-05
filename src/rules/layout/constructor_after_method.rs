use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast as ast;
use ruff_python_ast::visitor::Visitor;
use ruff_text_size::Ranged;

use super::definition_time::{bound_names, class_runtime_names, definition_time_exprs, signature_annotations};
use super::reorder::{Suite, reorder_fix};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{ExprCollector, is_overload};

pub fn constructor_after_method(checker: &Checker) {
    for class in checker.class_defs() {
        check_class(checker, class);
    }
}

fn check_class(checker: &Checker, class: &ast::StmtClassDef) {
    let body = &class.body;
    // The first method that is not a constructor: the constructors below it are the ones that move up.
    let Some((first, anchor)) = body.iter().enumerate().find_map(|(index, stmt)| {
        stmt.as_function_def_stmt()
            .filter(|function| !is_constructor(function.name.as_str()))
            .map(|function| (index, function))
    }) else {
        return;
    };
    let lazy = checker.semantic().annotations_are_lazy();
    let runtime_names = class_runtime_names(class);
    let mut counts: FxHashMap<&str, usize> = FxHashMap::default();
    for stmt in body
        .iter()
        .filter(|stmt| !stmt.as_function_def_stmt().is_some_and(|function| is_overload(&function.decorator_list)))
    {
        for name in bound_names(stmt) {
            *counts.entry(name).or_default() += 1;
        }
    }
    // The names the body binds from its first function up to the statement being checked: a constructor that reads one
    // of them when it is defined cannot move above it.
    let mut bound_before: FxHashSet<&str> = FxHashSet::default();
    let mut late = Vec::new();
    for (index, stmt) in body.iter().enumerate().skip(first + 1) {
        bound_before.extend(bound_names(&body[index - 1]));
        let Some(function) = stmt.as_function_def_stmt().filter(|function| is_constructor(function.name.as_str()))
        else {
            continue;
        };
        let name = function.name.as_str();
        let can_move = counts.get(name) == Some(&1)
            && !runtime_names.contains(name)
            && definition_reads(function, lazy).iter().all(|read| !bound_before.contains(read));
        late.push((index, &function.name, can_move));
    }
    if late.is_empty() {
        return;
    }
    // The `@overload` variants of a constructor share its name, and the last `def` of that name is the one the class
    // keeps: moving some variants past one that stays would leave a stub in its place.
    let stuck: FxHashSet<&str> =
        late.iter().filter(|(_, _, can_move)| !can_move).map(|(_, identifier, _)| identifier.as_str()).collect();
    let late: Vec<_> = late
        .into_iter()
        .map(|(index, identifier, can_move)| (index, identifier, can_move && !stuck.contains(identifier.as_str())))
        .collect();
    let moved: Vec<usize> = late.iter().filter(|(_, _, can_move)| *can_move).map(|(index, _, _)| *index).collect();
    let suite = Suite { body, separator: "\n" };
    let mut pending = (!moved.is_empty())
        .then(|| {
            let moved_set: FxHashSet<usize> = moved.iter().copied().collect();
            let expected: Vec<usize> =
                (0..first).chain(moved).chain((first..body.len()).filter(|index| !moved_set.contains(index))).collect();
            // The moved constructors land at `first`, so the span starts there.
            let high = (first..body.len()).rfind(|&position| expected[position] != position).unwrap_or(first);
            reorder_fix(checker, &suite, &expected, first..=high)
        })
        .flatten();
    let anchor_name = anchor.name.as_str();
    for (_, identifier, can_move) in late {
        let name = identifier.as_str();
        let message = format!("`{name}` is defined after `{anchor_name}`; constructors come before the other methods");
        let fix = if can_move { pending.take() } else { None };
        checker.report_fix(
            Rule::ConstructorAfterMethod,
            identifier.range(),
            message,
            fix,
            format!("Move `{name}` above `{anchor_name}`"),
        );
    }
}

// The methods Python calls to make an instance. A reader starts where the instance is made, so these sit ahead of the
// other methods.
pub(super) fn is_constructor(name: &str) -> bool {
    matches!(name, "__new__" | "__init__" | "__post_init__" | "__init_subclass__")
}

// The names a `def` reads when it runs: its decorators and default values, and its annotations when Python evaluates
// them at definition time. Decorators read annotations even under lazy annotations, as SMP201 assumes.
fn definition_reads(function: &ast::StmtFunctionDef, lazy: bool) -> FxHashSet<&str> {
    let mut reads = ExprCollector::new(|expr| expr.as_name_expr().map(|name| name.id.as_str()));
    for expr in definition_time_exprs(function) {
        reads.visit_expr(expr);
    }
    if !lazy || !function.decorator_list.is_empty() {
        for expr in signature_annotations(function) {
            reads.visit_expr(expr);
        }
    }
    reads.found.into_iter().collect()
}
