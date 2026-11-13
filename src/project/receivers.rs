use rustc_hash::FxHashMap;
use std::path::Path;

use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};

use super::facts::qualified;
use crate::syntax::{MOCK_CLASSES, last_dotted_segment};
use crate::syntax::{
    alias_root_binding, instance_writes, is_classmethod, is_name, last_segment, module_statements, pattern_capture,
    receiver,
};

// The class of an access the file cannot tie to one class, such as `type(obj).timeout` on an `obj` of unknown class.
pub(crate) const UNRESOLVED_CLASS: &str = "*";

// The attribute a write with a computed name is recorded under: the name is decided at run time, so the write may hit
// any field or none, and no field is named `*`.
pub(crate) const ANY_ATTRIBUTE: &str = "*";

// The attributes code sets or deletes on objects, keyed by the class of the object when parsing tells it: the receiver
// is `self` in a method, or a name bound by a call to a class or annotated with one. A class goes by its qualified
// name, such as `pkg.models.Result`, as two modules can each define a class with the same name.
//
// A mock accepts any attribute and is never a real instance, so writes to one are left out; writes to anything else are
// untyped. Reaching the instance `__dict__`, as `obj.__dict__` or `vars(obj)` do, counts as writing `__dict__`. A write
// whose name is computed at run time is recorded as `*`, and a weak reference to an instance as `__weakref__`.
#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct AttributeWrites {
    pub(super) typed: Vec<(String, String)>,
    pub(super) untyped: Vec<String>,
    // Classes whose instances get a `__dict__` store, such as `x.__dict__ = ...`, `del x.__dict__`, or a `setattr` or
    // `delattr` of it, which a frozen instance rejects. Reading `__dict__` is in `typed` and `untyped` alone.
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
    // The writes through a name an annotation gives a class, as (qualified class, attribute). Whether the run defines
    // that class is known only across the run, so the index decides which of these are typed.
    pub(super) declared: Vec<(String, String)>,
    pub(super) declared_dict_stores: Vec<String>,
    pub(super) classes: Vec<String>,
    // The attributes the file reads, sets or deletes through a class name, as (qualified class, attribute), which
    // replace a class attribute a slotted class turns into a descriptor. A class the file cannot resolve is
    // `UNRESOLVED_CLASS`, which counts against every class.
    pub(super) class_attributes: Vec<(String, String)>,
    // The classes the file defines with each base they name, as (qualified class, qualified base), so an access through
    // a subclass reaches the base's attributes.
    pub(super) class_bases: Vec<(String, String)>,
}

impl AttributeWrites {
    pub(super) fn collect(body: &[Stmt], aliases: &FxHashMap<&str, String>, module: &str) -> Self {
        let mut collector = Collector { aliases, module, scopes: Vec::new(), writes: Self::default() };
        collector.scopes.push(Scope { kind: ScopeKind::Plain, bindings: collector.bindings(body, &[]) });
        collector.visit_body(body);
        collector.writes.classes = module_classes(body, module);
        collector.writes
    }
}

// The classes a module defines at its top level, qualified by the module, which an annotation may name. A protocol is
// not one: its instances are any object with the attributes it lists.
fn module_classes(body: &[Stmt], module: &str) -> Vec<String> {
    module_statements(body)
        .filter_map(Stmt::as_class_def_stmt)
        .filter(|class| !class.bases().iter().any(|base| last_segment(map_subscript(base)) == Some("Protocol")))
        .map(|class| format!("{module}.{}", class.name))
        .collect()
}

struct Collector<'a, 'm> {
    aliases: &'m FxHashMap<&'a str, String>,
    module: &'m str,
    scopes: Vec<Scope<'a>>,
    writes: AttributeWrites,
}

struct Scope<'a> {
    kind: ScopeKind<'a>,
    bindings: FxHashMap<&'a str, Receiver>,
}

enum ScopeKind<'a> {
    Plain,
    Class { name: &'a str, members: FxHashMap<&'a str, Receiver> },
}

