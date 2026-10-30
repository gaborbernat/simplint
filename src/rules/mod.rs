pub mod comments;
pub mod constants;
pub mod dataclasses;
pub mod docstrings;
pub mod exports;
pub mod functions;
pub mod imports;
pub mod mocks;
pub mod names;
pub mod order;
pub mod pytest;
pub mod strings;
pub mod typing;
pub mod walrus;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, Stmt};
use ruff_python_trivia::{has_leading_content, has_trailing_content};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::checker::Checker;

pub fn is_alone_on_lines(source: &str, range: TextRange) -> bool {
    !has_leading_content(range.start(), source) && !has_trailing_content(range.end(), source)
}

pub fn delete_statement(source: &str, stmt: &Stmt, body: &[Stmt]) -> Edit {
    if body.len() == 1 {
        return Edit::range_replacement("pass".to_owned(), stmt.range());
    }
    Edit::range_deletion(source.full_lines_range(stmt.range()))
}

pub fn lines_with_trailing_blanks(source: &str, range: TextRange) -> TextRange {
    let mut end = source.full_line_end(range.end());
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
    }
    TextRange::new(source.line_start(range.start()), end)
}

pub fn for_each_body<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    walk_bodies(body, None, visit);
}

fn walk_bodies<'a>(body: &'a [Stmt], owner: Option<&'a Stmt>, visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    visit(body, owner);
    for stmt in body {
        for child in child_bodies(stmt) {
            walk_bodies(child, Some(stmt), visit);
        }
    }
}

fn child_bodies(stmt: &Stmt) -> Vec<&[Stmt]> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { body, .. })
        | Stmt::ClassDef(ast::StmtClassDef { body, .. })
        | Stmt::With(ast::StmtWith { body, .. }) => vec![body],
        Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
            vec![body, orelse]
        }
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => std::iter::once(body.as_slice())
            .chain(elif_else_clauses.iter().map(|clause| clause.body.as_slice()))
            .collect(),
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => std::iter::once(body.as_slice())
            .chain(handlers.iter().map(|ast::ExceptHandler::ExceptHandler(handler)| handler.body.as_slice()))
            .chain([orelse.as_slice(), finalbody.as_slice()])
            .collect(),
        Stmt::Match(ast::StmtMatch { cases, .. }) => cases.iter().map(|case| case.body.as_slice()).collect(),
        _ => Vec::new(),
    }
}

pub fn function_defs(body: &[Stmt]) -> Vec<&ast::StmtFunctionDef> {
    let mut functions = Vec::new();
    for_each_body(body, &mut |statements, _| {
        functions.extend(statements.iter().filter_map(Stmt::as_function_def_stmt));
    });
    functions
}

pub fn class_defs(body: &[Stmt]) -> Vec<&ast::StmtClassDef> {
    let mut classes = Vec::new();
    for_each_body(body, &mut |statements, _| {
        classes.extend(statements.iter().filter_map(Stmt::as_class_def_stmt));
    });
    classes
}

pub fn docstring(body: &[Stmt]) -> Option<&ast::ExprStringLiteral> {
    match body.first()? {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_string_literal_expr(),
        _ => None,
    }
}

pub fn dunder_all(body: &[Stmt]) -> Option<Vec<&str>> {
    body.iter().rev().find_map(|stmt| {
        all_literal(stmt)?.1.iter().map(|element| Some(element.as_string_literal_expr()?.value.to_str())).collect()
    })
}

pub fn all_literal(stmt: &Stmt) -> Option<(TextRange, &[Expr])> {
    let value = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.first().is_some_and(is_all_target) => value,
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) if is_all_target(target) => value,
        _ => return None,
    };
    match value.as_ref() {
        Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => {
            Some((value.range(), elts))
        }
        _ => None,
    }
}

pub fn is_all_target(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| name.id.as_str() == "__all__")
}

pub fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
    let ranges = checker.semantic().name_ranges(old);
    if appears_as_name(checker, new)
        || checker.is_imported_elsewhere(old)
        || ranges.is_empty()
        || name_token_count(checker, old) != ranges.len()
        || appears_in_strings(checker, old)
    {
        return None;
    }
    let exports = all_elements(checker.body())
        .into_iter()
        .filter(|element| element.value.to_str() == old)
        .filter_map(|element| Some(element.value.iter().next()?.content_range()));
    let mut edits = ranges.into_iter().chain(exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    Some(Fix::unsafe_edits(edits.next()?, edits))
}

pub fn appears_as_name(checker: &Checker, name: &str) -> bool {
    name_token_count(checker, name) > 0
}

fn name_token_count(checker: &Checker, name: &str) -> usize {
    checker
        .parsed
        .tokens()
        .iter()
        .filter(|token| token.kind() == TokenKind::Identifier && checker.text(token.range()) == name)
        .count()
}

fn appears_in_strings(checker: &Checker, name: &str) -> bool {
    let exports: Vec<TextRange> = all_elements(checker.body()).into_iter().map(Ranged::range).collect();
    checker.parsed.tokens().iter().any(|token| {
        matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle)
            && !exports.iter().any(|export| export.contains_range(token.range()))
            && checker
                .text(token.range())
                .split(|char: char| !(char.is_alphanumeric() || char == '_'))
                .any(|word| word == name)
    })
}

fn all_elements(body: &[Stmt]) -> Vec<&ast::ExprStringLiteral> {
    body.iter()
        .filter_map(all_literal)
        .flat_map(|(_, elements)| elements)
        .filter_map(Expr::as_string_literal_expr)
        .collect()
}

pub fn statement_block(source: &str, stmt: &Stmt) -> TextRange {
    let start = match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, .. })
        | Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
            decorator_list.first().map_or(stmt.start(), Ranged::start)
        }
        _ => stmt.start(),
    };
    let mut line_start = source.line_start(start);
    while line_start > TextSize::default() {
        let previous = source.line_start(line_start - TextSize::from(1));
        if !source[TextRange::new(previous, line_start)].trim_start().starts_with('#') {
            break;
        }
        line_start = previous;
    }
    TextRange::new(line_start, source.full_line_end(stmt.end()))
}

pub fn indentation(source: &str, ranged: impl Ranged) -> &str {
    &source[TextRange::new(source.line_start(ranged.start()), ranged.start())]
}

pub fn needs_parentheses(value: &Expr) -> bool {
    OperatorPrecedence::from_expr(value) < OperatorPrecedence::CallAttribute
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
}

pub fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

pub fn body_mentions(body: &[Stmt], name: &str, within: TextRange) -> bool {
    let mut finder = NameFinder { name, within: Some(within), found: false };
    finder.visit_body(body);
    finder.found
}

pub fn expr_mentions(expr: &Expr, name: &str) -> bool {
    let mut finder = NameFinder { name, within: None, found: false };
    finder.visit_expr(expr);
    finder.found
}

struct NameFinder<'n> {
    name: &'n str,
    within: Option<TextRange>,
    found: bool,
}

impl<'a> Visitor<'a> for NameFinder<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr
            && name.id.as_str() == self.name
            && self.within.is_none_or(|within| within.contains_range(name.range))
        {
            self.found = true;
        }
        visitor::walk_expr(self, expr);
    }
}
