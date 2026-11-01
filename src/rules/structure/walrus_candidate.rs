use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::Exports;
use crate::rules::structure::needs_parentheses;
use crate::rules::{expr_mentions, for_each_body, for_each_child_body, statement_deletion};

pub fn walrus_candidate(checker: &Checker) {
    // `:=` arrived in Python 3.8.
    if checker.settings.from_ruff.target_version < PythonVersion::PY38 {
        return;
    }
    check_body(checker, checker.body(), Scope::Module);
}

// The innermost `def` or `class` around a body.
#[derive(Clone, Copy)]
enum Scope<'a> {
    Module,
    Function(&'a Stmt),
    // An assignment here defines a class attribute, which `Class.name` reads without a name reference.
    Class,
}

fn check_body<'a>(checker: &Checker<'a>, body: &'a [Stmt], scope: Scope<'a>) {
    for pair in body.windows(2) {
        if let [Stmt::Assign(assign), if_stmt @ Stmt::If(ast::StmtIf { test, .. })] = pair
            && let Some(candidate) = candidate(checker, scope, assign, if_stmt, test)
        {
            report(checker, &candidate);
        }
    }
    for stmt in body {
        let inner = match stmt {
            Stmt::FunctionDef(_) => Scope::Function(stmt),
            Stmt::ClassDef(_) => Scope::Class,
            _ => scope,
        };
        for_each_child_body(stmt, &mut |child| check_body(checker, child, inner));
    }
}

fn report(checker: &Checker, candidate: &Candidate) {
    let &Candidate { assign, deleted, name, target, bare, single_use } = candidate;
    let value = checker.parenthesized_text(&assign.value, assign.into());
    let replacement = match (single_use, bare) {
        (true, false) if needs_parentheses(&assign.value) && value == checker.text(assign.value.range()) => {
            format!("({value})")
        }
        (true, _) => value.to_owned(),
        (false, true) => format!("{name} := {value}"),
        (false, false) => format!("({name} := {value})"),
    };
    let (message, title) = if single_use {
        (format!("Inline single-use `{name}` into the `if` test"), format!("Inline `{name}` into the `if` test"))
    } else {
        (
            format!("Assign `{name}` with `:=` inside the `if` test that consumes it"),
            format!("Assign `{name}` with `:=` in the `if` test"),
        )
    };
    let deletion = Edit::range_deletion(deleted);
    checker.report_fix(
        Rule::WalrusCandidate,
        assign.range,
        message,
        Some(Fix::safe_edits(deletion, [Edit::range_replacement(replacement, target)])),
        title,
    );
}

struct Candidate<'a> {
    assign: &'a ast::StmtAssign,
    deleted: TextRange,
    name: &'a str,
    target: TextRange,
    bare: bool,
    single_use: bool,
}

fn candidate<'a>(
    checker: &Checker<'a>,
    scope: Scope<'a>,
    assign: &'a ast::StmtAssign,
    if_stmt: &'a Stmt,
    test: &'a Expr,
) -> Option<Candidate<'a>> {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return None;
    };
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    let deleted = statement_deletion(checker, assign.range, None, false)?;
    // Type checkers treat a guard named `TYPE_CHECKING` as true whatever its value, so the name must stay.
    if name == "TYPE_CHECKING"
        || matches!(value, Expr::Lambda(_) | Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Named(_) | Expr::Starred(_))
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
        || expr_mentions(value, name)
        || checker.has_comment_in(TextRange::new(assign.start(), if_stmt.start()))
    {
        return None;
    }
    let (first, bare) = leading_name(test, true)?;
    if first.id.as_str() != name {
        return None;
    }
    // Inlining deletes the binding, so any other reader of it, earlier in the scope (a closure, the previous loop
    // iteration), in another module, or through `global`/`nonlocal`, would lose it.
    let (scope_body, scope_range, shared) = match scope {
        Scope::Class => return None,
        Scope::Function(function) => (
            std::slice::from_ref(function),
            function.range(),
            function.as_function_def_stmt().is_some_and(|function| declares_outer(&function.body, name)),
        ),
        Scope::Module => (
            checker.body(),
            TextRange::up_to(checker.source.text_len()),
            checker.project().is_referenced(name)
                || match checker.dunder_all() {
                    Exports::Listed(names) => names.contains(&name),
                    Exports::Dynamic => true,
                    Exports::Absent => false,
                },
        ),
    };
    if shared
        || body_mentions(scope_body, name, TextRange::new(scope_range.start(), assign.start()))
        || body_mentions(scope_body, name, TextRange::new(if_stmt.end(), scope_range.end()))
    {
        return None;
    }
    Some(Candidate {
        assign,
        deleted,
        name,
        target: first.range,
        bare,
        single_use: !body_mentions(std::slice::from_ref(if_stmt), name, TextRange::new(first.end(), if_stmt.end())),
    })
}

fn declares_outer(body: &[Stmt], name: &str) -> bool {
    let mut found = false;
    for_each_body(body, &mut |statements, _| {
        found |= statements.iter().any(|stmt| match stmt {
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                names.iter().any(|declared| declared.as_str() == name)
            }
            _ => false,
        });
    });
    found
}

fn leading_name(test: &Expr, bare: bool) -> Option<(&ast::ExprName, bool)> {
    match test {
        Expr::Name(name) => Some((name, bare)),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => leading_name(operand, false),
        Expr::Compare(ast::ExprCompare { operands, .. }) => leading_name(operands.first()?, false),
        Expr::BoolOp(ast::ExprBoolOp { values, .. }) => leading_name(values.first()?, false),
        _ => None,
    }
}

// Whether `body` holds `name` anywhere inside `within`.
fn body_mentions(body: &[Stmt], name: &str, within: TextRange) -> bool {
    let mut finder = NameFinder { name, within, found: false };
    finder.visit_body(body);
    finder.found
}

struct NameFinder<'n> {
    name: &'n str,
    within: TextRange,
    found: bool,
}

impl NameFinder<'_> {
    // A node can only hold a name inside `within` when the two overlap.
    fn skips(&self, node: TextRange) -> bool {
        self.found || node.end() <= self.within.start() || node.start() >= self.within.end()
    }
}

impl<'a> Visitor<'a> for NameFinder<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if !self.skips(stmt.range()) {
            visitor::walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.skips(expr.range()) {
            return;
        }
        if let Expr::Name(name) = expr
            && name.id.as_str() == self.name
            && self.within.contains_range(name.range)
        {
            self.found = true;
        }
        visitor::walk_expr(self, expr);
    }
}
