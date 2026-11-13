use std::cell::OnceCell;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, ExprContext, Stmt};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::inline::{enter_function, for_each_scoped_body, is_constant};
use crate::rules::logic::name_uses::FunctionNames;
use crate::rules::logic::sole_name_target;
use crate::syntax::identifier_words;

pub fn early_assignment(checker: &Checker) {
    let scopes = checker.state().local_scopes(checker);
    let cached_strings = OnceCell::new();
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, _depth| {
        let Some(function) = function else {
            return;
        };
        for (index, stmt) in body.iter().enumerate() {
            if let Stmt::Assign(assign) = stmt
                && let Some(target) = candidate(assign)
            {
                // Built on the first candidate, since most functions assign no constant.
                let scope = scopes.function(function).names();
                let strings = cached_strings.get_or_init(|| string_tokens(checker));
                check_assign(checker, scope, strings, (body, index), (assign, target));
            }
        }
    });
}

// The name an assignment binds when it assigns a constant to that single local name, the only kind the rule moves.
fn candidate(assign: &ast::StmtAssign) -> Option<&ast::ExprName> {
    let target = sole_name_target(assign)?;
    let name = target.id.as_str();
    (name != "_" && !is_dunder(name) && is_constant(&assign.value)).then_some(target)
}

// The ranges of the string tokens of the file in source order, which may hold a name the rule must not move past.
fn string_tokens(checker: &Checker) -> Vec<TextRange> {
    checker
        .parsed
        .tokens()
        .iter()
        .filter(|token| matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle))
        .map(Ranged::range)
        .collect()
}

// Moving the assignment down to its first use cannot change what the function does: nothing reads or binds the name
// between the two, and a constant is the same wherever it is built.
fn check_assign<'a>(
    checker: &Checker<'a>,
    scope: &FunctionNames<'a>,
    strings: &[TextRange],
    (body, index): (&'a [Stmt], usize),
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
) {
    let name = target.id.as_str();
    let names = &scope.names;
    if names.declared_outer.contains(name) || names.rebound.contains(name) || names.reads_by_text {
        return;
    }
    // The name must be read, not bound again, and every mention must sit in this block after the assignment. A use
    // before the assignment may run again in a loop, and a use outside the block may run when the block is left by an
    // exception, so either would see the name unbound after the move. One pass finds the first use and stops at the
    // first use out of place, as a name a function mentions often is checked once per assignment of it.
    let block = TextRange::new(body[0].start(), body[body.len() - 1].end());
    let mut earliest: Option<(TextRange, ExprContext)> = None;
    for (range, context) in names.mentions.get(name).into_iter().flatten().copied() {
        if range == target.range {
            continue;
        }
        if range.start() < assign.end()
            || !block.contains_range(range)
            || scope.deferred.holders(range).next().is_some()
        {
            return;
        }
        if earliest.is_none_or(|(known, _)| range.start() < known.start()) {
            earliest = Some((range, context));
        }
    }
    let Some((first, ExprContext::Load)) = earliest else {
        return;
    };
    // A string between the assignment and the use may be code that reads the name by its text, as `exec` runs one.
    let from = strings.partition_point(|string| string.start() < assign.end());
    let to = strings.partition_point(|string| string.end() <= first.start());
    if strings[from..to].iter().any(|string| spells(checker.text(*string), name)) {
        return;
    }
    let after = &body[index + 1..];
    // The use sits in the block after the assignment, so in the last statement that starts before it.
    let offset = after.partition_point(|stmt| stmt.start() <= first.start()) - 1;
    let next = &after[offset];
    // Only the statement directly after the assignment is SMP504's, which inlines a single use.
    if offset == 0 || checker.has_comment_in(TextRange::new(assign.start(), next.start())) {
        return;
    }
    // Two constants that wait for the same use would take turns moving down past each other, one round after another.
    // Left out, a constant only moves past statements that are not constants, which ends the fix rounds.
    if body[index + 1..index + 1 + offset].iter().any(is_constant_assign) {
        return;
    }
    checker.report_fix(
        Rule::EarlyAssignment,
        assign.range,
        format!("`{name}` is assigned above the statements that do not use it; assign it just before its first use"),
        // A comment above the assignment would stay where it is and sit above the statement that follows it.
        checker.comment_above(assign).is_none().then(|| move_fix(checker, assign, next)).flatten(),
        "Move the assignment down",
    );
}

// Whether the text holds the name as a whole identifier, so `out` is in `"out.append(1)"` but not in `"output"`.
fn spells(text: &str, name: &str) -> bool {
    identifier_words(text).any(|word| word == name)
}

fn is_constant_assign(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Assign(assign) if is_constant(&assign.value))
}

// The assignment's lines move in front of the statement that uses the name, when both sit on lines of their own. A
// statement sharing its line with others, joined by `;`, gets the line split around it, as `ruff format` splits it, so
// the assignment lands on a line of its own.
fn move_fix(checker: &Checker, assign: &ast::StmtAssign, next: &Stmt) -> Option<Fix> {
    if !checker.is_alone_on_lines(assign.range) {
        return None;
    }
    let source = checker.source;
    let lines = source.full_lines_range(assign.range);
    let start = source.line_start(next.start());
    let end = source.line_end(next.end());
    let before = checker.text(TextRange::new(start, next.start()));
    let after = checker.text(TextRange::new(next.end(), end));
    if before.trim().is_empty() && after.trim().is_empty() {
        return Some(Fix::safe_edits(
            Edit::range_deletion(lines),
            [Edit::insertion(checker.text(lines).to_owned(), start)],
        ));
    }
    let indentation = leading_indentation(before);
    let eol = checker.line_ending();
    let assign_text = checker.text(assign.range);
    let statement = checker.text(next.range());
    let mut content = if before.trim().is_empty() {
        format!("{indentation}{assign_text}{eol}{indentation}{statement}")
    } else {
        // The assignment sits alone on its lines, so a statement in front of the use on its line ends with `;`.
        let head = before.trim_end().strip_suffix(';').expect("a statement before the use on its line").trim_end();
        format!("{head}{eol}{indentation}{assign_text}{eol}{indentation}{statement}")
    };
    if !after.trim().is_empty() {
        let rest = after.trim_start().strip_prefix(';')?.trim_start();
        if !rest.is_empty() {
            content = format!("{content}{eol}{indentation}{rest}");
        }
    }
    Some(Fix::safe_edits(Edit::range_deletion(lines), [Edit::range_replacement(content, TextRange::new(start, end))]))
}
