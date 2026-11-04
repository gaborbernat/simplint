use std::collections::HashMap;
use std::path::Path;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};

use super::facts::qualified;
use crate::syntax::last_segment;

// The attributes code sets or deletes on objects, keyed by the class of the object when parsing tells it: the receiver
// is `self` in a method, or a name bound by a call to a class or annotated with one. A class goes by its qualified
// name, such as `pkg.models.Result`, as two modules can each define a class with the same name. A mock accepts any attribute and
// is never a real instance, so writes to one are left out; writes to anything else are untyped. Reaching the instance
// `__dict__`, as `obj.__dict__` or `vars(obj)` do, counts as writing `__dict__`.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AttributeWrites {
    pub(super) typed: Vec<(String, String)>,
    pub(super) untyped: Vec<String>,
    // The qualified bases of classes a slotted base breaks: one using `functools.cached_property`, which stores into an
    // instance `__dict__`, or one with several bases, as a slotted base's layout conflicts with another's.
    pub(super) unslottable_bases: Vec<String>,
    // The qualified bases of classes whose methods store attributes on `self`. The `__getstate__` a frozen slotted
    // dataclass gets pickles only fields, so a pickled instance of such a subclass loses the others.
    pub(super) stateful_bases: Vec<String>,
    // Whether the file pickles with a protocol that may be 0 or 1, which cannot pickle a slotted class without its own
    // `__getstate__`, as a dataclass gets only when frozen.
    pub(super) pickles_low_protocol: bool,
}

impl AttributeWrites {
    pub(super) fn collect(body: &[Stmt], aliases: &HashMap<&str, String>, module: &str) -> Self {
        let mut collector = Collector { aliases, module, scopes: Vec::new(), writes: Self::default() };
        let bindings = collector.bindings(body, &[]);
        collector.scopes.push(Scope { kind: ScopeKind::Plain, bindings });
        collector.visit_body(body);
        collector.writes
    }
}

// What a receiver is known to be.
#[derive(Clone, PartialEq, Eq)]
enum Receiver {
    // An instance of the class named as written here.
    Instance(String),
    // `self`, an instance of the class around the method, by its qualified name, which no import can shadow.
    Own(String),
    Mock,
    Unknown,
}

const MOCKS: [&str; 7] =
    ["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock", "PropertyMock", "create_autospec"];

struct Scope<'a> {
    kind: ScopeKind<'a>,
    bindings: HashMap<&'a str, Receiver>,
}

// A class scope keeps what its methods store on `self`; names a class body binds are not visible to its methods.
enum ScopeKind<'a> {
    Plain,
    Class { name: &'a str, members: HashMap<&'a str, Receiver> },
}

struct Collector<'a, 'm> {
    aliases: &'m HashMap<&'a str, String>,
    module: &'m str,
    scopes: Vec<Scope<'a>>,
    writes: AttributeWrites,
}

