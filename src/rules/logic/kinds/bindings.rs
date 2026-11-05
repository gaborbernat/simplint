use std::rc::Rc;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Operator, Pattern, Stmt};
use ruff_text_size::TextRange;

use super::Kind;
use super::declared::annotation;
use super::query::type_params_bind;
use crate::lint::checker::Checker;
use crate::lint::walker::Scope;
use crate::syntax::{alias_binding, for_each_child_body, is_name};

// The bindings of `name` in `scope`, or in the module for none.
pub(super) fn bindings_in<'a>(checker: &Checker<'a>, scope: Option<Scope<'a>>, name: &'a str) -> Rc<Bindings<'a>> {
    let range = match scope {
        Some(Scope::Function(function)) => function.range,
        Some(Scope::Class(class)) => class.range,
        Some(Scope::Lambda(lambda)) => lambda.range,
        None => TextRange::default(),
    };
    if let Some(bindings) = checker.state().kinds().bindings.borrow().get(&(range, name)) {
        return Rc::clone(bindings);
    }
    let bindings = Rc::new(match scope {
        Some(scope) => Bindings::of_scope(checker, scope, name),
        None => Bindings::of_body(checker, checker.body(), name),
    });
    checker.state().kinds().bindings.borrow_mut().insert((range, name), Rc::clone(&bindings));
    bindings
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
    pub(super) opaque: bool,
}

impl<'a> Bindings<'a> {
    pub(super) fn of_scope(checker: &Checker<'a>, scope: Scope<'a>, name: &str) -> Self {
        let type_param = type_params_bind(&scope, name);
        let mut bindings = match scope {
            Scope::Function(function) => {
                let mut bindings = Self::of_parameters(checker, &function.parameters, name);
                bindings.collect(checker, &function.body, name);
                bindings
            }
            Scope::Class(class) => Self::of_body(checker, &class.body, name),
            Scope::Lambda(lambda) => {
                let mut bindings = lambda
                    .parameters
                    .as_deref()
                    .map(|parameters| Self::of_parameters(checker, parameters, name))
                    .unwrap_or_default();
                Collector { checker, name, bindings: &mut bindings }.visit_expr(&lambda.body);
                bindings
            }
        };
        bindings.opaque |= type_param;
        bindings
    }

    pub(super) fn of_body(checker: &Checker<'a>, body: &'a [Stmt], name: &str) -> Self {
        let mut bindings = Self::default();
        bindings.collect(checker, body, name);
        bindings
    }

    pub(super) fn of_parameters(checker: &Checker, parameters: &ast::Parameters, name: &str) -> Self {
        let mut bindings = Self::default();
        if let Some(parameter) = parameters.iter_non_variadic_params().find(|parameter| parameter.name() == name) {
            bindings.declare(parameter.annotation().and_then(|annotation| self::annotation(checker, annotation)));
        }
        bindings.opaque |= [&parameters.vararg, &parameters.kwarg]
            .into_iter()
            .flatten()
            .any(|parameter| parameter.name.as_str() == name);
        bindings
    }

    pub(super) fn collect(&mut self, checker: &Checker<'a>, body: &'a [Stmt], name: &str) {
        Collector { checker, name, bindings: self }.visit_body(body);
    }

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

pub(super) struct Collector<'c, 'a, 'b> {
    pub(super) checker: &'c Checker<'a>,
    pub(super) name: &'c str,
    pub(super) bindings: &'b mut Bindings<'a>,
}

impl<'a> Visitor<'a> for Collector<'_, 'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let name = self.name;
        match stmt {
            // A nested def binds its own names; only its `global` or `nonlocal` reaches back here.
            Stmt::FunctionDef(function) => {
                if function.name.as_str() == name {
                    self.bindings.functions.push(function);
                }
                self.bindings.opaque |= declares(&function.body, name);
            }
            Stmt::ClassDef(class) => {
                if class.name.as_str() == name {
                    self.bindings.classes.push(class);
                }
                self.bindings.opaque |= declares(&class.body, name);
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                for target in targets {
                    if is_name(target, name) {
                        self.bindings.values.push(Value::Expr(value));
                    } else if unpacks(target, name) {
                        self.bindings.values.push(Value::Item(value));
                    } else {
                        self.visit_expr(target);
                    }
                }
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) if is_name(target, name) => {
                // A bare `Final` leaves the type to the value.
                if self.checker.semantic().match_typing_expr(annotation, "Final") {
                    self.bindings.values.extend(value.as_deref().map(Value::Expr));
                } else {
                    self.bindings.declare(self::annotation(self.checker, annotation));
                }
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) if is_name(target, name) => {
                if *op == Operator::Add {
                    self.bindings.values.push(Value::Expr(value));
                } else {
                    self.bindings.opaque = true;
                }
                self.visit_expr(value);
            }
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) if is_name(target, name) => {
                self.bindings.values.push(Value::Item(iter));
                self.visit_expr(iter);
                self.visit_body(body);
                self.visit_body(orelse);
            }
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    if item.optional_vars.as_deref().is_some_and(|target| is_name(target, name)) {
                        self.bindings.values.push(Value::Entered(&item.context_expr));
                        self.visit_expr(&item.context_expr);
                    } else {
                        self.visit_with_item(item);
                    }
                }
                self.visit_body(body);
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.bindings.opaque |= names.iter().any(|declared| declared.as_str() == name);
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    let bound = alias_binding(alias);
                    self.bindings.opaque |= bound == "*";
                    self.bindings.imported |= bound.split('.').next() == Some(name);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx, .. }) => {
                self.bindings.opaque |= id == self.name && *ctx != ExprContext::Load;
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
        self.bindings.opaque |= bound.as_ref().is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let bound = match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. })
            | Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => name.as_ref(),
            _ => None,
        };
        self.bindings.opaque |= bound.is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_pattern(self, pattern);
    }
}

impl<'a> Collector<'_, 'a, '_> {
    pub(super) fn visit_generators(&mut self, generators: &'a [ast::Comprehension]) {
        for generator in generators {
            self.visit_expr(&generator.iter);
            for condition in &generator.ifs {
                self.visit_expr(condition);
            }
        }
    }
}

// Whether `target` unpacks an item straight into `name`, as `a, b = ...` does.
pub(super) fn unpacks(target: &Expr, name: &str) -> bool {
    match target {
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().any(|elt| is_name(elt, name))
        }
        _ => false,
    }
}

// Whether a `global` or `nonlocal` anywhere in `body`, nested defs included, declares `name`.
pub(super) fn declares(body: &[Stmt], name: &str) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
            names.iter().any(|declared| declared.as_str() == name)
        }
        _ => {
            let mut found = false;
            for_each_child_body(stmt, &mut |child| found |= declares(child, name));
            found
        }
    })
}
