pub(crate) mod comments;
pub(crate) mod dataclasses;
pub(crate) mod docstrings;
pub(crate) mod exports;
pub(crate) mod imports;
pub(crate) mod naming;
pub(crate) mod strings;
pub(crate) mod structure;
pub(crate) mod testing;
pub(crate) mod typing;

use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::{any_over_expr, is_dunder};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_trivia::{has_leading_content, has_trailing_content, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::lint::checker::Checker;

pub fn is_alone_on_lines(source: &str, range: TextRange) -> bool {
    !has_leading_content(range.start(), source) && !has_trailing_content(range.end(), source)
}

pub fn delete_statement(source: &str, stmt: &Stmt, body: &[Stmt]) -> Edit {
    if body.len() == 1 {
        return Edit::range_replacement("pass".to_owned(), stmt.range());
    }
    Edit::range_deletion(source.full_lines_range(stmt.range()))
}

// The text deleting a statement removes: its lines when it has them to itself, with the blank lines after them when
// `trailing_blanks` is set. A statement sharing its line goes from the `;` that joins it to the statement before it, as
// in `run(); x = 1`, or up to the `next` statement, as in `"""Doc."""; run()`, unless a comment in between would go
// with it; `ruff format` puts joined statements on lines of their own either way.
pub fn statement_deletion(
    checker: &Checker,
    range: TextRange,
    next: Option<TextRange>,
    trailing_blanks: bool,
) -> Option<TextRange> {
    let source = checker.source;
    if is_alone_on_lines(source, range) {
        return Some(if trailing_blanks {
            lines_with_trailing_blanks(source, range)
        } else {
            source.full_lines_range(range)
        });
    }
    let line_start = source.line_start(range.start());
    let before = source[TextRange::new(line_start, range.start())].trim_end();
    let after = &source[TextRange::new(range.end(), source.line_end(range.end()))];
    if before.ends_with(';') && after.trim().is_empty() {
        return Some(TextRange::new(line_start + before.text_len() - TextSize::from(1), range.end()));
    }
    let up_to_next = TextRange::new(range.start(), next?.start());
    (!checker.has_comment_in(up_to_next)).then_some(up_to_next)
}

pub fn lines_with_trailing_blanks(source: &str, range: TextRange) -> TextRange {
    let mut end = source.full_line_end(range.end());
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
    }
    let mut start = source.line_start(range.start());
    // At the end of the file the blank lines above go too: after a module left with comments only, `ruff format` keeps
    // them, and Ruff reports blank lines ending the file.
    if end.to_usize() == source.len() {
        while start > TextSize::default() && source.full_line_str(start - TextSize::from(1)).trim().is_empty() {
            start = source.line_start(start - TextSize::from(1));
        }
    }
    TextRange::new(start, end)
}

pub fn for_each_body<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    walk_bodies(body, None, visit);
}

fn walk_bodies<'a>(body: &'a [Stmt], owner: Option<&'a Stmt>, visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    visit(body, owner);
    for stmt in body {
        for_each_child_body(stmt, &mut |child| walk_bodies(child, Some(stmt), visit));
    }
}

pub fn for_each_child_body<'a>(stmt: &'a Stmt, visit: &mut impl FnMut(&'a [Stmt])) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { body, .. })
        | Stmt::ClassDef(ast::StmtClassDef { body, .. })
        | Stmt::With(ast::StmtWith { body, .. }) => visit(body),
        Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
            visit(body);
            visit(orelse);
        }
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            visit(body);
            for clause in elif_else_clauses {
                visit(&clause.body);
            }
        }
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
            visit(body);
            for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                visit(&handler.body);
            }
            visit(orelse);
            visit(finalbody);
        }
        Stmt::Match(ast::StmtMatch { cases, .. }) => {
            for case in cases {
                visit(&case.body);
            }
        }
        _ => {}
    }
}

pub fn docstring(body: &[Stmt]) -> Option<&ast::ExprStringLiteral> {
    match body.first()? {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_string_literal_expr(),
        _ => None,
    }
}

pub fn is_main_guard(stmt: &Stmt) -> bool {
    let Stmt::If(ast::StmtIf { test, .. }) = stmt else {
        return false;
    };
    let Expr::Compare(ast::ExprCompare { ops, operands, .. }) = test.as_ref() else {
        return false;
    };
    let is_name = |expr: &Expr| expr.as_name_expr().is_some_and(|name| name.id.as_str() == "__name__");
    let is_main =
        |expr: &Expr| expr.as_string_literal_expr().is_some_and(|literal| literal.value.to_str() == "__main__");
    match (&**ops, &**operands) {
        ([ast::CmpOp::Eq], [left, right]) => (is_name(left) && is_main(right)) || (is_main(left) && is_name(right)),
        _ => false,
    }
}

