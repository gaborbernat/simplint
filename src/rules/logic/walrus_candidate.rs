use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::layout::joined_width;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::statement_deletion;
use crate::rules::logic::inline::{declared_outer, fits_line, for_each_scoped_body, inlined_text, leading_name};
use crate::rules::{expr_mentions, has_version_gate, needs_parentheses};

pub fn walrus_candidate(checker: &Checker) {
    // `:=` arrived in Python 3.8.
    if checker.settings.from_ruff.target_version < PythonVersion::PY38 || has_version_gate(checker.body()) {
        return;
    }
    for_each_scoped_body(checker.body(), Scope::Module, 0, &enter_scope, &mut |body, scope, depth| {
        for pair in body.windows(2) {
            if let [Stmt::Assign(assign), if_stmt @ Stmt::If(ast::StmtIf { test, .. })] = pair
                && let Some(candidate) = candidate(checker, scope, assign, if_stmt, test)
            {
                report(checker, &candidate, depth);
            }
        }
    });
}

// The innermost `def` or `class` around a body.
#[derive(Clone, Copy)]
enum Scope<'a> {
    Module,
    Function(&'a Stmt),
    // An assignment here defines a class attribute, which `Class.name` reads without a name reference.
    Class,
}

fn enter_scope<'a>(scope: Scope<'a>, stmt: &'a Stmt) -> Scope<'a> {
    match stmt {
        Stmt::FunctionDef(_) => Scope::Function(stmt),
        Stmt::ClassDef(_) => Scope::Class,
        _ => scope,
    }
}

fn report(checker: &Checker, candidate: &Candidate, depth: usize) {
    let &Candidate { assign, deleted, name, target, bare, single_use, .. } = candidate;
    let value_range = checker.parenthesized_range(&assign.value, assign.into());
    let value = checker.text(value_range);
    let replacement = match (single_use, bare) {
        (true, _) => inlined_text(checker, &assign.value, value_range, !bare && needs_parentheses(&assign.value)),
        (false, true) => format!("{name} := {value}"),
        (false, false) => format!("({name} := {value})"),
    };
    let wrapping = replacement.chars().count() - value.chars().count();
    if !header_fits(checker, candidate, value_range, wrapping, depth) {
        return;
    }
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

// The formatter splits an `if` header wider than its line width over several lines, which reads worse than the
// assignment and the `if` the fix joins. The width is judged on `if <test>:` joined on one line at its block's
// indentation, without parentheses around the test, which the formatter drops from a header that fits, so the answer
// does not depend on how the code is laid out now.
fn header_fits(
    checker: &Checker,
    candidate: &Candidate,
    value_range: TextRange,
    wrapping: usize,
    depth: usize,
) -> bool {
    let (source, tokens) = (checker.source, checker.parsed.tokens());
    let (Some(test), Some(value)) =
        (joined_width(source, tokens, candidate.test.range()), joined_width(source, tokens, value_range))
    else {
        return false;
    };
    let header = "if ".len() + test + ":".len();
    fits_line(checker, depth, header, candidate.name.chars().count(), value, wrapping)
}

struct Candidate<'a> {
    assign: &'a ast::StmtAssign,
    test: &'a Expr,
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
            function.as_function_def_stmt().is_some_and(|function| declared_outer(&function.body).contains(name)),
        ),
        Scope::Module => (
            checker.body(),
            TextRange::up_to(checker.source.text_len()),
            checker.project().is_referenced(name) || checker.dunder_all().may_list(name),
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
        test,
        deleted,
        name,
        target: first.range,
        bare,
        single_use: !body_mentions(std::slice::from_ref(if_stmt), name, TextRange::new(first.end(), if_stmt.end())),
    })
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
