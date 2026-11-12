use ruff_python_ast::{self as ast, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::flow::{FunctionNames, collect_names, first_exit, has_own_break};
use crate::rules::logic::inline::{enter_function, for_each_scoped_body};

pub fn loop_variable_after_loop(checker: &Checker) {
    let mut functions: FxHashMap<TextRange, FunctionNames> = FxHashMap::default();
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, _| {
        let Some(function) = function else {
            return;
        };
        for (index, stmt) in body.iter().enumerate() {
            let Stmt::For(for_loop) = stmt else {
                continue;
            };
            // A loop left by `break` keeps the item it stopped at, the usual way to search, and an `else` that leaves
            // the block means the code after the loop only runs after such a `break`.
            if has_own_break(&for_loop.body) || first_exit(&for_loop.orelse).is_some() {
                continue;
            }
            let names = functions.entry(function.range).or_insert_with(|| FunctionNames::of(&function.body));
            let mut targets = Vec::new();
            collect_names(&for_loop.target, &mut targets);
            for target in targets {
                check_target(checker, names, for_loop, &body[index + 1..], target);
            }
        }
    });
}

fn check_target(checker: &Checker, names: &FunctionNames, for_loop: &ast::StmtFor, rest: &[Stmt], name: &str) {
    if name == "_" || !names.is_plain_local(name) {
        return;
    }
    let mut mentions = names.mentions(name);
    // A name bound or read before the loop has a value of its own when the loop runs no time.
    if mentions.clone().any(|(range, _)| range.start() < for_loop.target.start()) {
        return;
    }
    // So does one the `else` binds, and it runs whenever the loop ends.
    let orelse = for_loop
        .orelse
        .first()
        .zip(for_loop.orelse.last())
        .map(|(first, last)| TextRange::new(first.start(), last.end()));
    if mentions.clone().any(|(range, _)| orelse.is_some_and(|orelse| orelse.contains_range(range))) {
        return;
    }
    let Some(after) = rest.first().zip(rest.last()).map(|(first, last)| TextRange::new(first.start(), last.end()))
    else {
        return;
    };
    let Some((read, ExprContext::Load)) = mentions.find(|(range, _)| after.contains_range(*range)) else {
        return;
    };
    let line = checker.line_number(for_loop.start());
    checker.report(
        Rule::LoopVariableAfterLoop,
        read,
        format!(
            "`{name}` is read after the loop on line {line} ends: it holds the last item, and is unbound when the loop \
             runs no time"
        ),
    );
}
