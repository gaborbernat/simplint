use std::collections::{HashMap, HashSet};

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Parameters, Pattern, Stmt};
use ruff_text_size::TextRange;

#[derive(Default)]
pub struct Semantic<'a> {
    pub bindings: HashMap<&'a str, Vec<Binding>>,
    pub references: Vec<Reference<'a>>,
    pub member_references: Vec<MemberReference<'a>>,
    global_declarations: Vec<(&'a str, TextRange)>,
}

impl<'a> Semantic<'a> {
    pub fn new(body: &'a [Stmt]) -> Self {
        let mut collector = Collector {
            scopes: vec![Scope::Module],
            evaluation: Evaluation::Runtime,
            statement: 0,
            function: None,
            classes: Vec::new(),
            store_kind: BindingKind::Other,
            semantic: Semantic::default(),
        };
        for (index, stmt) in body.iter().enumerate() {
            collector.statement = index;
            collector.visit_stmt(stmt);
        }
        collector.semantic
    }

    pub fn references_to(&self, name: &str) -> impl Iterator<Item = &Reference<'a>> {
        self.references.iter().filter(move |reference| reference.name == name)
    }

    pub fn is_rebound(&self, name: &str) -> bool {
        self.bindings.get(name).is_some_and(|bindings| bindings.len() > 1)
    }

    pub fn is_bound(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn name_ranges(&self, name: &str) -> Vec<TextRange> {
        let mut ranges: Vec<TextRange> = self
            .bindings
            .get(name)
            .into_iter()
            .flatten()
            .map(|binding| binding.range)
            .chain(self.references_to(name).map(|reference| reference.range))
            .chain(self.global_declarations.iter().filter(|(declared, _)| *declared == name).map(|(_, range)| *range))
            .collect();
        ranges.sort_by_key(ruff_text_size::Ranged::start);
        ranges.dedup();
        ranges
    }
}

pub struct Binding {
    pub range: TextRange,
    pub kind: BindingKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Assignment,
    Annotated,
    Augmented,
    Import,
    Function,
    Class,
    Target,
    Other,
}

#[derive(Clone, Copy)]
pub struct Reference<'a> {
    pub name: &'a str,
    pub range: TextRange,
    pub statement: usize,
    pub evaluation: Evaluation,
    pub function: Option<TextRange>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    Runtime,
    Deferred,
    Annotation,
}

pub struct MemberReference<'a> {
    pub class: TextRange,
    pub name: &'a str,
    pub range: TextRange,
}

struct Collector<'a> {
    scopes: Vec<Scope<'a>>,
    evaluation: Evaluation,
    statement: usize,
    function: Option<TextRange>,
    classes: Vec<(&'a str, TextRange)>,
    store_kind: BindingKind,
    semantic: Semantic<'a>,
}

