use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_module_function;
use crate::semantic::Semantic;
use crate::syntax::ExprCollector;

pub fn conditional_in_test(checker: &Checker) {
    let mut found: Vec<TextRange> = checker
        .function_defs()
        .iter()
        .copied()
        .filter(|function| is_test_module_function(checker, function))
        .flat_map(|function| conditionals(checker.semantic(), &function.body))
        .collect();
    found.sort_by_key(Ranged::start);
    found.dedup();
    for range in found {
        checker.report(Rule::ConditionalInTest, range, "Branching inside a test; parameterize instead");
    }
}

pub(crate) fn conditionals(semantic: &Semantic, body: &[Stmt]) -> Vec<TextRange> {
    let mut finder = Conditionals { semantic, loop_state: Vec::new(), found: Vec::new() };
    finder.visit_body(body);
    finder.found
}

struct Conditionals<'s, 'a> {
    semantic: &'s Semantic<'s>,
    // The names each enclosing loop rebinds as it runs: what a `while` body assigns, and the target of a `for` over
    // values the test receives rather than lists.
    loop_state: Vec<FxHashSet<&'a str>>,
    found: Vec<TextRange>,
}

impl<'a> Visitor<'a> for Conditionals<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::If(ast::StmtIf { test, .. }) => {
                self.branch(test, stmt.range());
                visitor::walk_stmt(self, stmt);
            }
            Stmt::Match(ast::StmtMatch { subject, .. }) => {
                self.branch(subject, stmt.range());
                visitor::walk_stmt(self, stmt);
            }
            Stmt::While(ast::StmtWhile { body, .. }) => {
                self.in_loop(stored_names(body), stmt);
            }
            Stmt::For(ast::StmtFor { target, iter, .. }) if receives(iter) => {
                let mut targets = target_names();
                targets.visit_expr(target);
                self.in_loop(targets.found.into_iter().collect(), stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::If(ast::ExprIf { test, .. }) => {
                self.branch(test, expr.range());
                visitor::walk_expr(self, expr);
            }
            Expr::ListComp(ast::ExprListComp { elt, generators, .. })
            | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
            | Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => {
                self.visit_element(elt);
                for generator in generators {
                    self.visit_comprehension(generator);
                }
            }
            Expr::DictComp(ast::ExprDictComp { key, value, generators, .. }) => {
                if let Some(key) = key {
                    self.visit_element(key);
                }
                self.visit_element(value);
                for generator in generators {
                    self.visit_comprehension(generator);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

impl<'a> Conditionals<'_, 'a> {
    // Like an `if` filter, a conditional that is a comprehension's item shapes data, so only its parts are visited.
    fn visit_element(&mut self, element: &'a Expr) {
        if let Expr::If(ast::ExprIf { test, body, orelse, .. }) = element {
            self.visit_expr(test);
            self.visit_expr(body);
            self.visit_expr(orelse);
        } else {
            self.visit_expr(element);
        }
    }

    // A branch on the interpreter or its environment follows the machine running the test, and a branch on what a
    // protocol loop received follows the exchange, so neither can become a parameter.
    fn branch(&mut self, test: &Expr, range: TextRange) {
        if environment(self.semantic, test) != Some(true) && !self.reads_loop_state(test) {
            self.found.push(range);
        }
    }

    fn reads_loop_state(&self, test: &Expr) -> bool {
        let mut reads = ExprCollector::new(|expr: &Expr| expr.as_name_expr().map(|name| name.id.as_str()));
        reads.visit_expr(test);
        reads.found.iter().any(|name| self.loop_state.iter().any(|names| names.contains(name)))
    }

    fn in_loop(&mut self, names: FxHashSet<&'a str>, stmt: &'a Stmt) {
        self.loop_state.push(names);
        visitor::walk_stmt(self, stmt);
        self.loop_state.pop();
    }
}

fn stored_names(body: &[Stmt]) -> FxHashSet<&str> {
    let mut stores = target_names();
    stores.visit_body(body);
    stores.found.into_iter().collect()
}

fn target_names<'a>() -> ExprCollector<impl FnMut(&'a Expr) -> Option<&'a str>, &'a str> {
    ExprCollector::new(|expr: &'a Expr| {
        expr.as_name_expr().filter(|name| name.ctx == ExprContext::Store).map(|name| name.id.as_str())
    })
}

// A `for` over a literal collection or a `range` lists cases the test chose, which parameters could hold; any other
// iterable hands the test values it receives.
fn receives(iter: &Expr) -> bool {
    match iter {
        Expr::List(_) | Expr::Tuple(_) | Expr::Set(_) | Expr::Dict(_) => false,
        Expr::Call(ast::ExprCall { func, .. }) => !func.as_name_expr().is_some_and(|name| name.id == "range"),
        _ => true,
    }
}

const ENVIRONMENT: &[&[&str]] = &[
    &["sys", "platform"],
    &["sys", "version_info"],
    &["sys", "implementation"],
    &["os", "name"],
    &["os", "environ"],
    &["os", "getenv"],
    &["platform"],
];

// Whether an expression reads the interpreter or its environment and nothing else: `Some(true)` when it reads one of
// them, `Some(false)` for a literal, `None` when it reads anything else.
fn environment(semantic: &Semantic, expr: &Expr) -> Option<bool> {
    match expr {
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => Some(false),
        Expr::Name(ast::ExprName { id, .. }) if is_environment_constant(id) => Some(true),
        Expr::Name(_) | Expr::Attribute(_) if names_environment(semantic, expr) => Some(true),
        Expr::Attribute(ast::ExprAttribute { value, .. }) => environment(semantic, value).filter(|read| *read),
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            combine([environment(semantic, value).filter(|read| *read), environment(semantic, slice)])
        }
        Expr::Slice(ast::ExprSlice { lower, upper, step, .. }) => {
            combine([lower, upper, step].into_iter().flatten().map(|part| environment(semantic, part)))
        }
        Expr::Call(ast::ExprCall { func, arguments, .. }) => combine(
            std::iter::once(environment(semantic, func).filter(|read| *read))
                .chain(arguments.args.iter().map(|argument| environment(semantic, argument))),
        ),
        Expr::Compare(ast::ExprCompare { operands, .. }) => {
            combine(operands.iter().map(|operand| environment(semantic, operand)))
        }
        Expr::BoolOp(ast::ExprBoolOp { values, .. }) | Expr::Tuple(ast::ExprTuple { elts: values, .. }) => {
            combine(values.iter().map(|value| environment(semantic, value)))
        }
        Expr::UnaryOp(ast::ExprUnaryOp { operand, .. }) => environment(semantic, operand),
        _ => None,
    }
}

fn combine(parts: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    parts.into_iter().try_fold(false, |reads, part| part.map(|read| reads || read))
}

fn names_environment(semantic: &Semantic, expr: &Expr) -> bool {
    semantic
        .qualified_name(expr)
        .is_some_and(|name| ENVIRONMENT.iter().any(|prefix| name.segments().starts_with(prefix)))
}

// A module constant that names a platform or interpreter check, such as `IS_WINDOWS` or `ON_PYPY`.
fn is_environment_constant(name: &str) -> bool {
    name.chars().all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '_')
        && (name.starts_with("IS_")
            || name.split('_').any(|part| {
                matches!(part, "WINDOWS" | "WIN32" | "PYPY" | "CPYTHON" | "MACOS" | "DARWIN" | "LINUX" | "POSIX")
            }))
}