impl<'a> Visitor<'a> for Collector<'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                for decorator in &function.decorator_list {
                    self.visit_decorator(decorator);
                }
                self.visit_parameters(&function.parameters);
                if let Some(returns) = &function.returns {
                    self.visit_annotation(returns);
                }
                let parameters = self.parameters(function);
                let bindings = self.bindings(&function.body, &parameters);
                self.scopes.push(Scope { kind: ScopeKind::Plain, bindings });
                self.visit_body(&function.body);
                self.scopes.pop();
            }
            Stmt::ClassDef(class) => {
                let bases: Vec<String> = class
                    .bases()
                    .iter()
                    .filter_map(|base| {
                        let written = UnqualifiedName::from_expr(base)?.segments().join(".");
                        Some(self.qualified(&written))
                    })
                    .collect();
                let members = members(class);
                if !members.is_empty() {
                    self.writes.stateful_bases.extend(bases.iter().cloned());
                }
                if class.bases().len() > 1 || class_mentions(class, "cached_property") {
                    self.writes.unslottable_bases.extend(bases);
                }
                for decorator in &class.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(arguments) = &class.arguments {
                    self.visit_arguments(arguments);
                }
                let bindings = self.bindings(&class.body, &[]);
                self.scopes.push(Scope { kind: ScopeKind::Class { name: class.name.as_str(), members }, bindings });
                self.visit_body(&class.body);
                self.scopes.pop();
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.record(value, attr.as_str());
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "__dict__" => {
                self.record(value, "__dict__");
            }
            Expr::Call(call @ ast::ExprCall { func, arguments, .. }) => {
                let builtin = func.as_name_expr().map(|name| name.id.as_str());
                match (&*arguments.args, builtin) {
                    ([target, Expr::StringLiteral(attribute), ..], Some("setattr" | "delattr")) => {
                        self.record(target, attribute.value.to_str());
                    }
                    ([target], Some("vars")) => self.record(target, "__dict__"),
                    _ => {}
                }
                self.writes.pickles_low_protocol |= self.pickles_low_protocol(call);
            }
            // A lambda's parameters and a comprehension's targets shadow the names around them.
            Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                let mut scan = Bindings { collector: self, bound: HashMap::new() };
                visitor::walk_expr(&mut scan, expr);
                let parameters = expr.as_lambda_expr().and_then(|lambda| lambda.parameters.as_deref());
                for parameter in parameters.into_iter().flat_map(|parameters| parameters.iter()) {
                    scan.bind(parameter.name().as_str(), Receiver::Unknown);
                }
                let bindings = scan.bound;
                self.scopes.push(Scope { kind: ScopeKind::Plain, bindings });
                visitor::walk_expr(self, expr);
                self.scopes.pop();
                return;
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

impl<'a> Collector<'a, '_> {
    fn record(&mut self, receiver: &Expr, attribute: &str) {
        match self.receiver(receiver) {
            Receiver::Instance(class) => self.writes.typed.push((self.qualified(&class), attribute.to_owned())),
            Receiver::Own(class) => self.writes.typed.push((class, attribute.to_owned())),
            Receiver::Mock => {}
            Receiver::Unknown => self.writes.untyped.push(attribute.to_owned()),
        }
    }

    // `pickle.dumps(obj, protocol)`, `pickle.dump(obj, file, protocol)` or `pickle.Pickler(file, protocol)` with a
    // protocol that is not known to be 2 or above, such as a test looping over every protocol.
    fn pickles_low_protocol(&self, call: &ast::ExprCall) -> bool {
        let Some(written) = UnqualifiedName::from_expr(&call.func) else {
            return false;
        };
        let position = match self.qualified(&written.segments().join(".")).as_str() {
            "pickle.dumps" | "pickle.Pickler" => 1,
            "pickle.dump" => 2,
            _ => return false,
        };
        let protocol = call.arguments.find_argument_value("protocol", position);
        protocol.is_some_and(|protocol| match protocol {
            Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(value), .. }) => {
                value.as_u8().is_none_or(|value| value < 2)
            }
            other => !last_segment(other).is_some_and(|name| matches!(name, "HIGHEST_PROTOCOL" | "DEFAULT_PROTOCOL")),
        })
    }

    // A class name as written here, qualified through the imports that bind its head, or else by this module.
    fn qualified(&self, written: &str) -> String {
        let segments: Vec<&str> = written.split('.').collect();
        qualified(&segments, self.aliases).unwrap_or_else(|| format!("{}.{written}", self.module))
    }

    // Anything reached from a mock, such as `client.return_value.session`, is a mock too.
    fn receiver(&self, expr: &Expr) -> Receiver {
        match expr {
            Expr::Name(name) => self.lookup(name.id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => match self.receiver(value) {
                Receiver::Mock => Receiver::Mock,
                Receiver::Instance(class) | Receiver::Own(class) => {
                    self.member(class.rsplit('.').next().unwrap_or_default(), attr.as_str())
                }
                Receiver::Unknown => Receiver::Unknown,
            },
            Expr::Call(ast::ExprCall { func, .. }) | Expr::Subscript(ast::ExprSubscript { value: func, .. })
                if self.receiver(func) == Receiver::Mock || assigned(expr) == Receiver::Mock =>
            {
                Receiver::Mock
            }
            _ => Receiver::Unknown,
        }
    }

    // A function sees the names of the enclosing functions and the module, not those of a class body around it.
    fn lookup(&self, name: &str) -> Receiver {
        let innermost = self.scopes.len() - 1;
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .filter(|(depth, scope)| *depth == innermost || matches!(scope.kind, ScopeKind::Plain))
            .find_map(|(_, scope)| scope.bindings.get(name).cloned())
            .unwrap_or(Receiver::Unknown)
    }

    // What the methods of the innermost class named `class` store on `self.{attribute}`.
    fn member(&self, class: &str, attribute: &str) -> Receiver {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| match &scope.kind {
                ScopeKind::Class { name, members } if *name == class => members.get(attribute).cloned(),
                _ => None,
            })
            .unwrap_or(Receiver::Unknown)
    }

    // A method's first parameter is the instance, unless a decorator makes it a class or static method.
    fn parameters(&self, function: &'a ast::StmtFunctionDef) -> Vec<(&'a str, Receiver)> {
        let class = match self.scopes.last().map(|scope| &scope.kind) {
            Some(ScopeKind::Class { name, .. }) => Some(*name),
            _ => None,
        };
        let is_method = class.is_some()
            && !function.decorator_list.iter().any(|decorator| {
                last_segment(&decorator.expression).is_some_and(|name| matches!(name, "staticmethod" | "classmethod"))
            });
        function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let receiver = match (index, class) {
                    (0, Some(class)) if is_method && !parameter.is_variadic() => {
                        Receiver::Own(format!("{}.{class}", self.module))
                    }
                    _ => parameter.annotation().map_or(Receiver::Unknown, annotated),
                };
                (parameter.name().as_str(), receiver)
            })
            .collect()
    }

    fn bindings(&self, body: &'a [Stmt], parameters: &[(&'a str, Receiver)]) -> HashMap<&'a str, Receiver> {
        let mut scan = Bindings { collector: self, bound: HashMap::new() };
        for (name, receiver) in parameters {
            scan.bind(name, receiver.clone());
        }
        scan.visit_body(body);
        scan.bound
    }

    // `with patch(...) as name` binds the mock that replaces the target.
    fn context(&self, value: &Expr) -> Receiver {
        let Expr::Call(ast::ExprCall { func, .. }) = value else {
            return Receiver::Unknown;
        };
        let target = match func.as_ref() {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "object" => value.as_ref(),
            other => other,
        };
        let module = UnqualifiedName::from_expr(target)
            .map(|name| qualified(name.segments(), self.aliases).unwrap_or_else(|| name.segments().join(".")));
        match module.as_deref() {
            Some("unittest.mock.patch" | "mock.patch") => Receiver::Mock,
            _ => assigned(value),
        }
    }
}

