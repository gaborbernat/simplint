use rustc_hash::FxHashMap;
use std::path::Path;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};

use super::facts::qualified;
use crate::syntax::MOCK_CLASSES;
use crate::syntax::{alias_binding, instance_writes, is_classmethod, is_name, last_segment, receiver};

// The attributes code sets or deletes on objects, keyed by the class of the object when parsing tells it: the receiver
// is `self` in a method, or a name bound by a call to a class or annotated with one. A class goes by its qualified
// name, such as `pkg.models.Result`, as two modules can each define a class with the same name. A mock accepts any
// attribute and is never a real instance, so writes to one are left out; writes to anything else are untyped. Reaching
// the instance `__dict__`, as `obj.__dict__` or `vars(obj)` do, counts as writing `__dict__`. A write whose name is
// computed at run time is recorded as `*`, and a weak reference to an instance as `__weakref__`.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AttributeWrites {
    pub(super) typed: Vec<(String, String)>,
    pub(super) untyped: Vec<String>,
    // The qualified classes whose instances get a `__dict__` store, `x.__dict__ = ...` or `del x.__dict__`, or a
    // `setattr` or `delattr` of it, which a frozen instance rejects. Reading `__dict__` is in `typed` and `untyped`
    // alone.
    pub(super) dict_stores: Vec<String>,
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
    pub(super) fn collect(body: &[Stmt], aliases: &FxHashMap<&str, String>, module: &str) -> Self {
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

struct Scope<'a> {
    kind: ScopeKind<'a>,
    bindings: FxHashMap<&'a str, Receiver>,
}

// A class scope keeps what its methods store on `self`; names a class body binds are not visible to its methods.
enum ScopeKind<'a> {
    Plain,
    Class { name: &'a str, members: FxHashMap<&'a str, Receiver> },
}

// The attribute a write with a computed name is recorded under: the name is decided at run time, so the write may hit
// any field or none, and no field is named `*`.
const ANY_ATTRIBUTE: &str = "*";

struct Collector<'a, 'm> {
    aliases: &'m FxHashMap<&'a str, String>,
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
                if attr.as_str() == "__dict__" {
                    self.record_dict_store(value);
                }
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "__dict__" => {
                self.record(value, "__dict__");
            }
            Expr::Call(call @ ast::ExprCall { func, arguments, .. }) => {
                let builtin = func.as_name_expr().map(|name| name.id.as_str());
                match (&*arguments.args, builtin) {
                    ([target, attribute, ..], Some("setattr" | "delattr")) => self.record_write(target, attribute),
                    ([target], Some("vars")) => self.record(target, "__dict__"),
                    ([target, ..], _) if self.weakly_references(func) => self.record(target, "__weakref__"),
                    ([target, attribute, ..], _) if is_patch_helper(func) => self.record_write(target, attribute),
                    _ => self.record_dunder_write(func, arguments),
                }
                self.writes.pickles_low_protocol |= self.pickles_low_protocol(call);
            }
            // A lambda's parameters and a comprehension's targets shadow the names around them.
            Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                let mut scan = Bindings { collector: self, bound: FxHashMap::default() };
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

    fn record_dict_store(&mut self, receiver: &Expr) {
        let class = match self.receiver(receiver) {
            Receiver::Instance(class) => self.qualified(&class),
            Receiver::Own(class) => class,
            Receiver::Mock | Receiver::Unknown => return,
        };
        self.writes.dict_stores.push(class);
    }

    // The write of the attribute `attribute` names: a literal name is recorded as written, and a computed one may be any
    // attribute of a typed receiver.
    fn record_write(&mut self, receiver: &Expr, attribute: &Expr) {
        let Expr::StringLiteral(name) = attribute else {
            return self.record_computed(receiver);
        };
        let name = name.value.to_str();
        self.record(receiver, name);
        if name == "__dict__" {
            self.record_dict_store(receiver);
        }
    }

    // Computed names are left out of an untyped receiver: the untyped list matches by field name, which a computed
    // name does not give.
    fn record_computed(&mut self, receiver: &Expr) {
        if matches!(self.receiver(receiver), Receiver::Instance(_) | Receiver::Own(_)) {
            self.record(receiver, ANY_ATTRIBUTE);
        }
    }

    // `obj.__setattr__(name, value)` and `obj.__delattr__(name)` write `name` on `obj`. `object.__setattr__` and
    // `super().__setattr__` stay out: they are how a frozen dataclass writes its own fields.
    fn record_dunder_write(&mut self, func: &Expr, arguments: &ast::Arguments) {
        let Some(ast::ExprAttribute { value, attr, .. }) = func.as_attribute_expr() else {
            return;
        };
        let object_write =
            is_name(value, "object") || value.as_call_expr().is_some_and(|call| is_name(&call.func, "super"));
        if let [attribute, ..] = &*arguments.args
            && matches!(attr.as_str(), "__setattr__" | "__delattr__")
            && !object_write
        {
            self.record_write(value, attribute);
        }
    }

    // `weakref.ref(obj)`, `weakref.proxy(obj)` and `weakref.finalize(obj, ...)`, however the imports name them.
    fn weakly_references(&self, func: &Expr) -> bool {
        UnqualifiedName::from_expr(func).is_some_and(|written| {
            matches!(
                self.qualified(&written.segments().join(".")).as_str(),
                "weakref.ref" | "weakref.proxy" | "weakref.finalize"
            )
        })
    }