#[derive(Clone, PartialEq, Eq)]
enum Receiver {
    Instance(String),
    // An instance of the class an annotation names, typed only when the run defines that class.
    Declared(String),
    // `self`, an instance of the class around the method, by its qualified name, which no import can shadow.
    Own(String),
    Mock,
    Unknown,
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
                let bindings = self.bindings(&function.body, &self.parameters(function));
                self.scopes.push(Scope { kind: ScopeKind::Plain, bindings });
                self.visit_body(&function.body);
                self.scopes.pop();
            }
            Stmt::ClassDef(class) => {
                let bases: Vec<String> = class
                    .bases()
                    .iter()
                    .filter_map(|base| Some(self.qualified_segments(UnqualifiedName::from_expr(base)?.segments())))
                    .collect();
                let qualified_class = format!("{}.{}", self.module, class.name);
                self.writes.class_bases.extend(bases.iter().map(|base| (qualified_class.clone(), base.clone())));
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
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                self.weakly_stored_by_assign(targets, value);
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(access) = self.class_access(expr) {
            self.writes.class_attributes.push(access);
        }
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
                self.weakly_stored_by_call(call);
                match (&*arguments.args, func.as_name_expr().map(|name| name.id.as_str())) {
                    ([target, attribute, ..], Some("setattr" | "delattr")) => self.record_write(target, attribute),
                    ([target], Some("vars")) => self.record(target, "__dict__"),
                    ([target, ..], _) if self.weakly_references(func) => self.record(target, "__weakref__"),
                    // `monkeypatch.setattr("pkg.Class.name", value)` names the object and the attribute in one path.
                    ([Expr::StringLiteral(path), ..], _) if is_patch_helper(func) => {
                        if let Some((_, name)) = path.value.to_str().rsplit_once('.') {
                            self.writes.untyped.push(name.to_owned());
                        }
                    }
                    ([target, attribute, ..], _) if is_patch_helper(func) => self.record_write(target, attribute),
                    _ => self.record_dunder_write(func, arguments),
                }
                self.writes.pickles_low_protocol |= self.pickles_low_protocol(call);
            }
            Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                let mut scan = Bindings { collector: self, bound: FxHashMap::default() };
                visitor::walk_expr(&mut scan, expr);
                let parameters = expr.as_lambda_expr().and_then(|lambda| lambda.parameters.as_deref());
                for parameter in parameters.into_iter().flat_map(|parameters| parameters.iter()) {
                    scan.bind(parameter.name().as_str(), Receiver::Unknown);
                }
                self.scopes.push(Scope { kind: ScopeKind::Plain, bindings: scan.bound });
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

    fn qualified_segments(&self, segments: &[&str]) -> String {
        qualified(segments, self.aliases).unwrap_or_else(|| format!("{}.{}", self.module, segments.join(".")))
    }

    // A `WeakKeyDictionary` holds the keys stored into it and a `WeakValueDictionary` the values.
    fn weakly_stored_by_assign(&mut self, targets: &'a [Expr], value: &'a Expr) {
        for target in targets {
            let Expr::Subscript(ast::ExprSubscript { value: container, slice, .. }) = target else {
                continue;
            };
            if self.is_instance_of(container, "weakref.WeakKeyDictionary") {
                self.record_on(self.stored(slice), "__weakref__");
            }
            if self.is_instance_of(container, "weakref.WeakValueDictionary") {
                self.record_on(self.stored(value), "__weakref__");
            }
        }
    }

    fn is_instance_of(&self, expr: &Expr, class: &str) -> bool {
        matches!(self.receiver(expr), Receiver::Instance(written) if self.qualified(&written) == class)
    }

    fn receiver(&self, expr: &Expr) -> Receiver {
        match expr {
            Expr::Name(name) => self.lookup(name.id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => match self.receiver(value) {
                Receiver::Mock => Receiver::Mock,
                Receiver::Instance(class) | Receiver::Declared(class) | Receiver::Own(class) => {
                    self.member(last_dotted_segment(&class), attr.as_str())
                }
                Receiver::Unknown => Receiver::Unknown,
            },
            // Anything reached from a mock, such as `client.return_value.session`, is a mock too.
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

    fn stored(&self, item: &Expr) -> Receiver {
        if item.is_call_expr() { assigned(item) } else { self.receiver(item) }
    }

    fn record_on(&mut self, receiver: Receiver, attribute: &str) {
        match receiver {
            Receiver::Instance(class) => self.writes.typed.push((self.qualified(&class), attribute.to_owned())),
            Receiver::Declared(class) => self.writes.declared.push((self.qualified(&class), attribute.to_owned())),
            Receiver::Own(class) => self.writes.typed.push((class, attribute.to_owned())),
            Receiver::Mock => {}
            Receiver::Unknown => self.writes.untyped.push(attribute.to_owned()),
        }
    }

    fn qualified(&self, written: &str) -> String {
        self.qualified_segments(&written.split('.').collect::<Vec<_>>())
    }

    // `type(obj)` and `obj.__class__` name the class of `obj`; an unresolved class counts against every class.
    fn class_access(&self, expr: &Expr) -> Option<(String, String)> {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                let class = match value.as_ref() {
                    Expr::Call(ast::ExprCall { func, arguments, .. })
                        if is_name(func, "type") && arguments.args.len() == 1 =>
                    {
                        self.class_of(&arguments.args[0]).unwrap_or_else(|| UNRESOLVED_CLASS.to_owned())
                    }
                    Expr::Attribute(ast::ExprAttribute { value: object, attr: dunder, .. })
                        if dunder.as_str() == "__class__" =>
                    {
                        self.class_of(object).unwrap_or_else(|| UNRESOLVED_CLASS.to_owned())
                    }
                    other => self.class_of(other)?,
                };
                Some((class, attr.to_string()))
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                let [target, name, ..] = &*arguments.args else {
                    return None;
                };
                let name = name.as_string_literal_expr()?.value.to_str();
                matches!(func.as_name_expr()?.id.as_str(), "getattr" | "setattr" | "delattr")
                    .then(|| (self.class_of(target).unwrap_or_else(|| UNRESOLVED_CLASS.to_owned()), name.to_owned()))
            }
            _ => None,
        }
    }

    fn class_of(&self, expr: &Expr) -> Option<String> {
        class_name(&self.qualified_segments(UnqualifiedName::from_expr(expr)?.segments()))
    }

    fn record(&mut self, receiver: &Expr, attribute: &str) {
        self.record_on(self.receiver(receiver), attribute);
    }

    fn record_dict_store(&mut self, receiver: &Expr) {
        let class = match self.receiver(receiver) {
            Receiver::Instance(class) => self.qualified(&class),
            Receiver::Own(class) => class,
            Receiver::Declared(class) => {
                self.writes.declared_dict_stores.push(self.qualified(&class));
                return;
            }
            Receiver::Mock | Receiver::Unknown => return,
        };
        self.writes.dict_stores.push(class);
    }

    // A weak container holds the items put into it through `weakref.ref`, so the items need a `__weakref__` slot: the
    // item a `WeakSet` gets through `add`, a `WeakMethod`'s object and a `WeakKeyDictionary` key.
    fn weakly_stored_by_call(&mut self, call: &ast::ExprCall) {
        let item = match (&*call.arguments.args, call.func.as_ref()) {
            ([item, ..], Expr::Attribute(ast::ExprAttribute { value, attr, .. }))
                if attr.as_str() == "add" && self.is_instance_of(value, "weakref.WeakSet") =>
            {
                Some(item)
            }
            ([Expr::Attribute(ast::ExprAttribute { value, .. }), ..], func)
                if UnqualifiedName::from_expr(func)
                    .is_some_and(|written| self.qualified_segments(written.segments()) == "weakref.WeakMethod") =>
            {
                Some(value.as_ref())
            }
            _ => None,
        };
        if let Some(item) = item {
            self.record_on(self.stored(item), "__weakref__");
        }
    }

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
        if matches!(self.receiver(receiver), Receiver::Instance(_) | Receiver::Declared(_) | Receiver::Own(_)) {
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

    fn weakly_references(&self, func: &Expr) -> bool {
        UnqualifiedName::from_expr(func).is_some_and(|written| {
            matches!(
                self.qualified_segments(written.segments()).as_str(),
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
            [name] => self.aliases.get(name).map_or(*name, |module| last_dotted_segment(module)),
            segments => segments.last().copied().unwrap_or_default(),
        };
        if !matches!(last, "dumps" | "dump" | "Pickler") {
            return false;
        }
        let position = match self.qualified_segments(written.segments()).as_str() {
            "pickle.dumps" | "pickle.Pickler" => 1,
            "pickle.dump" => 2,
            _ => return false,
        };
        call.arguments.find_argument_value("protocol", position).is_some_and(|protocol| match protocol {
            Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(value), .. }) => {
                value.as_u8().is_none_or(|value| value < 2)
            }
            other => !last_segment(other).is_some_and(|name| matches!(name, "HIGHEST_PROTOCOL" | "DEFAULT_PROTOCOL")),
        })
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

// The names a scope binds and what each holds; a name bound more than one way, or in a way parsing cannot follow, holds
// something unknown. Nested functions and classes are scopes of their own.
struct Bindings<'a, 'c, 'm> {
    collector: &'c Collector<'a, 'm>,
    bound: FxHashMap<&'a str, Receiver>,
}

impl<'a> Bindings<'a, '_, '_> {
    fn bind(&mut self, name: &'a str, receiver: Receiver) {
        bind(&mut self.bound, name, receiver);
    }
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
                    self.bind(alias_root_binding(alias), Receiver::Unknown);
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
        if let Some(name) = pattern_capture(pattern) {
            self.bind(name.as_str(), Receiver::Unknown);
        }
        visitor::walk_pattern(self, pattern);
    }
}

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
            && let declared @ Receiver::Declared(_) = annotated(annotation)
        {
            members.insert(name.id.as_str(), declared);
        }
    }
    members
}

