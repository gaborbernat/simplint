use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::exception_classes::{derives_from_base_exception, lineage};

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    let ast::ExceptHandler::ExceptHandler(handler) = handler;
    let Some(name) = handler.name.as_ref() else {
        return;
    };
    let uses = Uses::of(name.as_str(), &handler.body);
    if uses.reraised || uses.reads > 0 {
        return;
    }
    report(walker.checker, &uses.notes, name.as_str());
}

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return;
    };
    let ([Expr::Name(target)], Expr::Call(call)) = (targets.as_slice(), value.as_ref()) else {
        return;
    };
    let checker = walker.checker;
    let name = target.id.as_str();
    // Most assignments build no exception, and a name the file never calls `add_note` on needs no class lookup.
    let noted = checker.state().attribute_reads(checker).get(name).is_some_and(|reads| reads.contains(&"add_note"));
    if !noted || !lineage(checker, &call.func).is_some_and(|lineage| derives_from_base_exception(&lineage)) {
        return;
    }
    // A module-level name may be read by another module, so only a function's own local is followed.
    let Some(function) = walker.scopes().iter().rev().find_map(|scope| match scope {
        Scope::Function(function) => Some(*function),
        _ => None,
    }) else {
        return;
    };
    let uses = Uses::of(name, &function.body);
    if Assignments::count(name, &function.body) != 1 || uses.reads > 0 || uses.declared {
        return;
    }
    report(checker, &uses.notes, name);
}

fn report(checker: &Checker, notes: &[TextRange], name: &str) {
    let Some(first) = notes.first() else {
        return;
    };
    checker.report(
        Rule::DroppedExceptionNote,
        *first,
        format!("The note is added to `{name}`, which is then dropped; raise `{name}` or leave the note out"),
    );
}

// How a block uses one name: the `add_note` calls on it, the other reads of it, a bare `raise` that re-raises the
// exception being handled, and a `global` or `nonlocal` declaration that hands it to another scope.
#[derive(Default)]
struct Uses<'n> {
    name: &'n str,
    notes: Vec<TextRange>,
    reads: usize,
    reraised: bool,
    declared: bool,
}

impl<'n> Uses<'n> {
    fn of(name: &'n str, body: &[Stmt]) -> Self {
        let mut uses = Self { name, ..Self::default() };
        uses.visit_body(body);
        uses
    }
}

impl<'a> Visitor<'a> for Uses<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt
            && let Expr::Call(call) = value.as_ref()
            && is_note_on(call, self.name)
        {
            self.notes.push(value.range());
            self.visit_arguments(&call.arguments);
            return;
        }
        match stmt {
            Stmt::Raise(ast::StmtRaise { exc: None, .. }) => self.reraised = true,
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.declared |= names.iter().any(|declared| declared.as_str() == self.name);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr
            && name.id.as_str() == self.name
            && matches!(name.ctx, ExprContext::Load | ExprContext::Del)
        {
            self.reads += 1;
        }
        visitor::walk_expr(self, expr);
    }
}

fn is_note_on(call: &ast::ExprCall, name: &str) -> bool {
    call.func.as_attribute_expr().is_some_and(|attribute| {
        attribute.attr.as_str() == "add_note" && attribute.value.as_name_expr().is_some_and(|value| value.id == name)
    })
}

struct Assignments<'n> {
    name: &'n str,
    count: usize,
}

impl<'n> Assignments<'n> {
    fn count(name: &'n str, body: &[Stmt]) -> usize {
        let mut assignments = Self { name, count: 0 };
        assignments.visit_body(body);
        assignments.count
    }
}

impl<'a> Visitor<'a> for Assignments<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr
            && name.id.as_str() == self.name
            && matches!(name.ctx, ExprContext::Store)
        {
            self.count += 1;
        }
        visitor::walk_expr(self, expr);
    }
}