    // `pickle.dumps(obj, protocol)`, `pickle.dump(obj, file, protocol)` or `pickle.Pickler(file, protocol)` with a
    // protocol that is not known to be 2 or above, such as a test looping over every protocol.
    fn pickles_low_protocol(&self, call: &ast::ExprCall) -> bool {
        let Some(written) = UnqualifiedName::from_expr(&call.func) else {
            return false;
        };
        // Qualifying keeps the last segment, except for a bare name bound by an import, as `from pickle import dumps as
        // write` makes `write` mean `pickle.dumps`. Other calls cannot be a pickle function, so they skip qualifying.
        let last = match written.segments() {
            [name] => self.aliases.get(name).map_or(*name, |module| module.rsplit('.').next().unwrap_or_default()),
            segments => segments.last().copied().unwrap_or_default(),
        };
        if !matches!(last, "dumps" | "dump" | "Pickler") {
            return false;
        }
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
        let instance = receiver(function).filter(|_| !is_classmethod(function));
        function
            .parameters
            .iter()
            .map(|parameter| {
                let name = parameter.name().as_str();
                let receiver = match class {
                    Some(class) if instance == Some(name) => Receiver::Own(format!("{}.{class}", self.module)),
                    _ => parameter.annotation().map_or(Receiver::Unknown, annotated),
                };
                (name, receiver)
            })
            .collect()
    }

    fn bindings(&self, body: &'a [Stmt], parameters: &[(&'a str, Receiver)]) -> FxHashMap<&'a str, Receiver> {
        let mut scan = Bindings { collector: self, bound: FxHashMap::default() };
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
    bound: FxHashMap<&'a str, Receiver>,
}

impl<'a> Bindings<'a, '_, '_> {
    fn bind(&mut self, name: &'a str, receiver: Receiver) {
        bind(&mut self.bound, name, receiver);
    }
}

// A name bound to two different receivers holds something unknown.
fn bind<'a>(bound: &mut FxHashMap<&'a str, Receiver>, name: &'a str, receiver: Receiver) {
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
                    let bound = alias_binding(alias);
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

// What the methods of a class assign to their instances, each a single target. `object.__setattr__` writes stay out, as
// the index has never read them.
fn members(class: &ast::StmtClassDef) -> FxHashMap<&str, Receiver> {
    let mut members = FxHashMap::default();
    for write in
        instance_writes(class).into_iter().filter(|write| !is_classmethod(write.method) && !write.through_object)
    {
        let receiver = match write.stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. })
                if let [target] = targets.as_slice()
                    && target.is_attribute_expr() =>
            {
                assigned(value)
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) if target.is_attribute_expr() => {
                annotated(annotation)
            }
            _ => Receiver::Unknown,
        };
        bind(&mut members, write.attr, receiver);
    }
    for stmt in &class.body {
        if let Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) = stmt
            && let Expr::Name(name) = target.as_ref()
            && members.contains_key(name.id.as_str())
            && let declared @ Receiver::Instance(_) = annotated(annotation)
        {
            members.insert(name.id.as_str(), declared);
        }
    }
    members
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
    if MOCK_CLASSES.contains(&last) || segments.starts_with(&["mocker", "patch"]) {
        Receiver::Mock
    } else {
        instance(last)
    }
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
            Some(last) if MOCK_CLASSES.contains(&last) || last == "MockType" => Receiver::Mock,
            Some(last) => instance(last),
            None => Receiver::Unknown,
        },
    }
}

// Only a capitalized name is taken for a class; calling a lowercase factory says nothing about what it returns. A
// private class is taken without its leading underscores, and an all-caps name such as `URL` is a class too, while a
// one- or two-letter capital is more likely a type variable such as `T` or `KT`.
fn instance(name: &str) -> Receiver {
    let bare = name.trim_start_matches('_');
    let is_class = bare.starts_with(|first: char| first.is_ascii_uppercase())
        && (bare.chars().any(|character| character.is_ascii_lowercase()) || bare.len() >= 3);
    if is_class { Receiver::Instance(name.to_owned()) } else { Receiver::Unknown }
}

// A patch helper writes an attribute of its first argument: `monkeypatch.setattr(obj, "name", value)`, or
// `mock.patch.object(obj, "name")` however `patch` is imported.
fn is_patch_helper(func: &Expr) -> bool {
    let setter =
        func.as_attribute_expr().is_some_and(|attribute| matches!(attribute.attr.as_str(), "setattr" | "delattr"));
    setter || UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().ends_with(&["patch", "object"]))
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

// The parts of the module a file defines. A package's `__init__.py` defines the package itself, which imports name
// without `__init__`.
pub(crate) fn module_parts<S: AsRef<str>>(module_path: &[S]) -> &[S] {
    match module_path {
        [package @ .., last] if last.as_ref() == "__init__" => package,
        _ => module_path,
    }
}

// The dotted name of the module a file defines, or its file name outside a package.
pub(crate) fn module_name(module_path: Option<&[String]>, path: &Path) -> String {
    module_path.map_or_else(
        || path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().to_owned(),
        |module_path| module_parts(module_path).join("."),
    )
}