fn assigned(value: &Expr) -> Receiver {
    let Expr::Call(ast::ExprCall { func, .. }) = value else {
        return Receiver::Unknown;
    };
    let Some(name) = UnqualifiedName::from_expr(func) else {
        return Receiver::Unknown;
    };
    let segments = name.segments();
    if segments.last().is_some_and(|last| MOCK_CLASSES.contains(last)) || segments.starts_with(&["mocker", "patch"]) {
        Receiver::Mock
    } else {
        class_name(&segments.join(".")).map_or(Receiver::Unknown, Receiver::Instance)
    }
}

// Only a capitalized name is taken for a class; calling a lowercase factory says nothing about what it returns. A
// private class is taken without its leading underscores, and an all-caps name such as `URL` is a class too, while a
// one- or two-letter capital is more likely a type variable such as `T` or `KT`. The name keeps its module, as
// `models.Release`, so the import that binds the module qualifies it.
fn class_name(written: &str) -> Option<String> {
    let bare = last_dotted_segment(written).trim_start_matches('_');
    let is_class = bare.starts_with(|first: char| first.is_ascii_uppercase())
        && (bare.chars().any(|character| character.is_ascii_lowercase()) || bare.len() >= 3);
    is_class.then(|| written.to_owned())
}

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
        _ => match UnqualifiedName::from_expr(annotation) {
            Some(name) => {
                let last = name.segments().last().copied().unwrap_or_default();
                if MOCK_CLASSES.contains(&last) || last == "MockType" {
                    Receiver::Mock
                } else {
                    class_name(&name.segments().join(".")).map_or(Receiver::Unknown, Receiver::Declared)
                }
            }
            None => Receiver::Unknown,
        },
    }
}

// A patch helper writes an attribute of its first argument: `monkeypatch.setattr(obj, "name", value)`, or
// `mock.patch.object(obj, "name")` however `patch` is imported.
fn is_patch_helper(func: &Expr) -> bool {
    func.as_attribute_expr().is_some_and(|attribute| matches!(attribute.attr.as_str(), "setattr" | "delattr"))
        || UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().ends_with(&["patch", "object"]))
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