// The names a scope binds and what each holds; a name bound more than one way, or in a way parsing cannot follow,
// holds something unknown. Nested functions and classes are scopes of their own.
struct Bindings<'a, 'c, 'm> {
    collector: &'c Collector<'a, 'm>,
    bound: HashMap<&'a str, Receiver>,
}

impl<'a> Bindings<'a, '_, '_> {
    fn bind(&mut self, name: &'a str, receiver: Receiver) {
        bind(&mut self.bound, name, receiver);
    }
}

// A name bound to two different receivers holds something unknown.
fn bind<'a>(bound: &mut HashMap<&'a str, Receiver>, name: &'a str, receiver: Receiver) {
    bound
        .entry(name)
        .and_modify(|known| {
            if *known != receiver {
                *known = Receiver::Unknown;
            }
        })
        .or_insert(receiver);
}

impl<'a> Visitor<'a> for Bindings<'a, '_, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
                self.bind(name.as_str(), Receiver::Unknown);
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if let [Expr::Name(target)] = targets.as_slice() => {
                self.bind(target.id.as_str(), assigned(value));
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. })
                if let Expr::Name(target) = target.as_ref() =>
            {
                let receiver = match annotated(annotation) {
                    Receiver::Unknown => value.as_deref().map_or(Receiver::Unknown, assigned),
                    known => known,
                };
                self.bind(target.id.as_str(), receiver);
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    self.visit_expr(&item.context_expr);
                    match item.optional_vars.as_deref() {
                        Some(Expr::Name(target)) => {
                            self.bind(target.id.as_str(), self.collector.context(&item.context_expr));
                        }
                        Some(other) => self.visit_expr(other),
                        None => {}
                    }
                }
                self.visit_body(body);
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                    self.bind(bound.split('.').next().unwrap_or(bound), Receiver::Unknown);
                }
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                for name in names {
                    self.bind(name.as_str(), Receiver::Unknown);
                }
            }
            Stmt::Try(ast::StmtTry { handlers, .. }) => {
                for handler in handlers {
                    if let Some(name) = handler.as_except_handler().and_then(|handler| handler.name.as_ref()) {
                        self.bind(name.as_str(), Receiver::Unknown);
                    }
                }
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.bind(id.as_str(), Receiver::Unknown);
            }
            Expr::Named(ast::ExprNamed { target, value, .. }) if let Expr::Name(target) = target.as_ref() => {
                self.bind(target.id.as_str(), assigned(value));
                self.visit_expr(value);
            }
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let captured = match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. }) => name.as_ref(),
            Pattern::MatchMapping(ast::PatternMatchMapping { rest, .. }) => rest.as_ref(),
            _ => None,
        };
        if let Some(name) = captured {
            self.bind(name.as_str(), Receiver::Unknown);
        }
        visitor::walk_pattern(self, pattern);
    }
}

