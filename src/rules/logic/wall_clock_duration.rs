use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Operator, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::name_uses::LocalScope;

pub fn wall_clock_duration(checker: &Checker) {
    // `time.time` is spelled `time` wherever it is imported or called.
    if checker.state().unqualified_name_count(checker, "time") == 0 {
        return;
    }
    for scope in checker.state().local_scopes(checker).iter() {
        check_scope(checker, scope);
    }
}

fn check_scope(checker: &Checker, scope: &LocalScope) {
    let nodes = scope.nodes();
    // The names the scope binds once to `time.time()`, with the call each binds.
    let clocks: Vec<(&str, &ast::ExprCall)> = nodes
        .statements
        .iter()
        .filter_map(|stmt| {
            let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
                return None;
            };
            let [Expr::Name(target)] = targets.as_slice() else {
                return None;
            };
            let call = value.as_call_expr().filter(|call| is_wall_clock(checker, call))?;
            scope.uses().is_single_binding(target.id.as_str()).then_some((target.id.as_str(), call))
        })
        .collect();
    if clocks.is_empty() {
        return;
    }
    let clock_names: FxHashSet<&str> = clocks.iter().map(|(name, _)| *name).collect();
    let subtractions: Vec<&ast::ExprBinOp> = nodes
        .expressions
        .iter()
        .filter_map(|expr| expr.as_bin_op_expr())
        .filter(|bin_op| {
            bin_op.op == Operator::Sub
                && is_reading(checker, &bin_op.left, &clock_names)
                && is_reading(checker, &bin_op.right, &clock_names)
                && (bin_op.left.is_name_expr() || bin_op.right.is_name_expr())
        })
        .collect();
    if subtractions.is_empty() {
        return;
    }
    let operands: FxHashSet<TextRange> =
        subtractions.iter().flat_map(|bin_op| [bin_op.left.range(), bin_op.right.range()]).collect();
    let involved: Vec<&(&str, &ast::ExprCall)> =
        clocks.iter().filter(|(name, _)| subtractions.iter().any(|bin_op| mentions(bin_op, name))).collect();
    let uses = scope.uses();
    let only_durations =
        involved.iter().all(|(name, _)| uses.loads(name).iter().all(|load| operands.contains(&load.range)));
    let reports = subtractions
        .iter()
        .map(|bin_op| {
            (
                bin_op.range(),
                "`time.time()` follows the system clock, which can jump; `time.perf_counter()` measures durations"
                    .to_owned(),
                "Use `time.perf_counter()`".to_owned(),
            )
        })
        .collect();
    checker.report_shared_fix(Rule::WallClockDuration, reports, || {
        if !only_durations {
            return None;
        }
        let calls = involved.iter().map(|(_, call)| *call).chain(
            subtractions
                .iter()
                .flat_map(|bin_op| [bin_op.left.as_ref(), bin_op.right.as_ref()])
                .filter_map(Expr::as_call_expr),
        );
        let mut edits: Vec<Edit> = calls
            .map(|call| {
                let Expr::Attribute(attribute) = call.func.as_ref() else {
                    return None;
                };
                Some(Edit::range_replacement("perf_counter".to_owned(), attribute.attr.range()))
            })
            .collect::<Option<_>>()?;
        edits.sort_by_key(Edit::start);
        edits.dedup_by_key(|edit| edit.start());
        let first = edits.remove(0);
        Some(Fix::safe_edits(first, edits))
    });
}

fn is_wall_clock(checker: &Checker, call: &ast::ExprCall) -> bool {
    call.arguments.is_empty() && checker.semantic().resolves_to(&call.func, &["time", "time"])
}

fn is_reading(checker: &Checker, expr: &Expr, clocks: &FxHashSet<&str>) -> bool {
    match expr {
        Expr::Name(name) => clocks.contains(name.id.as_str()),
        Expr::Call(call) => is_wall_clock(checker, call),
        _ => false,
    }
}

fn mentions(bin_op: &ast::ExprBinOp, name: &str) -> bool {
    [bin_op.left.as_ref(), bin_op.right.as_ref()].iter().any(|operand| crate::syntax::is_name(operand, name))
}
