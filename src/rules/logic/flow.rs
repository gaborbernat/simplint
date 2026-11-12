use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::rules::logic::inline::Names;
use crate::syntax::Nesting;

// How a statement leaves the block it sits in, if it always does.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Exit {
    Return,
    Raise,
    Break,
    Continue,
    EndlessLoop,
}

impl Exit {
    // Why the code after the statement does not run.
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::Return => "the `return` above it always leaves the block",
            Self::Raise => "the `raise` above it always leaves the block",
            Self::Break => "the `break` above it always leaves the block",
            Self::Continue => "the `continue` above it always leaves the block",
            Self::EndlessLoop => "the `while True` loop above it has no `break` and never ends",
        }
    }
}

// The first statement of the block after which nothing in the block runs, with how it leaves.
pub(super) fn first_exit(body: &[Stmt]) -> Option<(usize, Exit)> {
    body.iter().enumerate().find_map(|(index, stmt)| exit_of(stmt).map(|exit| (index, exit)))
}

fn block_exit(body: &[Stmt]) -> Option<Exit> {
    body.iter().find_map(exit_of)
}

// A `with` never counts: its context manager may swallow an exception, even one raised while a `return` builds its
// value, and the code after the `with` then runs.
fn exit_of(stmt: &Stmt) -> Option<Exit> {
    match stmt {
        Stmt::Return(_) => Some(Exit::Return),
        Stmt::Raise(_) => Some(Exit::Raise),
        Stmt::Break(_) => Some(Exit::Break),
        Stmt::Continue(_) => Some(Exit::Continue),
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            if elif_else_clauses.last().is_none_or(|clause| clause.test.is_some()) {
                return None;
            }
            let first = block_exit(body)?;
            elif_else_clauses.iter().all(|clause| block_exit(&clause.body).is_some()).then_some(first)
        }
        Stmt::While(ast::StmtWhile { test, body, .. }) if is_true_constant(test) && !has_own_break(body) => {
            Some(Exit::EndlessLoop)
        }
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
            if let Some(exit) = block_exit(finalbody) {
                return Some(exit);
            }
            let exit = block_exit(body).or_else(|| block_exit(orelse))?;
            handlers
                .iter()
                .all(|ast::ExceptHandler::ExceptHandler(handler)| block_exit(&handler.body).is_some())
                .then_some(exit)
        }
        _ => None,
    }
}

pub(super) fn is_true_constant(expr: &Expr) -> bool {
    match expr {
        Expr::BooleanLiteral(literal) => literal.value,
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(value), .. }) => {
            value.as_u8().is_some_and(|value| value != 0)
        }
        _ => false,
    }
}

pub(super) fn is_false_constant(expr: &Expr) -> bool {
    match expr {
        Expr::BooleanLiteral(literal) => !literal.value,
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(value), .. }) => value.as_u8() == Some(0),
        _ => false,
    }
}

// Whether the loop body holds a `break` or `continue` of its own, not one of a loop or function nested in it.
pub(super) fn has_own_break(body: &[Stmt]) -> bool {
    own_jump(body, |stmt| matches!(stmt, Stmt::Break(_)))
}

pub(super) fn has_own_jump(body: &[Stmt]) -> bool {
    own_jump(body, |stmt| matches!(stmt, Stmt::Break(_) | Stmt::Continue(_)))
}

fn own_jump(body: &[Stmt], wanted: fn(&Stmt) -> bool) -> bool {
    body.iter().any(|stmt| match stmt {
        // The `else` of a nested loop belongs to the outer one.
        Stmt::For(ast::StmtFor { orelse, .. }) | Stmt::While(ast::StmtWhile { orelse, .. }) => own_jump(orelse, wanted),
        Stmt::FunctionDef(_) | Stmt::ClassDef(_) => false,
        _ if wanted(stmt) => true,
        _ => {
            let mut found = false;
            crate::syntax::for_each_child_body(stmt, &mut |child| found |= own_jump(child, wanted));
            found
        }
    })
}

// Whether the statements hold a `yield`, `global` or `nonlocal` of the enclosing function, each of which changes the
// function whether or not it runs: a `yield` makes it a generator, and a declaration changes the scope of its names.
pub(super) fn changes_function_unrun(body: &[Stmt]) -> bool {
    let mut finder = ScopeMarkers::default();
    finder.visit_body(body);
    finder.found
}

#[derive(Default)]
struct ScopeMarkers {
    found: bool,
}

impl<'a> Visitor<'a> for ScopeMarkers {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Global(_) | Stmt::Nonlocal(_) => self.found = true,
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Yield(_) | Expr::YieldFrom(_) => self.found = true,
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// What a function's body binds and reads, walked once per function.
pub(super) struct FunctionNames<'a> {
    pub(super) names: Names<'a>,
    deferred: Nesting,
    comprehensions: Vec<(TextRange, Vec<&'a str>)>,
}

impl<'a> FunctionNames<'a> {
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut walk = NestedScopes::default();
        walk.visit_body(body);
        Self { names: Names::of(body), deferred: Nesting::new(walk.deferred), comprehensions: walk.comprehensions }
    }

    // Whether the rule can follow the name: the function binds it only by assignment, reads no local by its text, and no
    // nested function, class or lambda mentions it, since that may read it at any time.
    pub(super) fn is_plain_local(&self, name: &str) -> bool {
        let names = &self.names;
        !names.reads_locals
            && !names.declared_outer.contains(name)
            && !names.rebound.contains(name)
            && !names.string_reads.contains(name)
            && self.mentions(name).all(|(range, _)| self.deferred.holders(range).next().is_none())
    }

    // The mentions of the function's own name, in source order, leaving out those of a comprehension that binds a name
    // of the same spelling.
    pub(super) fn mentions(&self, name: &str) -> impl Iterator<Item = (TextRange, ast::ExprContext)> + Clone {
        self.names.mentions.get(name).map_or(&[][..], Vec::as_slice).iter().copied().filter(move |(range, _)| {
            !self.comprehensions.iter().any(|(scope, bound)| scope.contains_range(*range) && bound.contains(&name))
        })
    }
}

#[derive(Default)]
struct NestedScopes<'a> {
    deferred: Vec<TextRange>,
    comprehensions: Vec<(TextRange, Vec<&'a str>)>,
}

impl<'a> Visitor<'a> for NestedScopes<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            self.deferred.push(stmt.range());
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let generators = match expr {
            Expr::Lambda(_) => {
                self.deferred.push(expr.range());
                None
            }
            Expr::ListComp(ast::ExprListComp { generators, .. })
            | Expr::SetComp(ast::ExprSetComp { generators, .. })
            | Expr::Generator(ast::ExprGenerator { generators, .. }) => Some(generators.as_slice()),
            Expr::DictComp(ast::ExprDictComp { generators, .. }) => Some(&generators[..]),
            _ => None,
        };
        if let Some(generators) = generators {
            let mut bound = Vec::new();
            for generator in generators {
                collect_names(&generator.target, &mut bound);
            }
            self.comprehensions.push((expr.range(), bound));
        }
        visitor::walk_expr(self, expr);
    }
}

pub(super) fn collect_names<'a>(target: &'a Expr, into: &mut Vec<&'a str>) {
    match target {
        Expr::Name(name) => into.push(name.id.as_str()),
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            for element in elts {
                collect_names(element, into);
            }
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => collect_names(value, into),
        _ => {}
    }
}
