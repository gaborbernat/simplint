use std::cell::RefCell;

use ruff_python_ast::helpers::{any_over_body, any_over_expr};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_module_function;
use crate::rules::testing::recognize::is_assert_raises_family;
use crate::syntax::{for_each_body, is_name, target_names};

pub fn loop_in_test(checker: &Checker) {
    let mut found = Vec::new();
    for function in checker.function_defs() {
        if is_test_module_function(checker, function) {
            Loops { cases: Cases { checker, function, known: RefCell::default() }, found: &mut found }
                .visit_body(&function.body);
        }
    }
    found.sort_by_key(Ranged::start);
    for range in found {
        checker.report(Rule::LoopInTest, range, "Loop with assertions in a test; parameterize the cases instead");
    }
}

struct Loops<'c, 'a> {
    cases: Cases<'c, 'a>,
    found: &'c mut Vec<TextRange>,
}

impl<'a> Visitor<'a> for Loops<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            // The loop's nested loops check the same cases, so the outermost one is the report.
            Stmt::For(for_stmt) if self.is_case_loop(for_stmt) => self.found.push(stmt.range()),
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}

impl<'a> Loops<'_, 'a> {
    // A parameterized test needs its cases before it runs, so a loop over values the test computes, such as the
    // items of a result, is a check of that result rather than a table of cases.
    fn is_case_loop(&self, for_stmt: &'a ast::StmtFor) -> bool {
        !for_stmt.is_async
            && for_stmt.body.iter().any(asserts_in)
            && exercises(for_stmt)
            && self.cases.is_table(&for_stmt.iter, 0)
    }
}

// Whether each case reaches the code under test: a statement other than an assertion reads the loop's target, or an
// assertion passes it to a call. A loop that never reads its target repeats one case, and one whose assertions only
// compare each item with a result computed before the loop, as `assert name in names` does, checks that one result.
fn exercises(for_stmt: &ast::StmtFor) -> bool {
    let names = target_names(&for_stmt.target);
    let mut finder = Exercises { names: &names, found: false };
    finder.visit_body(&for_stmt.body);
    finder.found
}

struct Exercises<'n> {
    names: &'n [&'n str],
    found: bool,
}

impl<'a> Visitor<'a> for Exercises<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assert(ast::StmtAssert { test, .. }) => self.found |= self.is_called_with(test),
            // `assertRaises(error, function, case)` calls the function with the case.
            Stmt::Expr(ast::StmtExpr { value, .. })
                if is_self_assertion(value)
                    && let Expr::Call(ast::ExprCall { func, arguments, .. }) = value.as_ref()
                    && !is_assert_raises_family(func) =>
            {
                self.found |= arguments
                    .args
                    .iter()
                    .chain(arguments.keywords.iter().map(|keyword| &keyword.value))
                    .any(|argument| self.is_called_with(argument));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.is_target(expr) {
            self.found = true;
        } else {
            visitor::walk_expr(self, expr);
        }
    }
}

impl Exercises<'_> {
    fn is_called_with(&self, expr: &Expr) -> bool {
        any_over_expr(expr, |call| call.is_call_expr() && any_over_expr(call, |read| self.is_target(read)))
    }

    fn is_target(&self, expr: &Expr) -> bool {
        expr.as_name_expr().is_some_and(|name| name.ctx.is_load() && self.names.contains(&name.id.as_str()))
    }
}

// Name lookups a value may follow, which stops an assignment that reads its own name from recursing forever.
const MAX_NAME_DEPTH: usize = 8;

struct Cases<'c, 'a> {
    checker: &'c Checker<'a>,
    function: &'a ast::StmtFunctionDef,
    // Whether a local name is known, by its binding, whether a table is asked and the depth it is asked at, so each
    // chain of names is followed once rather than once per path to it.
    known: RefCell<FxHashMap<(TextRange, bool, usize), bool>>,
}

