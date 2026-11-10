use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Operator, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::delete_statement;
use crate::rules::is_alone_on_lines;
use crate::semantic::future_import;
use crate::syntax::{alias_binding, module_statements, target_names};

pub fn future_annotations(checker: &Checker) {
    let body = checker.body();
    for stmt in body {
        let Some(import) = future_import(stmt) else {
            continue;
        };
        let Some(alias) = import.names.iter().find(|alias| alias.name.as_str() == "annotations") else {
            continue;
        };
        // Lazy annotations of a class read its namespace, so a name the class binds changes what they refer to. Only
        // the import keeps them resolving in the module globals.
        if class_shadows_annotation(body) {
            return;
        }
        // The edit rewrites or deletes the whole line, so a comment on it would go with the import.
        let edit = if checker.has_comment_in(checker.source.full_lines_range(stmt.range())) {
            None
        } else if import.names.len() > 1 {
            let kept: Vec<&str> = import
                .names
                .iter()
                .filter(|other| other.name.as_str() != "annotations")
                .map(|other| checker.text(other.range()))
                .collect();
            Some(Edit::range_replacement(format!("from __future__ import {}", kept.join(", ")), import.range()))
        } else {
            is_alone_on_lines(checker.source, stmt.range()).then(|| delete_statement(checker.source, stmt, body))
        };
        // Lazy annotations still evaluate on access, so a name imported only for type checking raises there, and so
        // does a quoted union or an import under an `if` that may be false.
        let applicability = if checker.name_token_count("TYPE_CHECKING") > 0
            || has_partly_quoted_union(body)
            || has_guarded_import(body)
        {
            Applicability::Unsafe
        } else {
            Applicability::Safe
        };
        checker.report_fix(
            Rule::FutureAnnotations,
            alias.range(),
            "Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`",
            edit.map(|edit| Fix::applicable_edit(edit, applicability)),
            "Remove the `annotations` future import",
        );
    }
}

// Evaluated, `"Node" | None` applies `|` to a str and raises, as `@dataclass` does when it reads the annotations.
fn has_partly_quoted_union(body: &[Stmt]) -> bool {
    let mut finder = QuotedUnion(false);
    finder.visit_body(body);
    finder.0
}

struct QuotedUnion(bool);

impl<'a> Visitor<'a> for QuotedUnion {
    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.0 = self.0 || any_over_expr(expr, &is_quoted_union);
    }
}

fn is_quoted_union(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::BitOr, right, .. })
            if left.is_string_literal_expr() || right.is_string_literal_expr()
    )
}

// A module-level `if` such as `if MYPY:` may skip the import, so the annotations read a name that is never bound.
fn has_guarded_import(body: &[Stmt]) -> bool {
    body.iter().filter(|stmt| stmt.is_if_stmt()).any(|guard| {
        module_statements(std::slice::from_ref(guard)).any(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)))
    })
}

// Whether a class binds a name that one of its annotations reads, such as `def list(self) -> list[str]`.
fn class_shadows_annotation(body: &[Stmt]) -> bool {
    let mut finder = ShadowingClass(false);
    finder.visit_body(body);
    finder.0
}

struct ShadowingClass(bool);

impl<'a> Visitor<'a> for ShadowingClass {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ClassDef(class) = stmt {
            self.0 |= shadows(&class.body);
        }
        visitor::walk_stmt(self, stmt);
    }
}

fn shadows(body: &[Stmt]) -> bool {
    let statements: Vec<&Stmt> = module_statements(body).collect();
    let bound: FxHashSet<&str> = statements.iter().flat_map(|stmt| bound_names(stmt)).collect();
    statements.iter().flat_map(|stmt| annotations(stmt)).any(|annotation| {
        let mut reads = Reads(Vec::new());
        reads.visit_expr(annotation);
        reads.0.iter().any(|name| bound.contains(name))
    })
}

// The names a class body statement binds in the class namespace. An annotation without a value binds nothing.
fn bound_names(stmt: &Stmt) -> Vec<&str> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            vec![name.as_str()]
        }
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().flat_map(target_names).collect(),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(_), .. })
        | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => target_names(target),
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().map(alias_binding).collect()
        }
        _ => Vec::new(),
    }
}

// The annotations of a method's parameters and return type, and of an annotated class field.
fn annotations(stmt: &Stmt) -> Vec<&Expr> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { parameters, returns, .. }) => {
            parameters.iter().filter_map(AnyParameterRef::annotation).chain(returns.as_deref()).collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => vec![annotation],
        _ => Vec::new(),
    }
}

struct Reads<'a>(Vec<&'a str>);

impl<'a> Visitor<'a> for Reads<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(ast::ExprName { id, .. }) = expr {
            self.0.push(id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}