// The leading whitespace of the line a node starts on; a node after a `;` shares the line, not its indentation.
pub fn indentation(source: &str, ranged: impl Ranged) -> &str {
    leading_indentation(source.full_line_str(ranged.start()))
}

pub fn is_class_like(name: &str) -> bool {
    let trimmed = name.trim_start_matches('_');
    trimmed.chars().next().is_some_and(char::is_uppercase) && trimmed.chars().any(char::is_lowercase)
}

pub fn expr_mentions(expr: &Expr, name: &str) -> bool {
    any_over_expr(expr, |expr| expr.as_name_expr().is_some_and(|found| found.id.as_str() == name))
}

// Each expression `pick` maps to a value, in the order a `Visitor` walks them.
pub struct ExprCollector<F, T> {
    pick: F,
    pub found: Vec<T>,
}

impl<F, T> ExprCollector<F, T> {
    pub fn new<'a>(pick: F) -> Self
    where
        F: FnMut(&'a Expr) -> Option<T>,
    {
        Self { pick, found: Vec::new() }
    }
}

impl<'a, F: FnMut(&'a Expr) -> Option<T>, T> Visitor<'a> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(value) = (self.pick)(expr) {
            self.found.push(value);
        }
        visitor::walk_expr(self, expr);
    }
}

// What a name's leading underscores say about who may use it. The two readings of "public" differ on dunders:
// `from module import *` without `__all__` skips every name with a leading underscore, dunders included, while the
// typing spec and PEP 8 treat only a non-dunder underscore name as internal, so `__version__` is public.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility {
    Public,
    Dunder,
    Private,
}

impl Visibility {
    pub(crate) fn of(name: &str) -> Self {
        if !name.starts_with('_') {
            Self::Public
        } else if is_dunder(name) {
            Self::Dunder
        } else {
            Self::Private
        }
    }
}

// The name an expression such as `typing.Final` or `@name.setter` ends with.
pub fn last_segment(expr: &Expr) -> Option<&str> {
    UnqualifiedName::from_expr(expr)?.segments().last().copied()
}

// A module-level `if sys.version_info < ...: raise ...` or `sys.exit(...)` exists to run on interpreters older than the
// target and report them, so fixes must not add syntax or imports those
// interpreters lack.
pub fn has_version_gate(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_if_stmt).any(|gate| {
        any_over_expr(&gate.test, &|expr: &Expr| last_segment(expr) == Some("version_info"))
            && gate.body.last().is_some_and(|last| match last {
                Stmt::Raise(_) => true,
                Stmt::Expr(ast::StmtExpr { value, .. }) => {
                    value.as_call_expr().is_some_and(|call| last_segment(&call.func) == Some("exit"))
                }
                _ => false,
            })
    })
}

// Ruff ranks names isort-style: constants in SCREAMING_SNAKE_CASE first, then CamelCase classes, then the rest. The two
// orders differ in the details: RUF022, sorting `__all__`, takes any name with a letter and no lowercase one for a
// constant and compares case-sensitively, while isort, sorting the names of a `from` import, wants more than one
// character for a constant and ignores case.
pub fn export_key(name: &str) -> (u8, String) {
    let rank = if name.chars().any(char::is_alphabetic) && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_owned())
}

pub fn isort_key(name: &str) -> (u8, String) {
    let rank = if name.len() > 1 && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_ascii_lowercase())
}

// `name = value` or `name: annotation = value`, binding a single plain name.
pub fn single_assignment(stmt: &Stmt) -> Option<(&ast::ExprName, Option<&Expr>, &Expr)> {
    let (target, annotation, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], None, value),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. }) => {
            (target.as_ref(), Some(annotation.as_ref()), value)
        }
        _ => return None,
    };
    Some((target.as_name_expr()?, annotation, value))
}

// The statements that run when the module loads, including those of its top-level `if` and `try` blocks, where
// modules pick an implementation by version or availability.
pub fn module_statements(body: &[Stmt]) -> impl Iterator<Item = &Stmt> {
    body.iter().flat_map(|stmt| -> Box<dyn Iterator<Item = &Stmt> + '_> {
        match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => Box::new(
                module_statements(body)
                    .chain(elif_else_clauses.iter().flat_map(|clause| module_statements(&clause.body))),
            ),
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => Box::new(
                module_statements(body)
                    .chain(handlers.iter().flat_map(|handler| match handler {
                        ast::ExceptHandler::ExceptHandler(handler) => module_statements(&handler.body),
                    }))
                    .chain(module_statements(orelse))
                    .chain(module_statements(finalbody)),
            ),
            _ => Box::new(std::iter::once(stmt)),
        }
    })
}
