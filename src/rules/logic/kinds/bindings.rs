use std::rc::Rc;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Operator, Pattern, Stmt};
use ruff_text_size::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};

use super::Kind;
use super::declared::annotation;
use crate::lint::checker::Checker;
use crate::lint::walker::Scope;
use crate::syntax::{alias_binding, for_each_child_body};

// The bindings of `name` in `scope`, or in the module for none. A scope is walked once for all of its names.
pub(super) fn bindings_in<'a>(checker: &Checker<'a>, scope: Option<Scope<'a>>, name: &'a str) -> Rc<Bindings<'a>> {
    let range = match scope {
        Some(Scope::Function(function)) => function.range,
        Some(Scope::Class(class)) => class.range,
        Some(Scope::Lambda(lambda)) => lambda.range,
        None => TextRange::default(),
    };
    if let Some(scope_bindings) = checker.state().kinds().scopes.borrow().get(&range) {
        return scope_bindings.get(name);
    }
    let scope_bindings = Rc::new(match scope {
        Some(scope) => ScopeBindings::of_scope(checker, scope),
        None => ScopeBindings::of_body(checker, checker.body()),
    });
    checker.state().kinds().scopes.borrow_mut().insert(range, Rc::clone(&scope_bindings));
    scope_bindings.get(name)
}

#[derive(Clone, Copy)]
pub(super) enum Value<'a> {
    Expr(&'a Expr),
    // An item of what iterating the expression gives, as a loop target or an unpacked name holds.
    Item(&'a Expr),
    // What entering the context manager gives a `with` target.
    Entered(&'a Expr),
}

// How one scope binds a name: the values assignments and loops give it, the kind an annotation declares for it, the
// functions it names, whether an import binds it, and whether anything else (a parameter, `with` target, `global`)
// binds it to a value the code does not show.
#[derive(Default)]
pub(super) struct Bindings<'a> {
    pub(super) values: Vec<Value<'a>>,
    pub(super) declared: Option<Kind>,
    pub(super) functions: Vec<&'a ast::StmtFunctionDef>,
    pub(super) classes: Vec<&'a ast::StmtClassDef>,
    pub(super) imported: bool,
    // The name a `from` import binds the name as, which is the class or function it names in its module.
    pub(super) original: Option<&'a str>,
    pub(super) opaque: bool,
}

impl<'a> Bindings<'a> {
    // An annotation the code shows no kind for, or two that disagree, leave the name unknown.
    pub(super) fn declare(&mut self, kind: Option<Kind>) {
        match kind {
            Some(kind) if self.declared.is_none_or(|declared| declared == kind) => self.declared = Some(kind),
            _ => self.opaque = true,
        }
    }

    pub(super) fn binds(&self) -> bool {
        self.declared.is_some()
            || self.opaque
            || self.imported
            || !self.values.is_empty()
            || !self.functions.is_empty()
            || !self.classes.is_empty()
    }

    pub(super) fn only_imported(&self) -> bool {
        self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.functions.is_empty()
    }

    pub(super) fn only_classes(&self) -> bool {
        !self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.functions.is_empty()
    }

    pub(super) fn only_functions(&self) -> Option<&[&'a ast::StmtFunctionDef]> {
        (!self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.classes.is_empty())
            .then_some(self.functions.as_slice())
    }
}

// The bindings of every name a scope binds, which one walk of the scope collects.
pub(super) struct ScopeBindings<'a> {
    names: FxHashMap<&'a str, Rc<Bindings<'a>>>,
    // A `from` import of `*` in the scope binds every name to a value the code does not show.
    star: bool,
}

impl<'a> ScopeBindings<'a> {
    fn of_scope(checker: &Checker<'a>, scope: Scope<'a>) -> Self {
        let mut collected = Collected::default();
        match scope {
            Scope::Function(function) => {
                collected.parameters(checker, &function.parameters);
                collected.collect(checker, &function.body);
            }
            Scope::Class(class) => collected.collect(checker, &class.body),
            Scope::Lambda(lambda) => {
                if let Some(parameters) = lambda.parameters.as_deref() {
                    collected.parameters(checker, parameters);
                }
                Collector { checker, scope: &mut collected }.visit_expr(&lambda.body);
            }
        }
        let type_params = match scope {
            Scope::Function(function) => function.type_params.as_deref(),
            Scope::Class(class) => class.type_params.as_deref(),
            Scope::Lambda(_) => None,
        };
        for param in type_params.into_iter().flat_map(|type_params| &type_params.type_params) {
            collected.of(param.name().as_str()).opaque = true;
        }
        collected.finish()
    }

