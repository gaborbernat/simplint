use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::checker::Checker;
use crate::rules::logic::inline::leading_name;
use crate::syntax::is_name;

// Whether Ruff's B006 or B008 reports the value once it is the default of `parameter`. Both skip a parameter whose
// annotation is immutable.
pub(super) fn reports_default(checker: &Checker, value: &Expr, parameter: &ast::ParameterWithDefault) -> bool {
    let from_ruff = &checker.settings.from_ruff;
    let b006 = from_ruff.ruff_runs(RuffRule::MutableArgumentDefault);
    let b008 = from_ruff.ruff_runs(RuffRule::FunctionCallInDefaultArgument);
    if !(b006 || b008) {
        return false;
    }
    let extend_immutable_calls: Vec<QualifiedName> =
        from_ruff.extend_immutable_calls.iter().map(|target| QualifiedName::from_dotted_name(target)).collect();
    if parameter
        .annotation()
        .is_some_and(|annotation| checker.semantic().has_immutable_annotation(annotation, &extend_immutable_calls))
    {
        return false;
    }
    let mut calls = DefaultCalls { checker, extend_immutable_calls: &extend_immutable_calls, reported: false };
    if b008 {
        calls.visit_expr(value);
    }
    calls.reported || (b006 && is_b006_mutable(checker, value))
}

// The values B006 reports as a mutable default, as its preview mode reports them: also a generator, and a tuple that
// holds one of the mutable values.
fn is_b006_mutable(checker: &Checker, value: &Expr) -> bool {
    match value {
        Expr::Generator(_) => true,
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().any(|elt| is_b006_mutable(checker, elt)),
        _ => checker.semantic().is_mutable_value(value),
    }
}

struct DefaultCalls<'a, 'b> {
    checker: &'a Checker<'b>,
    extend_immutable_calls: &'a [QualifiedName<'a>],
    reported: bool,
}

impl<'a> Visitor<'a> for DefaultCalls<'_, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::Call(ast::ExprCall { func, .. })
                if self.checker.semantic().is_b008_call(func, self.extend_immutable_calls) =>
            {
                self.reported = true;
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// Ruff's RUF070 reports the same pair even when its fix writes a different expression.
pub(super) fn ruff_reports_yield_from(target: &ast::ExprName, next: &Stmt) -> bool {
    let Stmt::Expr(ast::StmtExpr { value, .. }) = next else {
        return false;
    };
    let Expr::YieldFrom(ast::ExprYieldFrom { value: yielded, .. }) = value.as_ref() else {
        return false;
    };
    is_name(yielded, target.id.as_str())
}

// Ruff's EM101 and EM102 ask for the message of an exception to be a variable, so the `raise` keeps it.
pub(super) fn raises_with_name(next: &Stmt, name: &str) -> bool {
    let Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) = next else {
        return false;
    };
    exc.as_call_expr().is_some_and(|call| call.arguments.args.iter().any(|argument| is_name(argument, name)))
}

// `return name` is left to Ruff's RET504, which covers the return case of a function that does not yield. This rule
// would also add an inferred return annotation to a literal value, which Ruff does not write. `yield name` is never
// reported, as before.
pub(super) fn returns_or_yields(body: &[Stmt], next: &Stmt, name: &str) -> bool {
    let value = match next {
        Stmt::Return(ast::StmtReturn { value, .. }) if !is_generator(body) => value.as_deref(),
        Stmt::Expr(ast::StmtExpr { value, .. }) => match value.as_ref() {
            Expr::Yield(ast::ExprYield { value, .. }) => value.as_deref(),
            _ => None,
        },
        _ => None,
    };
    value.is_some_and(|value| is_name(value, name))
}

// Ruff's RET504 skips a function that yields, and its return visitor reads the lambdas and comprehensions of the body
// and the decorators, annotations and defaults of a nested function, but not the bodies of nested functions or classes.
fn is_generator(body: &[Stmt]) -> bool {
    let mut finder = Yields { found: false };
    finder.visit_body(body);
    finder.found
}

struct Yields {
    found: bool,
}

impl<'a> Visitor<'a> for Yields {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, parameters, returns, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(returns) = returns {
                    self.visit_expr(returns);
                }
                self.visit_parameters(parameters);
            }
            Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Yield(_) | Expr::YieldFrom(_) => self.found = true,
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// An `if` whose test starts with the name is SMP502's, which writes the value into the test itself.
pub(super) fn leads_if_test(next: &Stmt, name: &str) -> bool {
    let Stmt::If(ast::StmtIf { test, .. }) = next else {
        return false;
    };
    leading_name(test, true).is_some_and(|(found, _)| found.id.as_str() == name)
}
