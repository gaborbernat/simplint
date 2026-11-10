use std::cell::OnceCell;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::is_alone_on_lines;
use crate::rules::logic::inline::{Names, enter_function, for_each_scoped_body, is_constant};

pub fn early_assignment(checker: &Checker) {
    // A function's scope is walked once, and only when one of its blocks assigns a constant to a name.
    let mut scopes: FxHashMap<TextRange, Scope> = FxHashMap::default();
    let strings = OnceCell::new();
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, _depth| {
        let Some(function) = function else {
            return;
        };
        for (index, stmt) in body.iter().enumerate() {
            if let Stmt::Assign(assign) = stmt
                && let Some(target) = candidate(assign)
            {
                let scope = scopes.entry(function.range).or_insert_with(|| Scope::of(&function.body));
                let strings = strings.get_or_init(|| string_tokens(checker));
                check_assign(checker, scope, strings, body, index, (assign, target));
            }
        }
    });
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

// The name an assignment binds when it assigns a constant to that single local name, the only kind the rule moves.
fn candidate(assign: &ast::StmtAssign) -> Option<&ast::ExprName> {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return None;
    };
    let name = target.id.as_str();
    (name != "_" && !is_dunder(name) && is_constant(&assign.value)).then_some(target)
}

// A function's names, the ranges of the scopes nested in it, and whether it reads names by their text: a scope runs
// whenever it is called, so a name it mentions must exist from the assignment on.
struct Scope<'a> {
    names: Names<'a>,
    nested: Vec<TextRange>,
    dynamic: bool,
}

impl<'a> Scope<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut walk = ScopeWalk::default();
        walk.visit_body(body);
        Self { names: Names::of(body), nested: walk.ranges, dynamic: walk.dynamic }
    }
}

#[derive(Default)]
struct ScopeWalk {
    ranges: Vec<TextRange>,
    dynamic: bool,
}

impl<'a> Visitor<'a> for ScopeWalk {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            self.ranges.push(stmt.range());
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => self.ranges.push(expr.range()),
            Expr::Name(name) => self.dynamic |= reads_by_name(name.id.as_str()),
            Expr::Attribute(attribute) => self.dynamic |= reads_by_name(attribute.attr.as_str()),
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// The builtins that read the locals or globals by name, and the `exec` and `exec_` calls that run code from a string,
// such as `six.exec_`, which may read any name in the function.
fn reads_by_name(name: &str) -> bool {
    matches!(name, "eval" | "globals" | "locals" | "vars") || name.ends_with("exec") || name.ends_with("exec_")
}

// Whether the text holds the name as a whole identifier, so `out` is in `"out.append(1)"` but not in `"output"`.
fn spells(text: &str, name: &str) -> bool {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_')).any(|word| word == name)
}

// A constant assigned to a local whose first use is at least two statements down the same block. Nothing reads or
// binds the name between the two, so moving the assignment down to the use cannot change what the function does, and a
// constant is the same wherever it is built.
fn check_assign<'a>(
    checker: &Checker<'a>,
    scope: &Scope<'a>,
    strings: &[TextRange],
    body: &'a [Stmt],
    index: usize,
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
) {
    let name = target.id.as_str();
    let names = &scope.names;
    if names.declared_outer.contains(name) || names.rebound.contains(name) || names.reads_locals || scope.dynamic {
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
            || scope.nested.iter().any(|nested| nested.contains_range(range))
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
    let candidate = after.partition_point(|stmt| stmt.start() <= first.start()).checked_sub(1);
    let Some(offset) = candidate.filter(|&offset| after[offset].range().contains_range(first)) else { return };
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
        move_fix(checker, assign, next),
        "Move the assignment down",
    );
}

fn is_constant_assign(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Assign(assign) if is_constant(&assign.value))
}

// The assignment's lines move in front of the statement that uses the name, when both sit on lines of their own. A
// statement sharing its line with others, joined by `;`, gets the line split around it, as `ruff format` splits it, so
// the assignment lands on a line of its own.
fn move_fix(checker: &Checker, assign: &ast::StmtAssign, next: &Stmt) -> Option<Fix> {
    let source = checker.source;
    if !is_alone_on_lines(source, assign.range) {
        return None;
    }
    let lines = source.full_lines_range(assign.range);
    let start = source.line_start(next.start());
    let end = source.line_end(next.end());
    let before = checker.text(TextRange::new(start, next.start()));
    let after = checker.text(TextRange::new(next.end(), end));
    if before.trim().is_empty() && after.trim().is_empty() {
        let text = checker.text(lines).to_owned();
        return Some(Fix::safe_edits(Edit::range_deletion(lines), [Edit::insertion(text, start)]));
    }
    let indentation = leading_indentation(before);
    let eol = checker.stylist.line_ending().as_str();
    let assign_text = checker.text(assign.range);
    let statement = checker.text(next.range());
    let mut content = if before.trim().is_empty() {
        format!("{indentation}{assign_text}{eol}{indentation}{statement}")
    } else {
        let head = before.trim_end().strip_suffix(';')?.trim_end();
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
