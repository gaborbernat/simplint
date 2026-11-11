use std::cell::OnceCell;

use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::{RaiseStatementVisitor, ReturnStatementVisitor, body_without_leading_docstring};
use ruff_python_ast::statement_visitor::StatementVisitor;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, ExceptHandler, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::cognitive_complexity;
use crate::rules::logic::forwarding_function::{forward_target, is_called};
use crate::rules::testing::{conditionals, is_test_module_function};
use crate::syntax::{Visibility, is_repeating_expr, last_segment};

pub fn single_caller_helper(checker: &Checker<'_>) {
    let defs = OnceCell::new();
    for stmt in checker.body() {
        if let Stmt::FunctionDef(helper) = stmt {
            check_helper(checker, helper, &defs);
        }
    }
}

fn check_helper<'a>(
    checker: &Checker<'a>,
    helper: &ast::StmtFunctionDef,
    defs: &OnceCell<FxHashMap<TextRange, &'a ast::StmtFunctionDef>>,
) {
    let name = helper.name.as_str();
    if Visibility::of(name) != Visibility::Private
        || !helper.decorator_list.is_empty()
        || helper.is_async
        || yields(&helper.body)
        || checker.project().is_referenced(name)
        || checker.dunder_all().may_list(name)
        || helper.returns.as_deref().is_some_and(|returns| is_type_guard(checker, returns))
    {
        return;
    }
    // The helper's one reference must be the call, so a reference that is only passed on fails here.
    let [reference] = checker.semantic().references_to(name) else {
        return;
    };
    if !is_called(checker, reference.range) {
        return;
    }
    // A helper that only forwards its arguments is forwarding-function's (SMP501), which asks to call the target instead.
    if forward_target(checker, helper, &[reference.range], None).is_some() {
        return;
    }
    let defs =
        defs.get_or_init(|| checker.function_defs().iter().map(|function| (function.range, *function)).collect());
    let Some(caller) = reference.function.and_then(|range| defs.get(&range).copied()) else {
        return;
    };
    // A recursive helper calls itself, which inlining cannot remove.
    if caller.range == helper.range {
        return;
    }
    // Inlining a branching helper into a test would make conditional-in-test's (SMP803) report fire there.
    if is_test_module_function(checker, caller) && !conditionals(&helper.body).is_empty() {
        return;
    }
    let knobs = &checker.settings.project.knobs;
    let helper_body = body_without_leading_docstring(&helper.body);
    let helper_statements = helper_body.len();
    let caller_statements = body_without_leading_docstring(&caller.body).len();
    if helper_statements >= knobs.max_helper_statements
        || caller_statements + helper_statements > knobs.max_caller_statements
    {
        return;
    }
    // The loop walk waits for the size limits, so a long caller with many helpers is not walked once per helper.
    if in_repeat(&caller.body, reference.range) {
        return;
    }
    // Inlining at the call site adds the helper's nesting-sensitive increments, so a caller that would pass the limit
    // cognitive-complexity reports stays as it is. A call the walk cannot place is left alone.
    let limit = knobs.max_cognitive_complexity;
    let (caller_score, nesting) = cognitive_complexity::score_probed(&caller.body, reference.range);
    let inlined =
        nesting.map_or(usize::MAX, |nesting| caller_score + cognitive_complexity::score(&helper.body, nesting));
    if inlined > limit || ruff_limit_crossed(checker, &caller.body, helper_body, reference.range) {
        return;
    }
    let caller_name = caller.name.as_str();
    checker.report(
        Rule::SingleCallerHelper,
        helper.name.range(),
        format!("`{name}` is called only from `{caller_name}`, and inlining keeps `{caller_name}` short; inline it"),
    );
}