    fn of_body(checker: &Checker<'a>, body: &'a [Stmt]) -> Self {
        let mut collected = Collected::default();
        collected.collect(checker, body);
        collected.finish()
    }

    fn get(&self, name: &str) -> Rc<Bindings<'a>> {
        self.names.get(name).map_or_else(|| Rc::new(Bindings { opaque: self.star, ..Bindings::default() }), Rc::clone)
    }
}

// The bindings of each name as the walk finds them, before the star import of the scope reaches the names.
#[derive(Default)]
struct Collected<'a> {
    names: FxHashMap<&'a str, Bindings<'a>>,
    star: bool,
}

impl<'a> Collected<'a> {
    fn of(&mut self, name: &'a str) -> &mut Bindings<'a> {
        self.names.entry(name).or_default()
    }

    fn collect(&mut self, checker: &Checker<'a>, body: &'a [Stmt]) {
        Collector { checker, scope: self }.visit_body(body);
    }

    fn parameters(&mut self, checker: &Checker<'a>, parameters: &'a ast::Parameters) {
        let mut declared = FxHashSet::default();
        for parameter in
            parameters.iter_non_variadic_params().filter(|parameter| declared.insert(parameter.name().as_str()))
        {
            let kind = parameter.annotation().and_then(|annotation| self::annotation(checker, annotation));
            self.of(parameter.name().as_str()).declare(kind);
        }
        for variadic in [&parameters.vararg, &parameters.kwarg].into_iter().flatten() {
            self.of(variadic.name.as_str()).opaque = true;
        }
    }

    fn finish(self) -> ScopeBindings<'a> {
        let star = self.star;
        let names = self
            .names
            .into_iter()
            .map(|(name, mut bindings)| {
                bindings.opaque |= star;
                (name, Rc::new(bindings))
            })
            .collect();
        ScopeBindings { names, star }
    }
}

struct Collector<'c, 'a, 'b> {
    checker: &'c Checker<'a>,
    scope: &'b mut Collected<'a>,
}

impl<'a> Collector<'_, 'a, '_> {
    // Visits `expr` for the names that `skip` does not hold, for a statement that binds the names it skips on its own.
    fn visit_expr_except(&mut self, expr: &'a Expr, skip: impl Fn(&str) -> bool) {
        let mut nested = Collected::default();
        Collector { checker: self.checker, scope: &mut nested }.visit_expr(expr);
        for (name, bindings) in nested.names {
            if bindings.opaque && !skip(name) {
                self.scope.of(name).opaque = true;
            }
        }
    }

    fn declared_in(&mut self, body: &'a [Stmt]) {
        let mut names = FxHashSet::default();
        declared_names(body, &mut names);
        for name in names {
            self.scope.of(name).opaque = true;
        }
    }

    fn visit_generators(&mut self, generators: &'a [ast::Comprehension]) {
        for generator in generators {
            self.visit_expr(&generator.iter);
            for condition in &generator.ifs {
                self.visit_expr(condition);
            }
        }
    }
}

