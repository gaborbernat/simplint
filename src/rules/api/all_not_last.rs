use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_body;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::lines_with_trailing_blanks;
use crate::rules::is_alone_on_lines;
use crate::syntax::is_main_guard;
use crate::syntax::{all_binding, all_mutation, is_name};

pub fn all_not_last(checker: &Checker) {
    let body = checker.body();
    let positions: Vec<usize> =
        body.iter().enumerate().filter(|(_, stmt)| all_binding(stmt).is_some()).map(|(index, _)| index).collect();
    let Some(&last) = positions.last() else {
        return;
    };
    let end = body.len() - usize::from(body.last().is_some_and(is_main_guard));
    // Calls on `__all__` such as `__all__.extend(...)` change the list the binding made, so they stay with it.
    if body[last + 1..end].iter().all(|stmt| all_mutation(stmt).is_some()) {
        return;
    }
    let stmt = &body[last];
    let source = checker.source;
    // A later `__all__ += [...]` inside a block extends the list, so it must still find it bound above.
    let semantic = checker.semantic();
    // A module that reads `__all__` at run time, by attribute, string or `globals()`, needs the list bound above it, and
    // so does a function or lambda that reads it, by name or at run time, wherever it is defined.
    let referenced_later = semantic
        .references_to("__all__")
        .iter()
        .any(|reference| reference.statement > last || reference.function.is_some())
        || semantic.bindings("__all__").iter().any(|binding| binding.range.start() > stmt.end())
        || any_over_body(&body[last + 1..], reads_all_dynamically)
        || scope_reads_dynamically(body);
    let after = source.full_line_end(stmt.end());
    // A `# fmt: off` / `# fmt: on` pair around `__all__` keeps `ruff format` off its layout, which can hold a
    // suppression comment for several names on one line, so the pair moves with it.
    let above = checker.comment_above(stmt);
    let fence = above
        .filter(|comment| source[*comment].trim() == "# fmt: off" && source.full_line_str(after).trim() == "# fmt: on");
    // Any other comment above `__all__`, or above its pair, would stay on the line above the statement that follows.
    let fix = (positions.len() == 1
        && !referenced_later
        && is_string_list(stmt)
        && is_alone_on_lines(source, stmt.range())
        && (above.is_none() || fence.is_some_and(|off| checker.comment_above(off).is_none())))
    .then(|| {
        let lines =
            lines_with_trailing_blanks(source, fence.map_or(stmt.range(), |off| TextRange::new(off.start(), after)));
        let insertion = source.full_line_end(body[end - 1].end());
        let text = source[lines].trim_end();
        let eol = checker.line_ending();
        let content = if insertion.to_usize() == source.len() && !source.ends_with('\n') {
            format!("{eol}{eol}{eol}{text}")
        } else {
            format!("{eol}{eol}{text}{eol}")
        };
        Fix::safe_edits(Edit::range_deletion(lines), [Edit::insertion(content, insertion)])
    });
    checker.report_fix(
        Rule::AllNotLast,
        stmt.range(),
        "`__all__` is not the last statement; move it to the end of the module",
        fix,
        "Move `__all__` to the end of the module",
    );
}

// A list or tuple of names, which keeps its value when moved; a computed value may depend on where it runs.
fn is_string_list(stmt: &Stmt) -> bool {
    match all_binding(stmt).and_then(|binding| binding.value) {
        Some(Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) => {
            elts.iter().all(Expr::is_string_literal_expr)
        }
        _ => false,
    }
}

fn scope_reads_dynamically(body: &[Stmt]) -> bool {
    let mut reads = ScopeReads::default();
    reads.visit_body(body);
    reads.found
}

// A function or lambda body runs when it is called, so a dynamic read in one counts wherever the scope is defined.
#[derive(Default)]
struct ScopeReads {
    depth: usize,
    found: bool,
}

impl<'a> Visitor<'a> for ScopeReads {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                self.depth += 1;
                self.visit_body(&function.body);
                self.depth -= 1;
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.depth > 0 && reads_all_dynamically(expr) {
            self.found = true;
        }
        match expr {
            Expr::Lambda(lambda) => {
                self.depth += 1;
                self.visit_expr(&lambda.body);
                self.depth -= 1;
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

fn reads_all_dynamically(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(ast::ExprAttribute { attr, ctx, .. }) => attr.as_str() == "__all__" && ctx.is_load(),
        Expr::StringLiteral(literal) => literal.value.to_str() == "__all__",
        Expr::Call(ast::ExprCall { func, .. }) => {
            ["globals", "vars", "dir"].into_iter().any(|name| is_name(func, name))
        }
        _ => false,
    }
}