// Ruff's complexity and size rules count the inlined statements too. The counts are conservative estimates, so a caller
// one of them might report stays as it is. A call in a conditional operand cannot be inlined without a new `if`, so
// the caller stays while any of those rules runs, unless the helper is a lone `return`, which inlines as an expression.
fn ruff_limit_crossed(checker: &Checker, caller: &[Stmt], helper: &[Stmt], reference: TextRange) -> bool {
    let from_ruff = &checker.settings.from_ruff;
    let mut site = CallSite { reference, returned: false, elif_test: false, conditional: false, depth: 0, nesting: 0 };
    site.visit_body(caller);
    let mut tried = TryContext { reference, statements: 0, catches: false, handled: Vec::new(), catch_all: false };
    tried.visit_body(caller);
    let mut helper_nesting = NestedDepth::default();
    helper_nesting.visit_body(helper);
    let mut raises = RaiseStatementVisitor::default();
    raises.visit_body(helper);
    let counted = [
        RuffRule::ComplexStructure,
        RuffRule::TooManyBranches,
        RuffRule::TooManyStatements,
        RuffRule::TooManyReturnStatements,
    ]
    .into_iter()
    .any(|rule| from_ruff.ruff_runs(rule));
    let lone_return = matches!(helper, [Stmt::Return(_)]);
    if site.conditional && counted && !lone_return {
        return true;
    }
    let helper_returns = ruff_returns(helper);
    // Inlining turns an `elif` test into an `else` holding a nested `if`, which adds a branch and a statement.
    let elif = usize::from(site.elif_test && !lone_return);
    // Each early return becomes an `else` around the statements after it, which adds a statement per return.
    let inlined_returns = (2 * helper_returns).saturating_sub(1);
    let caller_returns = ruff_returns(caller);
    let returns = if site.returned { caller_returns + helper_returns - 1 } else { caller_returns };
    (from_ruff.ruff_runs(RuffRule::TooManyNestedBlocks)
        && site.nesting + helper_nesting.maximum > from_ruff.max_nested_blocks)
        || (from_ruff.ruff_runs(RuffRule::TooManyStatementsInTryClause)
            && tried.catches
            && tried.statements + ruff_statements(helper) > from_ruff.max_statements_in_try)
        || (from_ruff.ruff_runs(RuffRule::RaiseWithinTry)
            && tried.catches
            && raises.raises.iter().any(|(_, exception, _)| {
                let exception_name = exception.and_then(|exception| match exception {
                    Expr::Call(call) => last_segment(&call.func),
                    _ => last_segment(exception),
                });
                exception_name
                    .is_some_and(|name| tried.catch_all || tried.handled.iter().any(|handler| catches(handler, name)))
            }))
        || (from_ruff.ruff_runs(RuffRule::ComplexStructure)
            && mccabe_complexity(caller) + mccabe_complexity(helper) + 1 > from_ruff.max_complexity)
        || (from_ruff.ruff_runs(RuffRule::TooManyBranches)
            && ruff_branches(caller) + ruff_branches(helper) + helper_returns.saturating_sub(1) + elif
                > from_ruff.max_branches)
        || (from_ruff.ruff_runs(RuffRule::TooManyStatements)
            && ruff_statements(caller) + ruff_statements(helper) + inlined_returns + elif > from_ruff.max_statements)
        || (from_ruff.ruff_runs(RuffRule::TooManyReturnStatements) && returns > from_ruff.max_returns)
}

// Where the call sits in its caller: as the value of a `return`, in the test of an `elif`, or in a conditional operand.
struct CallSite {
    reference: TextRange,
    returned: bool,
    elif_test: bool,
    conditional: bool,
    depth: usize,
    nesting: usize,
}

impl<'a> Visitor<'a> for CallSite {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Return(ast::StmtReturn { value: Some(value), .. }) => {
                self.returned |=
                    matches!(value.as_ref(), Expr::Call(ast::ExprCall { func, .. }) if func.range() == self.reference);
            }
            Stmt::If(ast::StmtIf { elif_else_clauses, .. }) => {
                self.elif_test |= elif_else_clauses
                    .iter()
                    .any(|clause| clause.test.as_ref().is_some_and(|test| test.range().contains_range(self.reference)));
            }
            _ => {}
        }
        self.depth += usize::from(is_nested_block(stmt));
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(is_nested_block(stmt));
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::If(ast::ExprIf { body, orelse, .. }) => {
                self.conditional |=
                    body.range().contains_range(self.reference) || orelse.range().contains_range(self.reference);
            }
            Expr::BoolOp(ast::ExprBoolOp { values, .. }) => {
                self.conditional |= values.iter().skip(1).any(|value| value.range().contains_range(self.reference));
            }
            _ => {}
        }
        if expr.range() == self.reference {
            self.nesting = self.depth;
        }
        visitor::walk_expr(self, expr);
    }
}

fn is_nested_block(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::If(_) | Stmt::While(_) | Stmt::For(_) | Stmt::Try(_) | Stmt::With(_))
}

#[derive(Default)]
struct NestedDepth {
    depth: usize,
    maximum: usize,
}

impl<'a> Visitor<'a> for NestedDepth {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.depth += usize::from(is_nested_block(stmt));
        self.maximum = self.maximum.max(self.depth);
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(is_nested_block(stmt));
    }
}

struct TryContext<'a> {
    reference: TextRange,
    statements: usize,
    catches: bool,
    handled: Vec<&'a Expr>,
    catch_all: bool,
}

impl<'a> Visitor<'a> for TryContext<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Try(branch) = stmt
            && !branch.handlers.is_empty()
            && branch.body.iter().any(|part| part.range().contains_range(self.reference))
        {
            self.statements = self.statements.max(ruff_statements(&branch.body));
            self.catches = true;
            for handler in &branch.handlers {
                let ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { type_, .. }) = handler;
                match type_ {
                    Some(exception) => self.handled.push(exception),
                    None => self.catch_all = true,
                }
            }
        }
        visitor::walk_stmt(self, stmt);
    }
}