impl<'a> Visitor<'a> for Collector<'_, 'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            // A nested def binds its own names; only its `global` or `nonlocal` reaches back here.
            Stmt::FunctionDef(function) => {
                self.scope.of(function.name.as_str()).functions.push(function);
                self.declared_in(&function.body);
            }
            Stmt::ClassDef(class) => {
                self.scope.of(class.name.as_str()).classes.push(class);
                self.declared_in(&class.body);
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                for target in targets {
                    match target {
                        Expr::Name(name) => self.scope.of(name.id.as_str()).values.push(Value::Expr(value)),
                        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                            let unpacked: FxHashSet<&str> =
                                elts.iter().filter_map(Expr::as_name_expr).map(|name| name.id.as_str()).collect();
                            for &name in &unpacked {
                                self.scope.of(name).values.push(Value::Item(value));
                            }
                            for elt in elts.iter().filter(|elt| elt.as_name_expr().is_none()) {
                                self.visit_expr_except(elt, |name| unpacked.contains(name));
                            }
                        }
                        _ => self.visit_expr(target),
                    }
                }
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) => match target.as_name_expr() {
                Some(name) => {
                    let name = name.id.as_str();
                    // A bare `Final` leaves the type to the value.
                    if self.checker.semantic().match_typing_expr(annotation, "Final") {
                        self.scope.of(name).values.extend(value.as_deref().map(Value::Expr));
                    } else {
                        let kind = self::annotation(self.checker, annotation);
                        self.scope.of(name).declare(kind);
                    }
                    if let Some(value) = value {
                        self.visit_expr(value);
                    }
                    self.visit_expr_except(annotation, |other| other == name);
                }
                None => visitor::walk_stmt(self, stmt),
            },
            Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) => match target.as_name_expr() {
                Some(name) => {
                    let bindings = self.scope.of(name.id.as_str());
                    if *op == Operator::Add {
                        bindings.values.push(Value::Expr(value));
                    } else {
                        bindings.opaque = true;
                    }
                    self.visit_expr(value);
                }
                None => visitor::walk_stmt(self, stmt),
            },
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => match target.as_name_expr() {
                Some(name) => {
                    self.scope.of(name.id.as_str()).values.push(Value::Item(iter));
                    self.visit_expr(iter);
                    self.visit_body(body);
                    self.visit_body(orelse);
                }
                None => visitor::walk_stmt(self, stmt),
            },
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    match item.optional_vars.as_deref().and_then(Expr::as_name_expr) {
                        Some(name) => {
                            self.scope.of(name.id.as_str()).values.push(Value::Entered(&item.context_expr));
                            self.visit_expr(&item.context_expr);
                        }
                        None => self.visit_with_item(item),
                    }
                }
                self.visit_body(body);
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                for declared in names {
                    self.scope.of(declared.as_str()).opaque = true;
                }
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                let from = matches!(stmt, Stmt::ImportFrom(_));
                for alias in names {
                    let bound = alias_binding(alias);
                    if bound == "*" {
                        self.scope.star = true;
                    } else {
                        self.scope.of(bound.split_once('.').map_or(bound, |(first, _)| first)).imported = true;
                        if from {
                            self.scope.of(bound).original = Some(alias.name.as_str());
                        }
                    }
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx, .. }) => {
                if *ctx != ExprContext::Load {
                    self.scope.of(id.as_str()).opaque = true;
                }
            }
            Expr::Lambda(_) => {}
            // A comprehension binds its targets in a scope of its own; only a `:=` inside reaches this one.
            Expr::ListComp(ast::ExprListComp { elt, generators, .. })
            | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
            | Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => {
                self.visit_expr(elt);
                self.visit_generators(generators);
            }
            Expr::DictComp(ast::ExprDictComp { key, value, generators, .. }) => {
                if let Some(key) = key {
                    self.visit_expr(key);
                }
                self.visit_expr(value);
                self.visit_generators(generators);
            }
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: bound, .. }) = handler;
        if let Some(bound) = bound {
            self.scope.of(bound.as_str()).opaque = true;
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let bound = match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. })
            | Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => name.as_ref(),
            _ => None,
        };
        if let Some(bound) = bound {
            self.scope.of(bound.as_str()).opaque = true;
        }
        visitor::walk_pattern(self, pattern);
    }
}

// The names a `global` or `nonlocal` declares in `body`, nested bodies included.
fn declared_names<'a>(body: &'a [Stmt], names: &mut FxHashSet<&'a str>) {
    for stmt in body {
        match stmt {
            Stmt::Global(ast::StmtGlobal { names: declared, .. })
            | Stmt::Nonlocal(ast::StmtNonlocal { names: declared, .. }) => {
                names.extend(declared.iter().map(ast::Identifier::as_str));
            }
            _ => for_each_child_body(stmt, &mut |child| declared_names(child, names)),
        }
    }
}
