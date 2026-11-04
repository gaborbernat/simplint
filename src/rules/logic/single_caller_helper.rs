use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::forwarding_function::forward_target;
use crate::syntax::{Visibility, is_repeating_expr};

pub fn single_caller_helper(checker: &Checker) {
    for stmt in checker.body() {
        if let Stmt::FunctionDef(helper) = stmt {
            check_helper(checker, helper);
        }
    }
}

fn check_helper(checker: &Checker, helper: &ast::StmtFunctionDef) {
    let name = helper.name.as_str();
    if Visibility::of(name) != Visibility::Private
        || !helper.decorator_list.is_empty()
        || helper.is_async
        || yields(&helper.body)
        || checker.project().is_referenced(name)
        || checker.dunder_all().may_list(name)
    {
        return;
    }
    // The helper's one reference must be the call, so a reference that is only passed on fails here.
    let [reference] = checker.semantic().references_to(name) else {
        return;
    };
    if !checker.source[reference.range.end().to_usize()..].trim_start().starts_with('(') {
        return;
    }
    // A helper that only forwards its arguments is forwarding-function's (SMP501), which asks to call the target instead.
    if forward_target(checker, helper, &[reference.range]).is_some() {
        return;
    }
    let Some(caller) = reference
        .function
        .and_then(|range| checker.function_defs().iter().copied().find(|function| function.range == range))
    else {
        return;
    };
    // A recursive helper calls itself, which inlining cannot remove.
    if caller.range == helper.range || in_repeat(&caller.body, reference.range) {
        return;
    }
    let knobs = &checker.settings.project.knobs;
    let helper_statements = body_without_leading_docstring(&helper.body).len();
    let caller_statements = body_without_leading_docstring(&caller.body).len();
    if helper_statements >= knobs.max_helper_statements
        || caller_statements + helper_statements > knobs.max_caller_statements
    {
        return;
    }
    let caller_name = caller.name.as_str();
    checker.report(
        Rule::SingleCallerHelper,
        helper.name.range(),
        format!("`{name}` is called only from `{caller_name}`, and inlining keeps `{caller_name}` short; inline it"),
    );
}

// A generator's body runs only when it is iterated, so inlining it into a caller would make the caller a generator.
fn yields(body: &[Stmt]) -> bool {
    let mut finder = Yields { found: false };
    finder.visit_body(body);
    finder.found
}

struct Yields {
    found: bool,
}

impl<'a> Visitor<'a> for Yields {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            visitor::walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Yield(_) | Expr::YieldFrom(_) => self.found = true,
            Expr::Lambda(_) => return,
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// Whether the call sits in a loop, comprehension or lambda of its caller, where inlining would run the helper's body
// once per pass.
fn in_repeat(body: &[Stmt], reference: TextRange) -> bool {
    let mut finder = Repeats { reference, depth: 0, repeated: false };
    finder.visit_body(body);
    finder.repeated
}

struct Repeats {
    reference: TextRange,
    depth: usize,
    repeated: bool,
}

impl<'a> Visitor<'a> for Repeats {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let repeats = matches!(stmt, Stmt::For(_) | Stmt::While(_));
        self.depth += usize::from(repeats);
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(repeats);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.range() == self.reference {
            self.repeated = self.depth > 0;
        }
        let repeats = is_repeating_expr(expr);
        self.depth += usize::from(repeats);
        visitor::walk_expr(self, expr);
        self.depth -= usize::from(repeats);
    }
}
