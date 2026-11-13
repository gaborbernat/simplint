// What a scope's body binds and reads, for the rules that follow a local variable from its binding to its reads without
// a type checker. Control flow lives in `flow`.

use std::cell::OnceCell;

use ruff_python_ast::visitor::{Visitor, walk_expr, walk_pattern, walk_stmt};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::rules::logic::inline::Names;
use crate::syntax::{Nesting, target_names, top_level};

// The module and each function of a file, nested ones included, with what the local-variable rules read off each. A
// rule asks only for the scopes its cheap syntactic trigger points at, and the first rule to ask builds the answer for
// the others.
pub(crate) struct LocalScopes<'a> {
    scopes: Vec<LocalScope<'a>>,
    functions: FxHashMap<TextSize, usize>,
}

impl<'a> LocalScopes<'a> {
    pub(crate) fn of(body: &'a [Stmt], functions: &[&'a ast::StmtFunctionDef]) -> Self {
        let scopes = std::iter::once(LocalScope::new(body, None))
            .chain(functions.iter().map(|function| LocalScope::new(&function.body, Some(*function))))
            .collect();
        let functions = functions.iter().enumerate().map(|(index, function)| (function.start(), index + 1)).collect();
        Self { scopes, functions }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &LocalScope<'a>> {
        self.scopes.iter()
    }

    pub(super) fn function(&self, function: &ast::StmtFunctionDef) -> &LocalScope<'a> {
        &self.scopes[self.functions[&function.start()]]
    }
}

pub(super) struct LocalScope<'a> {
    pub(super) body: &'a [Stmt],
    // The function whose body this is, `None` for the module.
    pub(super) function: Option<&'a ast::StmtFunctionDef>,
    uses: OnceCell<NameUses<'a>>,
    nodes: OnceCell<ScopeNodes<'a>>,
    names: OnceCell<FunctionNames<'a>>,
}

impl<'a> LocalScope<'a> {
    fn new(body: &'a [Stmt], function: Option<&'a ast::StmtFunctionDef>) -> Self {
        Self { body, function, uses: OnceCell::new(), nodes: OnceCell::new(), names: OnceCell::new() }
    }

    pub(super) fn uses(&self) -> &NameUses<'a> {
        self.uses.get_or_init(|| self.function.map_or_else(|| NameUses::of_body(self.body), NameUses::of_function))
    }

    pub(super) fn nodes(&self) -> &ScopeNodes<'a> {
        self.nodes.get_or_init(|| ScopeNodes::of(self.body))
    }

    pub(super) fn names(&self) -> &FunctionNames<'a> {
        self.names.get_or_init(|| FunctionNames::of(self.body))
    }
}

#[derive(Default)]
pub(super) struct NameUses<'a> {
    stores: FxHashMap<&'a str, Vec<TextRange>>,
    loads: FxHashMap<&'a str, Vec<&'a ast::ExprName>>,
    // Names a nested function, lambda or class reads or binds, a comprehension binds, or a `global`, `nonlocal`, `del`
    // or `match` pattern touches: a read of the scope alone cannot follow them.
    opaque: FxHashSet<&'a str>,
}

impl<'a> NameUses<'a> {
    // The uses in a function: its parameters bind their names before the body runs.
    pub(super) fn of_function(function: &'a ast::StmtFunctionDef) -> Self {
        let mut uses = Self::default();
        for parameter in &function.parameters {
            uses.stores.entry(parameter.name().as_str()).or_default().push(parameter.name().range());
        }
        uses.visit_body(&function.body);
        uses
    }

    pub(super) fn of_body(body: &'a [Stmt]) -> Self {
        let mut uses = Self::default();
        uses.visit_body(body);
        uses
    }

    // A local the scope binds once and only it reads.
    pub(super) fn is_single_binding(&self, name: &str) -> bool {
        self.stores(name).len() == 1 && !self.is_opaque(name)
    }

    pub(super) fn stores(&self, name: &str) -> &[TextRange] {
        self.stores.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn loads(&self, name: &str) -> &[&'a ast::ExprName] {
        self.loads.get(name).map_or(&[], Vec::as_slice)
    }

    pub(super) fn is_opaque(&self, name: &str) -> bool {
        self.opaque.contains(name)
    }

    fn mark_opaque(&mut self, node: &'a Stmt) {
        let mut names = AllNames::default();
        names.visit_stmt(node);
        self.opaque.extend(names.0);
    }
}

impl<'a> Visitor<'a> for NameUses<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {
                if let Some(name) = defined_name(stmt) {
                    self.stores.entry(name.0).or_default().push(name.1);
                }
                self.mark_opaque(stmt);
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.opaque.extend(names.iter().map(ast::Identifier::as_str));
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name);
                    let name = top_level(bound.as_str());
                    self.stores.entry(name).or_default().push(bound.range());
                }
            }
            Stmt::Try(ast::StmtTry { handlers, .. }) => {
                for handler in handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    if let Some(name) = &handler.name {
                        self.stores.entry(name.as_str()).or_default().push(name.range());
                    }
                }
                walk_stmt(self, stmt);
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => match name.ctx {
                ExprContext::Load => self.loads.entry(name.id.as_str()).or_default().push(name),
                ExprContext::Store => self.stores.entry(name.id.as_str()).or_default().push(name.range),
                ExprContext::Del | ExprContext::Invalid => {
                    self.opaque.insert(name.id.as_str());
                }
            },
            Expr::Lambda(_) => {
                let mut names = AllNames::default();
                names.visit_expr(expr);
                self.opaque.extend(names.0);
            }
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::Generator(_) | Expr::DictComp(_) => {
                for generator in comprehension_generators(expr) {
                    let mut names = AllNames::default();
                    names.visit_expr(&generator.target);
                    self.opaque.extend(names.0);
                }
                walk_expr(self, expr);
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let mut names = AllNames::default();
        names.visit_pattern(pattern);
        self.opaque.extend(names.0);
        walk_pattern(self, pattern);
    }
}