// What the methods of a class assign to `self.{name}`, each a single target.
struct Members<'a> {
    members: HashMap<&'a str, Receiver>,
}

impl<'a> Visitor<'a> for Members<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(_) => {}
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if let [target] = targets.as_slice() => {
                self.member(target, assigned(value));
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) => {
                self.member(target, annotated(annotation));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) =
            expr
            && value.as_name_expr().is_some_and(|name| name.id.as_str() == "self")
        {
            self.members.insert(attr.as_str(), Receiver::Unknown);
        }
        visitor::walk_expr(self, expr);
    }
}

impl<'a> Members<'a> {
    fn member(&mut self, target: &'a Expr, receiver: Receiver) {
        if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = target
            && value.as_name_expr().is_some_and(|name| name.id.as_str() == "self")
        {
            bind(&mut self.members, attr.as_str(), receiver);
        } else {
            self.visit_expr(target);
        }
    }
}

// What the methods of `class` store on `self`, so `self.client.name = value` knows when `client` is a mock. An
// annotation in the class body, such as `client: Client`, declares what its methods store, whatever they assign.
fn members(class: &ast::StmtClassDef) -> HashMap<&str, Receiver> {
    let mut scan = Members { members: HashMap::new() };
    scan.visit_body(&class.body);
    for stmt in &class.body {
        if let Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) = stmt
            && let Expr::Name(name) = target.as_ref()
            && scan.members.contains_key(name.id.as_str())
            && let declared @ Receiver::Instance(_) = annotated(annotation)
        {
            scan.members.insert(name.id.as_str(), declared);
        }
    }
    scan.members
}

// What a call binds: a mock from a mock class or `mocker.patch`, otherwise an instance of the class it names.
fn assigned(value: &Expr) -> Receiver {
    let Expr::Call(ast::ExprCall { func, .. }) = value else {
        return Receiver::Unknown;
    };
    let Some(name) = UnqualifiedName::from_expr(func) else {
        return Receiver::Unknown;
    };
    let segments = name.segments();
    let last = segments.last().copied().unwrap_or_default();
    if MOCKS.contains(&last) || segments.starts_with(&["mocker", "patch"]) { Receiver::Mock } else { instance(last) }
}

// The class an annotation names, seeing through `Optional[...]` and `| None`.
fn annotated(annotation: &Expr) -> Receiver {
    match annotation {
        Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::BitOr, right, .. }) => {
            match (left.is_none_literal_expr(), right.is_none_literal_expr()) {
                (true, false) => annotated(right),
                (false, true) => annotated(left),
                _ => Receiver::Unknown,
            }
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) if last_segment(value) == Some("Optional") => {
            annotated(slice)
        }
        _ => match last_segment(annotation) {
            Some(last) if MOCKS.contains(&last) || last == "MockType" => Receiver::Mock,
            Some(last) => instance(last),
            None => Receiver::Unknown,
        },
    }
}

// Only a capitalized name is taken for a class; calling a lowercase factory says nothing about what it returns.
fn instance(name: &str) -> Receiver {
    let is_class = name.starts_with(|first: char| first.is_ascii_uppercase())
        && name.chars().any(|character| character.is_ascii_lowercase());
    if is_class { Receiver::Instance(name.to_owned()) } else { Receiver::Unknown }
}

fn class_mentions(class: &ast::StmtClassDef, name: &str) -> bool {
    let mut mentions = Mentions { name, found: false };
    mentions.visit_body(&class.body);
    mentions.found
}

struct Mentions<'n> {
    name: &'n str,
    found: bool,
}

impl<'a> Visitor<'a> for Mentions<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.found |= match expr {
            Expr::Name(ast::ExprName { id, .. }) => id.as_str() == self.name,
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => attr.as_str() == self.name,
            _ => false,
        };
        visitor::walk_expr(self, expr);
    }
}

// The dotted name of the module a file defines, or its file name outside a package.
// A package's `__init__.py` defines the package itself, which imports name without `__init__`.
pub(crate) fn module_name(module_path: Option<&[String]>, path: &Path) -> String {
    module_path.map_or_else(
        || path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_owned(),
        |module_path| match module_path {
            [package @ .., last] if last == "__init__" && !package.is_empty() => package.join("."),
            _ => module_path.join("."),
        },
    )
}