fn catches(handler: &Expr, raised: &str) -> bool {
    match handler {
        Expr::Tuple(tuple) => tuple.elts.iter().any(|element| catches(element, raised)),
        _ => last_segment(handler).is_some_and(|name| name == raised || matches!(name, "Exception" | "BaseException")),
    }
}

// Ruff's `get_complexity_number` for C901, without the one it adds for the function itself.
fn mccabe_complexity(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + mccabe_complexity(body)
                    + elif_else_clauses
                        .iter()
                        .map(|clause| usize::from(clause.test.is_some()) + mccabe_complexity(&clause.body))
                        .sum::<usize>()
            }
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                1 + mccabe_complexity(body) + mccabe_complexity(orelse)
            }
            Stmt::With(ast::StmtWith { body, .. }) | Stmt::ClassDef(ast::StmtClassDef { body, .. }) => {
                mccabe_complexity(body)
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                let irrefutable =
                    cases.last().is_some_and(|case| case.guard.is_none() && case.pattern.is_irrefutable());
                cases.iter().map(|case| 1 + mccabe_complexity(&case.body)).sum::<usize>() - usize::from(irrefutable)
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                mccabe_complexity(body)
                    + usize::from(!orelse.is_empty())
                    + mccabe_complexity(orelse)
                    + mccabe_complexity(finalbody)
                    + handlers.iter().map(|handler| 1 + mccabe_complexity(handler_body(handler))).sum::<usize>()
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) => 1 + mccabe_complexity(body),
            _ => 0,
        })
        .sum()
}

// Ruff's `num_branches` for PLR0912.
fn ruff_branches(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + ruff_branches(body)
                    + elif_else_clauses.len()
                    + elif_else_clauses.iter().map(|clause| ruff_branches(&clause.body)).sum::<usize>()
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                cases.len() + cases.iter().map(|case| ruff_branches(&case.body)).sum::<usize>()
            }
            Stmt::With(ast::StmtWith { body, .. }) => ruff_branches(body),
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                1 + ruff_branches(body) + usize::from(!orelse.is_empty()) + ruff_branches(orelse)
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                ruff_branches(body)
                    + usize::from(!orelse.is_empty())
                    + ruff_branches(orelse)
                    + usize::from(!finalbody.is_empty())
                    + ruff_branches(finalbody)
                    + handlers.iter().map(|handler| 1 + ruff_branches(handler_body(handler))).sum::<usize>()
            }
            _ => 0,
        })
        .sum()
}

// Ruff's `num_statements` for PLR0915, which follows Pylint's counts.
fn ruff_statements(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + ruff_statements(body)
                    + elif_else_clauses.iter().map(|clause| 1 + ruff_statements(&clause.body)).sum::<usize>()
            }
            Stmt::For(ast::StmtFor { body, orelse, .. }) => ruff_statements(body) + ruff_statements(orelse),
            Stmt::While(ast::StmtWhile { body, orelse, .. }) => 1 + ruff_statements(body) + ruff_statements(orelse),
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                1 + cases.iter().map(|case| 1 + ruff_statements(&case.body)).sum::<usize>()
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                1 + ruff_statements(body)
                    + usize::from(!orelse.is_empty())
                    + ruff_statements(orelse)
                    + 2 * usize::from(!finalbody.is_empty())
                    + ruff_statements(finalbody)
                    + usize::from(handlers.len() > 1)
                    + handlers.iter().map(|handler| 1 + ruff_statements(handler_body(handler))).sum::<usize>()
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) | Stmt::With(ast::StmtWith { body, .. }) => {
                1 + ruff_statements(body)
            }
            Stmt::Return(_) => 0,
            _ => 1,
        })
        .sum()
}

// Ruff's `ReturnStatementVisitor` count for PLR0911.
fn ruff_returns(body: &[Stmt]) -> usize {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(body);
    visitor.returns.len()
}

fn handler_body(handler: &ExceptHandler) -> &[Stmt] {
    let ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { body, .. }) = handler;
    body
}

// Inlining replaces the call with a bare expression, which drops the narrowing a `TypeGuard` or `TypeIs` return gives.
fn is_type_guard(checker: &Checker, returns: &Expr) -> bool {
    let Expr::Subscript(subscript) = returns else {
        return false;
    };
    checker
        .semantic()
        .qualified_name(&subscript.value)
        .is_some_and(|name| matches!(name.segments(), ["typing" | "typing_extensions", "TypeGuard" | "TypeIs"]))
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