impl<'a> Cases<'_, 'a> {
    // Cases the test file lists: a literal, a name for one, or the builtins that number, pair, combine or reorder
    // them. An attribute, such as `module.names`, is data of the code under test, which the loop checks rather than
    // lists, and a `range` sweeps numbers or steps through state rather than naming cases.
    fn is_table(&self, expr: &'a Expr, depth: usize) -> bool {
        match expr {
            Expr::Attribute(_) => false,
            Expr::Name(name) => self.is_known_name(name, depth, true),
            Expr::BinOp(ast::ExprBinOp { left, right, .. }) => {
                self.is_table(left, depth) && self.is_known(right, depth)
            }
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
                self.is_table(value, depth) && self.is_known(slice, depth)
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                self.is_case_builder(func, depth)
                    && arguments.args.iter().all(|argument| self.is_table(argument, depth))
                    && arguments.keywords.iter().all(|keyword| self.is_known(&keyword.value, depth))
            }
            _ => self.is_known(expr, depth),
        }
    }

    // Values known before the test runs: literals, names the test does not bind or assigns once to such a value,
    // and the builtins that number, pair, combine or reorder them.
    fn is_known(&self, expr: &'a Expr, depth: usize) -> bool {
        let known = |expr: &'a Expr| self.is_known(expr, depth);
        match expr {
            Expr::StringLiteral(_)
            | Expr::BytesLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_)
            | Expr::NoneLiteral(_)
            | Expr::EllipsisLiteral(_) => true,
            Expr::List(ast::ExprList { elts, .. })
            | Expr::Tuple(ast::ExprTuple { elts, .. })
            | Expr::Set(ast::ExprSet { elts, .. }) => elts.iter().all(known),
            Expr::Dict(dict) => dict.items.iter().all(|item| item.key.as_ref().is_none_or(known) && known(&item.value)),
            Expr::Starred(ast::ExprStarred { value, .. }) | Expr::Attribute(ast::ExprAttribute { value, .. }) => {
                known(value)
            }
            Expr::UnaryOp(ast::ExprUnaryOp { operand, .. }) => known(operand),
            Expr::BinOp(ast::ExprBinOp { left, right, .. }) => known(left) && known(right),
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => known(value) && known(slice),
            Expr::Slice(ast::ExprSlice { lower, upper, step, .. }) => {
                [lower, upper, step].into_iter().flatten().all(|bound| known(bound))
            }
            Expr::Call(_) => self.is_table(expr, depth),
            Expr::Name(name) => self.is_known_name(name, depth, false),
            _ => false,
        }
    }

    fn is_case_builder(&self, func: &'a Expr, depth: usize) -> bool {
        const BUILTINS: &[&str] = &["enumerate", "zip", "reversed", "sorted", "list", "tuple", "set", "frozenset"];
        const ITERTOOLS: &[&str] = &["product", "permutations", "combinations", "combinations_with_replacement"];
        const METHODS: &[&str] = &["items", "keys", "values", "split"];
        let semantic = self.checker.semantic();
        match func {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if METHODS.contains(&attr.as_str()) => {
                self.is_table(value, depth)
            }
            _ => {
                BUILTINS.iter().any(|name| semantic.is_builtin(func, name))
                    || ITERTOOLS.iter().any(|name| semantic.resolves_to(func, &["itertools", name]))
            }
        }
    }

    // A name the test function does not bind is a module constant or a builtin. A local name is known when its only
    // binding is an assignment of a known value; a parameter, a loop target or a result the test computes is not.
    fn is_known_name(&self, name: &ast::ExprName, depth: usize, table: bool) -> bool {
        let function = self.function.range();
        let mut bindings = self
            .checker
            .semantic()
            .local_bindings(name.id.as_str())
            .iter()
            .filter(|binding| function.contains_range(**binding));
        let Some(&binding) = bindings.next() else {
            return true;
        };
        let key = (binding, table, depth);
        if let Some(&known) = self.known.borrow().get(&key) {
            return known;
        }
        let known = bindings.next().is_none()
            && depth < MAX_NAME_DEPTH
            && !is_mutated(&self.function.body, name.id.as_str())
            && assigned_value(&self.function.body, binding).is_some_and(|value| {
                if table { self.is_table(value, depth + 1) } else { self.is_known(value, depth + 1) }
            });
        self.known.borrow_mut().insert(key, known);
        known
    }
}

// A container the test fills in, as `values.append(compute())` does, holds what the test computes.
fn is_mutated(body: &[Stmt], name: &str) -> bool {
    const MUTATORS: &[&str] = &["append", "extend", "insert", "add", "update", "setdefault"];
    any_over_body(body, |expr| match expr {
        Expr::Call(ast::ExprCall { func, .. }) => matches!(func.as_ref(),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. })
                if MUTATORS.contains(&attr.as_str()) && is_name(value, name)),
        Expr::Subscript(ast::ExprSubscript { value, ctx, .. }) => !ctx.is_load() && is_name(value, name),
        _ => false,
    })
}

fn assigned_value(body: &[Stmt], binding: TextRange) -> Option<&Expr> {
    let mut assigned = None;
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            let (target, value) = match stmt {
                Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], value),
                Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => (target.as_ref(), value),
                _ => continue,
            };
            if target.is_name_expr() && target.range() == binding {
                assigned = Some(value.as_ref());
            }
        }
    });
    assigned
}

fn asserts_in(stmt: &Stmt) -> bool {
    let mut finder = Asserts { found: false };
    finder.visit_stmt(stmt);
    finder.found
}

// Assertions that run once per item, or in a helper's body, are the loop's checks. Those inside a `try` of the loop
// are not: a retry around an assertion waits for a condition, which is polling rather than a set of cases. Nor are
// those inside a subtest, which runs each case to the end and names it in the report, as a parameterized test does.
struct Asserts {
    found: bool,
}

impl<'a> Visitor<'a> for Asserts {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::With(ast::StmtWith { items, .. }) if items.iter().any(|item| is_subtest(&item.context_expr)) => {}
            Stmt::Assert(_) => self.found = true,
            Stmt::Try(ast::StmtTry { handlers, orelse, finalbody, .. }) => {
                for handler in handlers {
                    self.visit_except_handler(handler);
                }
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if is_self_assertion(expr) {
            self.found = true;
        } else {
            visitor::walk_expr(self, expr);
        }
    }
}

// `TestCase.subTest` in `unittest`, and the `subtests` fixture of pytest.
fn is_subtest(expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return false;
    };
    matches!(func.as_ref(), Expr::Attribute(ast::ExprAttribute { value, attr, .. })
        if attr == "subTest" || (attr == "test" && is_name(value, "subtests")))
}

// The `assertEqual` family of `unittest.TestCase`, which checks like an assert statement.
fn is_self_assertion(expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return false;
    };
    matches!(func.as_ref(), Expr::Attribute(ast::ExprAttribute { value, attr, .. })
        if is_name(value, "self") && attr.starts_with("assert"))
}