enum Scope<'a> {
    Module,
    Class { locals: HashSet<&'a str> },
    Function { locals: HashSet<&'a str>, globals: HashSet<&'a str> },
    Lambda { locals: HashSet<&'a str> },
    Comprehension { locals: HashSet<&'a str> },
}

impl<'a> Visitor<'a> for Collector<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                for decorator in &function.decorator_list {
                    self.visit_decorator(decorator);
                }
                self.visit_parameters_outside(&function.parameters);
                if let Some(returns) = &function.returns {
                    self.visit_annotation(returns);
                }
                self.bind(function.name.as_str(), function.name.range, BindingKind::Function);
                let (locals, globals) = Locals::of_function(&function.parameters, &function.body);
                let saved_function = self.function;
                if matches!(self.scopes.last(), Some(Scope::Class { .. })) && self.function.is_none() {
                    self.function = Some(function.range);
                }
                self.enter_function(function.range, Scope::Function { locals, globals }, |collector| {
                    collector.visit_body(&function.body);
                });
                self.function = saved_function;
            }
            Stmt::ClassDef(class) => {
                for decorator in &class.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(arguments) = &class.arguments {
                    self.visit_arguments(arguments);
                }
                self.bind(class.name.as_str(), class.name.range, BindingKind::Class);
                self.scopes.push(Scope::Class { locals: Locals::of_body(&class.body) });
                self.classes.push((class.name.as_str(), class.range));
                self.visit_body(&class.body);
                self.classes.pop();
                self.scopes.pop();
            }
            Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    self.bind(import_binding(alias), alias.range, BindingKind::Import);
                }
            }
            Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    let identifier = alias.asname.as_ref().unwrap_or(&alias.name);
                    self.bind(identifier.as_str(), identifier.range, BindingKind::Import);
                }
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) => {
                for name in names {
                    self.semantic.global_declarations.push((name.as_str(), name.range));
                }
            }
            Stmt::Assign(_) | Stmt::AnnAssign(_) | Stmt::AugAssign(_) => {
                self.store_kind = match stmt {
                    Stmt::Assign(_) => BindingKind::Assignment,
                    Stmt::AnnAssign(_) => BindingKind::Annotated,
                    _ => BindingKind::Augmented,
                };
                visitor::walk_stmt(self, stmt);
                self.store_kind = BindingKind::Other;
            }
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => {
                self.visit_expr(iter);
                self.visit_target(target);
                self.visit_body(body);
                self.visit_body(orelse);
            }
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    self.visit_expr(&item.context_expr);
                    if let Some(target) = &item.optional_vars {
                        self.visit_target(target);
                    }
                }
                self.visit_body(body);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        let saved = std::mem::replace(&mut self.evaluation, Evaluation::Annotation);
        self.visit_expr(expr);
        self.evaluation = saved;
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, range, .. }) => self.reference(id.as_str(), *range),
            Expr::Name(ast::ExprName { id, range, .. }) => {
                let kind = std::mem::replace(&mut self.store_kind, BindingKind::Other);
                self.bind(id.as_str(), *range, kind);
                self.store_kind = kind;
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                self.member_reference(value, attr);
                self.visit_expr(value);
            }
            Expr::Lambda(lambda) => {
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameters_outside(parameters);
                }
                let locals = lambda
                    .parameters
                    .iter()
                    .flat_map(|parameters| parameters.iter())
                    .map(|parameter| parameter.name().as_str());
                self.enter_function(lambda.range, Scope::Lambda { locals: locals.collect() }, |collector| {
                    collector.visit_expr(&lambda.body);
                });
            }
            Expr::ListComp(ast::ExprListComp { elt, generators, .. })
            | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
            | Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => {
                self.visit_comprehensions(generators, &[elt]);
            }
            Expr::DictComp(ast::ExprDictComp { key, value, generators, .. }) => {
                let elements: Vec<&Expr> = key.as_deref().into_iter().chain([value.as_ref()]).collect();
                self.visit_comprehensions(generators, &elements);
            }
            Expr::Named(ast::ExprNamed { target, value, .. }) => {
                self.visit_expr(value);
                if let Expr::Name(ast::ExprName { id, range, .. }) = target.as_ref() {
                    self.bind(id.as_str(), *range, BindingKind::Other);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(handler_node) = handler;
        if let Some(name) = &handler_node.name {
            self.bind(name.as_str(), name.range, BindingKind::Target);
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        for name in pattern_names(pattern) {
            self.bind(name.as_str(), name.range, BindingKind::Target);
        }
        visitor::walk_pattern(self, pattern);
    }
}

impl<'a> Collector<'a> {
    fn visit_target(&mut self, target: &'a Expr) {
        let saved = std::mem::replace(&mut self.store_kind, BindingKind::Target);
        self.visit_expr(target);
        self.store_kind = saved;
    }

    fn visit_parameters_outside(&mut self, parameters: &'a Parameters) {
        for default in parameters.iter_non_variadic_params().filter_map(|parameter| parameter.default.as_deref()) {
            self.visit_expr(default);
        }
        for annotation in parameters.iter().filter_map(ruff_python_ast::AnyParameterRef::annotation) {
            self.visit_annotation(annotation);
        }
    }

    fn bind(&mut self, name: &'a str, range: TextRange, kind: BindingKind) {
        let module_level = match self.scopes.iter().rev().find(|scope| !matches!(scope, Scope::Comprehension { .. })) {
            Some(Scope::Module) => true,
            Some(Scope::Function { globals, .. }) => globals.contains(name),
            _ => false,
        };
        if module_level {
            self.semantic.bindings.entry(name).or_default().push(Binding { range, kind });
        }
    }

    fn enter_function(&mut self, range: TextRange, scope: Scope<'a>, visit: impl FnOnce(&mut Self)) {
        let saved_evaluation = std::mem::replace(&mut self.evaluation, Evaluation::Deferred);
        let saved_function = self.function;
        self.function = Some(saved_function.unwrap_or(range));
        self.scopes.push(scope);
        visit(self);
        self.scopes.pop();
        self.function = saved_function;
        self.evaluation = saved_evaluation;
    }

    fn reference(&mut self, name: &'a str, range: TextRange) {
        if self.resolves_to_module(name) {
            self.semantic.references.push(Reference {
                name,
                range,
                statement: self.statement,
                evaluation: self.evaluation,
                function: self.function,
            });
        }
    }

    fn resolves_to_module(&self, name: &str) -> bool {
        let innermost = self.scopes.len() - 1;
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, scope)| match scope {
                Scope::Module => Some(true),
                Scope::Class { locals } if index == innermost && locals.contains(name) => Some(false),
                Scope::Function { globals, .. } if globals.contains(name) => Some(true),
                Scope::Function { locals, .. } | Scope::Lambda { locals } | Scope::Comprehension { locals }
                    if locals.contains(name) =>
                {
                    Some(false)
                }
                Scope::Class { .. } | Scope::Function { .. } | Scope::Lambda { .. } | Scope::Comprehension { .. } => {
                    None
                }
            })
            .unwrap_or(true)
    }

    fn member_reference(&mut self, value: &Expr, attr: &'a ast::Identifier) {
        let (Expr::Name(ast::ExprName { id, .. }), Some((class_name, class)), Some(_)) =
            (value, self.classes.last().copied(), self.function)
        else {
            return;
        };
        if matches!(id.as_str(), "self" | "cls") || id.as_str() == class_name {
            self.semantic.member_references.push(MemberReference { class, name: attr.as_str(), range: attr.range });
        }
    }

    fn visit_comprehensions(&mut self, generators: &'a [ast::Comprehension], elements: &[&'a Expr]) {
        let (first, rest) = (&generators[0], &generators[1..]);
        self.visit_expr(&first.iter);
        let mut locals = HashSet::new();
        for generator in generators {
            collect_targets(&generator.target, &mut locals);
        }
        self.scopes.push(Scope::Comprehension { locals });
        for expr in &first.ifs {
            self.visit_expr(expr);
        }
        for generator in rest {
            self.visit_expr(&generator.iter);
            for expr in &generator.ifs {
                self.visit_expr(expr);
            }
        }
        for element in elements {
            self.visit_expr(element);
        }
        self.scopes.pop();
    }
}

