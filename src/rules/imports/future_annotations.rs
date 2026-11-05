use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{delete_statement, is_alone_on_lines};
use crate::semantic::future_import;
use crate::syntax::{alias_binding, module_statements};

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
        let edit = if import.names.len() > 1 {
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
        // Lazy annotations still evaluate on access, so a name imported only for type checking raises there.
        let applicability =
            if checker.name_token_count("TYPE_CHECKING") > 0 { Applicability::Unsafe } else { Applicability::Safe };
        checker.report_fix(
            Rule::FutureAnnotations,
            alias.range(),
            "Annotations are evaluated lazily from Python 3.14; drop `from __future__ import annotations`",
            edit.map(|edit| Fix::applicable_edit(edit, applicability)),
            "Remove the `annotations` future import",
        );
    }
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

fn target_names(target: &Expr) -> Vec<&str> {
    match target {
        Expr::Name(ast::ExprName { id, .. }) => vec![id.as_str()],
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().flat_map(target_names).collect()
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => target_names(value),
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
