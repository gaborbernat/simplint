use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::flow::has_own_jump;
use crate::rules::logic::name_uses::FunctionNames;
use crate::rules::logic::inline::{for_each_scoped_body, is_constant};

// The function a block belongs to, and whether a `try` or `with` of that function holds the block. There an exception
// raised between two assignments may go on to a handler, a `finally` or past the `with`, where the first value is read.
#[derive(Clone, Copy)]
struct Place<'a> {
    function: Option<&'a ast::StmtFunctionDef>,
    guarded: bool,
}

pub fn dead_store(checker: &Checker) {
    let scopes = checker.state().local_scopes(checker);
    let start = Place { function: None, guarded: false };
    for_each_scoped_body(checker.body(), start, 0, &enter, &mut |body, place, _| {
        let Some(function) = place.function.filter(|_| !place.guarded) else {
            return;
        };
        for (index, stmt) in body.iter().enumerate() {
            let Some(first) = store(stmt) else {
                continue;
            };
            let names = scopes.function(function).names();
            check_store(checker, names, &body[index + 1..], stmt, first);
        }
    });
}

fn enter<'a>(place: Place<'a>, stmt: &'a Stmt) -> Place<'a> {
    match stmt {
        Stmt::FunctionDef(definition) => Place { function: Some(definition), guarded: false },
        Stmt::ClassDef(_) => Place { function: None, guarded: false },
        Stmt::Try(_) | Stmt::With(_) => Place { guarded: true, ..place },
        _ => place,
    }
}

// The single name a statement assigns, with the value it assigns.
fn store(stmt: &Stmt) -> Option<(&ast::ExprName, &Expr)> {
    let (target, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            let [target] = targets.as_slice() else {
                return None;
            };
            (target, value.as_ref())
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => (target.as_ref(), value.as_ref()),
        _ => return None,
    };
    let target = target.as_name_expr()?;
    let name = target.id.as_str();
    (name != "_" && !is_dunder(name)).then_some((target, value))
}

fn check_store(
    checker: &Checker,
    names: &FunctionNames,
    rest: &[Stmt],
    stmt: &Stmt,
    (target, first): (&ast::ExprName, &Expr),
) {
    let name = target.id.as_str();
    if !names.is_plain_local(name) {
        return;
    }
    let Some((mention, context)) = names.mentions(name).find(|(range, _)| range.start() >= stmt.end()) else {
        return;
    };
    // The next mention must be the target of a later assignment in this block, whose value does not read the name.
    let Some(position) = rest.iter().position(|later| later.range().contains_range(mention)) else {
        return;
    };
    let later = &rest[position];
    let Some((next, value)) = store(later) else {
        return;
    };
    if next.range != mention
        || context != ExprContext::Store
        || names.mentions(name).any(|(range, _)| value.range().contains_range(range))
    {
        return;
    }
    // `None` over a built value lets the value go before the function ends, often to free its memory.
    if value.is_none_literal_expr() && !is_constant(first) {
        return;
    }
    // A `break` or `continue` between them goes back to the loop, which may read the first value.
    if has_own_jump(&rest[..position]) {
        return;
    }
    let line = checker.line_number(later.start());
    checker.report_fix(
        Rule::DeadStore,
        stmt.range(),
        format!("The value assigned to `{name}` is replaced on line {line} before anything reads it"),
        removal(checker, stmt),
        "Remove the value that is never read",
    );
}

// A constant has no effect beyond the name it binds, so the assignment goes, and an annotated one keeps its annotation.
// Any other value may do something when it is built, so it is left in place.
fn removal(checker: &Checker, stmt: &Stmt) -> Option<Fix> {
    if !checker.is_alone_on_lines(stmt.range()) || checker.has_comment_in(stmt.range()) {
        return None;
    }
    match stmt {
        Stmt::Assign(ast::StmtAssign { value, .. }) if is_constant(value) => {
            Some(Fix::safe_edit(Edit::range_deletion(checker.source.full_lines_range(stmt.range()))))
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, value: Some(value), target, .. }) if is_constant(value) => {
            let declaration = format!("{}: {}", checker.text(target.range()), checker.text(annotation.range()));
            Some(Fix::safe_edit(Edit::range_replacement(declaration, stmt.range())))
        }
        _ => None,
    }
}