#[derive(Default)]
struct Locals<'a> {
    locals: HashSet<&'a str>,
    globals: HashSet<&'a str>,
}

impl<'a> Locals<'a> {
    fn of_function(parameters: &'a Parameters, body: &'a [Stmt]) -> (HashSet<&'a str>, HashSet<&'a str>) {
        let mut collector = Self::default();
        collector.locals.extend(parameters.iter().map(|parameter| parameter.name().as_str()));
        collector.visit_body(body);
        let globals = collector.globals;
        let locals = collector.locals.into_iter().filter(|name| !globals.contains(name)).collect();
        (locals, globals)
    }

    fn of_body(body: &'a [Stmt]) -> HashSet<&'a str> {
        let mut collector = Self::default();
        collector.visit_body(body);
        collector.locals
    }
}

impl<'a> Visitor<'a> for Locals<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
                self.locals.insert(name.as_str());
            }
            Stmt::Import(ast::StmtImport { names, .. }) => self.locals.extend(names.iter().map(import_binding)),
            Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.locals.extend(names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()));
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) => {
                self.globals.extend(names.iter().map(ast::Identifier::as_str));
            }
            Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.locals.extend(names.iter().map(ast::Identifier::as_str));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.locals.insert(id.as_str());
            }
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_comprehension(&mut self, comprehension: &'a ast::Comprehension) {
        self.visit_expr(&comprehension.iter);
        for expr in &comprehension.ifs {
            self.visit_expr(expr);
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(handler_node) = handler;
        if let Some(name) = &handler_node.name {
            self.locals.insert(name.as_str());
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        self.locals.extend(pattern_names(pattern).into_iter().map(ast::Identifier::as_str));
        visitor::walk_pattern(self, pattern);
    }
}

fn import_binding(alias: &ast::Alias) -> &str {
    match &alias.asname {
        Some(asname) => asname.as_str(),
        None => alias.name.as_str().split('.').next().unwrap_or_default(),
    }
}

fn pattern_names(pattern: &Pattern) -> Vec<&ast::Identifier> {
    match pattern {
        Pattern::MatchAs(ast::PatternMatchAs { name: Some(name), .. })
        | Pattern::MatchStar(ast::PatternMatchStar { name: Some(name), .. })
        | Pattern::MatchMapping(ast::PatternMatchMapping { rest: Some(name), .. }) => vec![name],
        _ => Vec::new(),
    }
}

fn collect_targets<'a>(target: &'a Expr, names: &mut HashSet<&'a str>) {
    match target {
        Expr::Name(ast::ExprName { id, .. }) => {
            names.insert(id.as_str());
        }
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            for element in elts {
                collect_targets(element, names);
            }
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => collect_targets(value, names),
        _ => {}
    }
}