fn defined_name(stmt: &Stmt) -> Option<(&str, TextRange)> {
    match stmt {
        Stmt::FunctionDef(function) => Some((function.name.as_str(), function.name.range())),
        Stmt::ClassDef(class) => Some((class.name.as_str(), class.name.range())),
        _ => None,
    }
}

// Every name a node mentions, in any context.
#[derive(Default)]
struct AllNames<'a>(Vec<&'a str>);

impl<'a> Visitor<'a> for AllNames<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.0.push(name.id.as_str());
        }
        walk_expr(self, expr);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name: Some(name), .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name: Some(name), .. })
            | Pattern::MatchMapping(ast::PatternMatchMapping { rest: Some(name), .. }) => self.0.push(name.as_str()),
            _ => {}
        }
        walk_pattern(self, pattern);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Some((name, _)) = defined_name(stmt) {
            self.0.push(name);
        }
        if let Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) = stmt {
            self.0.extend(names.iter().map(ast::Identifier::as_str));
        }
        walk_stmt(self, stmt);
    }

    fn visit_parameters(&mut self, parameters: &'a ast::Parameters) {
        self.0.extend(parameters.iter().map(|parameter| parameter.name().as_str()));
        ruff_python_ast::visitor::walk_parameters(self, parameters);
    }
}

// Every statement and expression of one scope, outside the bodies of nested functions, classes and lambdas.
#[derive(Default)]
pub(super) struct ScopeNodes<'a> {
    pub(super) statements: Vec<&'a Stmt>,
    pub(super) expressions: Vec<&'a Expr>,
}

impl<'a> ScopeNodes<'a> {
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut nodes = Self::default();
        nodes.visit_body(body);
        nodes
    }
}

impl<'a> Visitor<'a> for ScopeNodes<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.statements.push(stmt);
        if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            walk_stmt(self, stmt);
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.expressions.push(expr);
        if !expr.is_lambda_expr() {
            walk_expr(self, expr);
        }
    }
}

// The statement lists of one scope, without those of nested functions and classes, which are scopes of their own.
pub(super) fn for_each_suite<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt])) {
    visit(body);
    for stmt in body {
        if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            crate::syntax::for_each_child_body(stmt, &mut |child| for_each_suite(child, visit));
        }
    }
}

pub(super) fn comprehension_generators(expr: &Expr) -> &[ast::Comprehension] {
    match expr {
        Expr::ListComp(ast::ExprListComp { generators, .. })
        | Expr::SetComp(ast::ExprSetComp { generators, .. })
        | Expr::Generator(ast::ExprGenerator { generators, .. }) => generators,
        Expr::DictComp(ast::ExprDictComp { generators, .. }) => generators,
        _ => &[],
    }
}

// What a scope's body binds and reads by name, with the nested scopes that may read a name at any time.
pub(super) struct FunctionNames<'a> {
    pub(super) names: Names<'a>,
    // The nested functions, classes and lambdas.
    pub(super) deferred: Nesting,
    comprehensions: Vec<(TextRange, Vec<&'a str>)>,
}

impl<'a> FunctionNames<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut walk = NestedScopes::default();
        walk.visit_body(body);
        Self { names: Names::of(body), deferred: Nesting::new(walk.deferred), comprehensions: walk.comprehensions }
    }

    // Whether the rule can follow the name: the function binds it only by assignment, reads no local by its text, and
    // no nested function, class or lambda mentions it, since that may read it at any time.
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
    pub(super) fn mentions(&self, name: &str) -> impl Iterator<Item = (TextRange, ExprContext)> + Clone {
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
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.is_lambda_expr() {
            self.deferred.push(expr.range());
        }
        let generators = comprehension_generators(expr);
        if !generators.is_empty() {
            let bound = generators.iter().flat_map(|generator| target_names(&generator.target)).collect();
            self.comprehensions.push((expr.range(), bound));
        }
        walk_expr(self, expr);
    }
}
