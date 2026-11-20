use std::cell::OnceCell;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{Visitor, walk_expr};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

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

fn candidate(assign: &ast::StmtAssign) -> Option<&ast::ExprName> {
    let target = sole_name_target(assign)?;
    let name = target.id.as_str();
    (name != "_" && !is_dunder(name) && is_constant(&assign.value)).then_some(target)
}

// A string can contain source code that reads a local by name. Treat literal dictionary and subscript keys as data.
fn string_tokens(checker: &Checker) -> Vec<TextRange> {
    let mut keys = Keys(Vec::new());
    keys.visit_body(checker.body());
    let keys: FxHashSet<TextRange> = keys.0.into_iter().collect();
    checker
        .parsed
        .tokens()
        .iter()
        .filter(|token| matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle))
        .map(Ranged::range)
        .filter(|range| !keys.contains(range))
        .collect()
}

struct Keys(Vec<TextRange>);

impl Keys {
    fn add(&mut self, key: &Expr) {
        if let Expr::StringLiteral(string) = key {
            self.0.extend(string.value.iter().map(Ranged::range));
        }
    }
}

impl<'a> Visitor<'a> for Keys {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Subscript(subscript) => self.add(&subscript.slice),
            Expr::Dict(dict) => {
                for key in dict.iter_keys().flatten() {
                    self.add(key);
                }
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

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
    // An earlier read can run again on the next iteration. An outside read can run if an exception leaves this block
    // before the new assignment site.
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
    // Moving a constant past another one can make repeated fix passes alternate their order.
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
