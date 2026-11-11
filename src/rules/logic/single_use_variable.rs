use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::helpers::{any_over_expr, is_dunder};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, keep_debug_text, statement_deletion};
use crate::rules::logic::inline::{Names, declared_outer, enter_function, fits, for_each_scoped_body};
use crate::rules::logic::sole_name_target;

mod reading;
mod replacement;
mod ruff;

use reading::{ReadSite, is_effect, reorders};
use replacement::inlined_replacement;
use ruff::{leads_if_test, raises_with_name, reports_default, returns_or_yields, ruff_reports_yield_from};

pub fn single_use_variable(checker: &Checker) {
    // A function's names are walked once, and only when one of its blocks assigns a single name.
    let mut names: FxHashMap<TextRange, Names> = FxHashMap::default();
    // A call rebinds only a name some function declares `global` or `nonlocal`.
    let rebindable = declared_outer(checker.body());
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, depth| {
        let Some(function) = function else {
            return;
        };
        for pair in body.windows(2) {
            if let [Stmt::Assign(assign), next] = pair
                && let Some(target) = sole_name_target(assign)
            {
                let names = names.entry(function.range).or_insert_with(|| Names::of(&function.body));
                check_pair(checker, (names, &rebindable), (assign, target), next, depth);
            }
        }
    });
}

// `assign` binds `target`, its only target.
fn check_pair<'a>(
    checker: &Checker<'a>,
    (names, rebindable): (&Names<'a>, &FxHashSet<&str>),
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
    next: &'a Stmt,
    depth: usize,
) {
    let name = target.id.as_str();
    let Some((use_range, reading)) = single_read(checker, names, (assign, target), next) else {
        return;
    };
    let Some((range, replacement)) = inlined_replacement(checker, assign, next, (use_range, &reading)) else {
        return;
    };
    if ruff_reports_yield_from(checker, target, next) {
        return;
    }
    let deletion = statement_deletion(checker, assign.range, Some(next.range()), Blanks::Shared)
        .expect("no comment sits between the assignment and the statement after it");
    let deletion = Edit::range_deletion(deletion);
    // An empty value, such as `''` spliced into an f-string, leaves nothing to write, which only a deletion expresses.
    let edit =
        if replacement.is_empty() { Edit::range_deletion(range) } else { Edit::range_replacement(replacement, range) };
    let edits = keep_debug_text(checker, vec![edit]);
    let fix = match edits {
        Some(edits) if !reading.in_tstring => {
            // A read in a `=` debug field rewrites the whole field, so the width is measured on the field it becomes.
            let Some(replace) = edits.first() else { return };
            if !fits(checker, next, replace, depth) {
                return;
            }
            let applicability =
                if reorders(rebindable, assign, &reading) { Applicability::Unsafe } else { Applicability::Safe };
            Some(Fix::applicable_edits(deletion, edits, applicability))
        }
        // A debug field that cannot be rewritten keeps the report, and so does a t-string, whose fields keep their
        // source text, as the variable is still used once.
        _ => None,
    };
    checker.report_fix(
        Rule::SingleUseVariable,
        assign.range,
        format!("`{name}` is used once, right below; inline its value"),
        fix,
        format!("Inline `{name}`"),
    );
}

// The one read of the assigned name that the statement after the assignment makes, with its range, when inlining the
// value there keeps the value's evaluation in order.
fn single_read<'a>(
    checker: &Checker<'a>,
    names: &Names<'a>,
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
    next: &'a Stmt,
) -> Option<(TextRange, ReadSite<'a>)> {
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    // A `while` test runs again on every pass, and `_` is a placeholder rather than a name to read.
    if name == "_"
        || is_dunder(name)
        || matches!(value, Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Await(_) | Expr::Named(_))
        || matches!(next, Stmt::While(_))
        || checker.has_comment_in(TextRange::new(assign.start(), next.start()))
        || names.declared_outer.contains(name)
        || leads_if_test(next, name)
        || returns_or_yields(checker, next, name)
        || raises_with_name(next, name)
        || names.rebound.contains(name)
        || names.string_reads.contains(name)
        || names.reads_locals
    {
        return None;
    }
    let mut others =
        names.mentions.get(name).into_iter().flatten().copied().filter(|(range, _)| *range != target.range);
    let (Some((use_range, ExprContext::Load)), None) = (others.next(), others.next()) else {
        return None;
    };
    if !next.range().contains_range(use_range) {
        return None;
    }
    let reading = ReadSite::of(next, use_range);
    // Inlining moves the generator into the call, which then drops it, so its `finally` runs at the call's end.
    if reading.read != Some(false) || reading.consumes {
        return None;
    }
    if reading.default_parameter.is_some_and(|parameter| reports_default(checker, value, parameter)) {
        return None;
    }
    // A conditional or a string literal nested in a replacement field reads worse than the statements it replaces.
    if !reading.fstring_quotes.is_empty()
        && !matches!(value, Expr::StringLiteral(_) | Expr::FString(_))
        && any_over_expr(value, &|sub: &Expr| matches!(sub, Expr::StringLiteral(_) | Expr::FString(_) | Expr::If(_)))
    {
        return None;
    }
    // Ruff's C419 writes a list comprehension passed to `all` or `any` as a generator, which stops at the first element
    // that decides the result, so the work of the elements past it would not run.
    if reading.short_circuits && matches!(value, Expr::ListComp(_)) && any_over_expr(value, &is_effect) {
        return None;
    }
    Some((use_range, reading))
}
