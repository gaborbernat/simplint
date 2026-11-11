use std::rc::Rc;

use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::{self as ast, Expr, Operator};
use ruff_text_size::TextRange;

use super::bindings::{Bindings, Value, bindings_in};
use super::declared::{
    Class, annotation, attribute_root, class_at, instance_of, is_property, keeps_return, self_parameter, target_binds,
};
use super::members::{Members, members_of, written_outside};
use super::tables::{
    OS_PATH_FUNCTIONS, PATH_CLASSES, PLATFORM_FUNCTIONS, STRING_CONSTANTS, binary, items, method, shared,
};
use super::{Kind, Kinds};
use crate::lint::walker::{Scope, Walker};
use crate::rules::is_class_like;
use crate::syntax::{Visibility, sole_argument};

impl<'w, 'a> Kinds<'w, 'a> {
    pub(in crate::rules::logic) fn new(walker: &'w Walker<'w, 'a>, range: TextRange) -> Self {
        // A default value or decorator sits in the scope around its def.
        let scopes =
            walker.scopes().iter().take_while(|scope| scope.body_range().contains_range(range)).copied().collect();
        Self { walker, scopes, at_spot: true, resolving: Vec::new(), reading: Vec::new(), relies_on: usize::MAX }
    }

    pub(in crate::rules::logic) fn is_str(&mut self, expr: &'a Expr) -> bool {
        self.kind(expr, self.scopes.len()) == Some(Kind::Str)
    }

    // An operator on such an instance may run the class's own method.
    pub(in crate::rules::logic) fn is_class_instance(&mut self, expr: &'a Expr) -> bool {
        matches!(self.kind(expr, self.scopes.len()), Some(Kind::Instance(_) | Kind::ImportedInstance))
    }

    fn kind(&mut self, expr: &'a Expr, depth: usize) -> Option<Kind> {
        match expr {
            Expr::StringLiteral(_) | Expr::FString(_) => Some(Kind::Str),
            Expr::BytesLiteral(_) => Some(Kind::Bytes),
            Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(_), .. }) => Some(Kind::Int),
            Expr::List(ast::ExprList { elts, .. })
            | Expr::Tuple(ast::ExprTuple { elts, .. })
            | Expr::Set(ast::ExprSet { elts, .. }) => match self.same(elts, depth) {
                None if elts.is_empty() => Some(Kind::StrItems),
                Some(Kind::Str) => Some(Kind::StrItems),
                Some(Kind::Int) => Some(Kind::IntItems),
                _ => None,
            },
            Expr::Dict(dict) => {
                let mut kinds = Vec::new();
                for item in &dict.items {
                    kinds.push(self.kind(item.key.as_ref()?, depth)?);
                    kinds.push(self.kind(&item.value, depth)?);
                }
                match shared(kinds) {
                    None if dict.items.is_empty() => Some(Kind::StrItems),
                    kind => kind.and_then(items),
                }
            }
            // `%` on a `str` formats whatever the right side holds.
            Expr::BinOp(ast::ExprBinOp { left, op: Operator::Mod, .. }) if left.is_string_literal_expr() => {
                Some(Kind::Str)
            }
            Expr::BinOp(ast::ExprBinOp { left, op, right, .. }) => {
                let sides = (self.kind(left, depth)?, self.kind(right, depth)?);
                binary(*op, sides)
            }
            Expr::If(ast::ExprIf { body, orelse, .. }) => self.same([&**body, &**orelse], depth),
            // `or` stops at the first true value, so only the last must be a `str` when the others may be `None`.
            Expr::BoolOp(ast::ExprBoolOp { op: ast::BoolOp::Or, values, .. })
                if values.split_last().is_some_and(|(last, rest)| {
                    rest.iter().all(|value| matches!(self.kind(value, depth), Some(Kind::Str | Kind::OptionalStr)))
                        && self.kind(last, depth) == Some(Kind::Str)
                }) =>
            {
                Some(Kind::Str)
            }
            Expr::BoolOp(ast::ExprBoolOp { values, .. }) => self.same(values, depth),
            Expr::Named(ast::ExprNamed { value, .. }) => self.kind(value, depth),
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
                match (self.kind(value, depth)?, slice.is_slice_expr()) {
                    (Kind::Str, _) | (Kind::StrItems, false) => Some(Kind::Str),
                    (Kind::StrItems, true) => Some(Kind::StrItems),
                    (Kind::IntItems, false) => Some(Kind::Int),
                    (Kind::IntItems, true) => Some(Kind::IntItems),
                    _ => None,
                }
            }
            Expr::Call(call) => self.call(call, depth),
            Expr::Attribute(attribute) => self.attribute(attribute, depth),
            Expr::Name(name) => self.name(&name.id, depth),
            _ => None,
        }
    }

    fn same(&mut self, exprs: impl IntoIterator<Item = &'a Expr>, depth: usize) -> Option<Kind> {
        exprs.into_iter().map(|expr| self.kind(expr, depth)).collect::<Option<Vec<_>>>().and_then(shared)
    }

    fn item(&mut self, expr: &'a Expr, depth: usize) -> Option<Kind> {
        match self.kind(expr, depth)? {
            Kind::Str | Kind::StrItems => Some(Kind::Str),
            Kind::IntItems => Some(Kind::Int),
            _ => None,
        }
    }

    fn call(&mut self, call: &'a ast::ExprCall, depth: usize) -> Option<Kind> {
        let ast::ExprCall { func, arguments, .. } = call;
        match func.as_ref() {
            Expr::Name(name) => match self.lookup(&name.id, depth) {
                None => match name.id.as_str() {
                    "ascii" | "bin" | "chr" | "format" | "hex" | "input" | "oct" | "repr" | "str" => Some(Kind::Str),
                    "int" | "len" | "ord" => Some(Kind::Int),
                    "frozenset" | "list" | "reversed" | "set" | "sorted" | "tuple" => {
                        sole_argument(call).and_then(|iterable| self.item(iterable, depth)).and_then(items)
                    }
                    "range" => Some(Kind::IntItems),
                    _ => None,
                },
                Some((bindings, scope)) if bindings.imported => {
                    let run = (scope == 0).then(|| self.walker.checker.project().run_import_class(&name.id)).flatten();
                    self.imported_callee(call, run, bindings.original, depth)
                }
                Some((bindings, _)) => {
                    if let [class] = bindings.classes.as_slice() {
                        bindings.only_classes().then(|| instance_of(self.walker.checker, class)).flatten()
                    } else {
                        bindings.only_functions().and_then(|functions| self.returns(functions))
                    }
                }
            },
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                let receiver = self.kind(value, depth);
                if receiver == Some(Kind::ImportedInstance) {
                    return Some(Kind::ImportedInstance);
                }
                if let Some(kind) = receiver.and_then(|receiver| method(receiver, attr)) {
                    return Some(kind);
                }
                if receiver == Some(Kind::StrItems) && attr == "get" {
                    return match arguments.args.as_ref() {
                        [_] => Some(Kind::OptionalStr),
                        [_, default] => (self.kind(default, depth) == Some(Kind::Str)).then_some(Kind::Str),
                        _ => None,
                    };
                }
                let checker = self.walker.checker;
                if let Some(Kind::Instance(range)) = receiver {
                    let members = members_of(checker, class_at(checker, range)?, attr);
                    return members.only_methods().and_then(|methods| self.returns(methods));
                }
                if let Some((members, _)) = self.members(value, attr, depth) {
                    return members.only_methods().and_then(|methods| self.returns(methods));
                }
                self.imported_attribute_call(call, value, attr, depth)
            }
            _ => None,
        }
    }

    fn imported_callee(
        &mut self,
        call: &'a ast::ExprCall,
        run: Option<bool>,
        name: Option<&str>,
        depth: usize,
    ) -> Option<Kind> {
        match run {
            Some(true) => Some(Kind::ImportedInstance),
            Some(false) => None,
            None => self
                .library_call(call, depth)
                .or_else(|| name.filter(|name| is_class_like(name)).map(|_| Kind::ImportedInstance)),
        }
    }

    fn imported_attribute_call(
        &mut self,
        call: &'a ast::ExprCall,
        value: &'a Expr,
        attr: &'a str,
        depth: usize,
    ) -> Option<Kind> {
        let run = match value.as_name_expr() {
            Some(module) => match self.lookup(&module.id, depth) {
                Some((bindings, 0)) if bindings.imported => {
                    self.walker.checker.project().run_module_member(&module.id, attr)
                }
                _ => None,
            },
            None => None,
        };
        self.imported_callee(call, run, self.imported_root(value, depth).then_some(attr), depth)
    }

    fn imported_root(&self, expr: &'a Expr, depth: usize) -> bool {
        attribute_root(expr)
            .as_name_expr()
            .and_then(|name| self.lookup(&name.id, depth))
            .is_some_and(|(bindings, _)| bindings.imported)
    }

    fn library_call(&mut self, call: &'a ast::ExprCall, depth: usize) -> Option<Kind> {
        let args = &call.arguments.args;
        match self.qualified(&call.func, depth)?.segments() {
            ["os", "getcwd"]
            | ["getpass", "getuser"]
            | ["json", "dumps"]
            | ["secrets", "token_hex" | "token_urlsafe"]
            | ["shlex", "join" | "quote"]
            | ["socket", "gethostname"]
            | ["textwrap", "dedent" | "fill" | "indent" | "shorten"] => Some(Kind::Str),
            ["platform", function] if PLATFORM_FUNCTIONS.contains(function) => Some(Kind::Str),
            // On `bytes` these return `bytes`, so an argument must be text.
            ["os", "path", function] if OS_PATH_FUNCTIONS.contains(function) => {
                args.iter().any(|arg| self.kind(arg, depth) == Some(Kind::Str)).then_some(Kind::Str)
            }
            ["re", "escape"] => args.iter().any(|arg| self.kind(arg, depth) == Some(Kind::Str)).then_some(Kind::Str),
            ["os", "path", "split" | "splitext"] => match args.as_ref() {
                [path] => (self.kind(path, depth) == Some(Kind::Str)).then_some(Kind::StrItems),
                _ => None,
            },
            ["os", "environ", "get"] | ["os", "getenv"] => match args.as_ref() {
                [_] => Some(Kind::OptionalStr),
                [_, default] => (self.kind(default, depth) == Some(Kind::Str)).then_some(Kind::Str),
                _ => None,
            },
            ["pathlib", class] | ["pathlib", class, "cwd" | "home"] if PATH_CLASSES.contains(class) => Some(Kind::Path),
            _ => None,
        }
    }

    fn attribute(&mut self, attribute: &'a ast::ExprAttribute, depth: usize) -> Option<Kind> {
        let ast::ExprAttribute { value, attr, .. } = attribute;
        if matches!(attr.as_str(), "__name__" | "__qualname__" | "__module__") {
            return Some(Kind::Str);
        }
        if let Expr::Call(ast::ExprCall { func, .. }) = value.as_ref()
            && attr == "hex"
            && self
                .qualified(func, depth)
                .is_some_and(|name| matches!(name.segments(), ["uuid", "uuid1" | "uuid3" | "uuid4" | "uuid5"]))
        {
            return Some(Kind::Str);
        }
        if let Some((members, class)) = self.members(value, attr, depth) {
            return self.member(&members, class, attr);
        }
        let receiver = self.kind(value, depth);
        if receiver == Some(Kind::ImportedInstance) {
            return Some(Kind::ImportedInstance);
        }
        if let Some(Kind::Instance(range)) = receiver {
            let checker = self.walker.checker;
            let members = members_of(checker, class_at(checker, range)?, attr);
            // What its methods set on the instance comes from scopes this spot does not see.
            if members.opaque || !members.methods.is_empty() {
                return None;
            }
            if members.declared.is_some() {
                return members.declared;
            }
            return members.values.is_empty().then(|| self.returns(&members.properties)).flatten();
        }
        if receiver == Some(Kind::Path) {
            return match attr.as_str() {
                "name" | "stem" | "suffix" => Some(Kind::Str),
                "parent" => Some(Kind::Path),
                _ => None,
            };
        }
        match (self.qualified(value, depth)?.segments(), attr.as_str()) {
            (["os"], "curdir" | "extsep" | "linesep" | "name" | "pardir" | "pathsep" | "sep")
            | (["sys"], "executable" | "platform" | "version") => Some(Kind::Str),
            (["string"], constant) if STRING_CONSTANTS.contains(&constant) => Some(Kind::Str),
            (["os"], "environ") => Some(Kind::StrItems),
            _ => None,
        }
    }

    fn qualified(&self, expr: &'a Expr, depth: usize) -> Option<QualifiedName<'w>> {
        let (bindings, scope) = self.lookup(&attribute_root(expr).as_name_expr()?.id, depth)?;
        (scope == 0 && bindings.only_imported()).then(|| self.walker.checker.semantic().qualified_name(expr)).flatten()
    }

    fn name(&mut self, name: &'a str, depth: usize) -> Option<Kind> {
        // A comprehension binds its targets in a scope of its own.
        if self.at_spot
            && self.resolving.is_empty()
            && let Some(generator) = self.comprehension(name)
        {
            // An unpacked target takes an item of an item, which no kind here tells.
            return generator.target.is_name_expr().then(|| self.item(&generator.iter, depth)).flatten();
        }
        let Some((bindings, depth)) = self.lookup(name, depth) else {
            return matches!(name, "__file__" | "__name__").then_some(Kind::Str);
        };
        if bindings.opaque || bindings.imported || !bindings.functions.is_empty() {
            return None;
        }
        let key = (name, depth.checked_sub(1).map_or_else(TextRange::default, |index| self.scopes[index].body_range()));
        if let Some(index) = self.resolving.iter().position(|(other, at, _)| (*other, *at) == key) {
            self.relies_on = self.relies_on.min(index + 1);
            return self.resolving[index].2;
        }
        let checker = self.walker.checker;
        if let Some(kind) = checker.state().kinds().resolved.borrow().get(&key) {
            return *kind;
        }
        let outer = std::mem::replace(&mut self.relies_on, usize::MAX);
        let kind = self.bound(&bindings, key, depth);
        if self.relies_on > self.resolving.len() {
            checker.state().kinds().resolved.borrow_mut().insert(key, kind);
        }
        self.relies_on = self.relies_on.min(outer);
        kind
    }

    fn bound(&mut self, bindings: &Bindings<'a>, key: (&'a str, TextRange), depth: usize) -> Option<Kind> {
        // An annotation holds unless a value the code shows contradicts it, as a loop over numbers would.
        if let Some(declared) = bindings.declared {
            let kinds = self.values(&bindings.values, key, Some(declared), depth);
            return kinds.iter().flatten().all(|kind| *kind == declared).then_some(declared);
        }
        // A binding that reads the name it binds has the kind the others give it, so first find that kind with such
        // reads unknown, then check every binding against it.
        let first = self.values(&bindings.values, key, None, depth);
        let guess = shared(first.iter().flatten().copied());
        if first.iter().all(Option::is_some) {
            return guess;
        }
        guess.filter(|guess| {
            self.values(&bindings.values, key, Some(*guess), depth).iter().all(|kind| kind == &Some(*guess))
        })
    }

    fn values(
        &mut self,
        values: &[Value<'a>],
        key: (&'a str, TextRange),
        assumed: Option<Kind>,
        depth: usize,
    ) -> Vec<Option<Kind>> {
        self.resolving.push((key.0, key.1, assumed));
        let kinds = values
            .iter()
            .map(|value| match *value {
                Value::Expr(expr) => self.kind(expr, depth),
                Value::Item(expr) => self.item(expr, depth),
                Value::Entered(expr) => self.entered(expr, depth),
            })
            .collect();
        self.resolving.pop();
        kinds
    }

    // What `with` gives the target of a context manager: the name of a temporary directory is a `str`.
    fn entered(&self, expr: &'a Expr, depth: usize) -> Option<Kind> {
        let Expr::Call(ast::ExprCall { func, .. }) = expr else {
            return None;
        };
        self.qualified(func, depth)
            .is_some_and(|name| name.segments() == ["tempfile", "TemporaryDirectory"])
            .then_some(Kind::Str)
    }

    fn comprehension(&self, name: &str) -> Option<&'a ast::Comprehension> {
        self.walker.expressions().iter().rev().find_map(|expr| {
            let generators = match expr {
                Expr::ListComp(ast::ExprListComp { generators, .. })
                | Expr::SetComp(ast::ExprSetComp { generators, .. })
                | Expr::Generator(ast::ExprGenerator { generators, .. }) => generators.as_slice(),
                Expr::DictComp(ast::ExprDictComp { generators, .. }) => generators,
                _ => return None,
            };
            generators.iter().rev().find(|generator| target_binds(&generator.target, name))
        })
    }

    // A class body is skipped unless innermost, since its methods do not see it, but a type parameter of the class is
    // seen from them.
    fn lookup(&self, name: &'a str, depth: usize) -> Option<(Rc<Bindings<'a>>, usize)> {
        let checker = self.walker.checker;
        self.scopes[..depth]
            .iter()
            .enumerate()
            .rev()
            .filter(|(index, scope)| match scope {
                Scope::Class(class) => index + 1 == depth || type_params_bind(class, name),
                Scope::Function(_) | Scope::Lambda(_) => true,
            })
            .map(|(index, scope)| (bindings_in(checker, Some(*scope), name), index + 1))
            .chain(std::iter::once_with(|| (bindings_in(checker, None, name), 0)))
            .find(|(bindings, _)| bindings.binds())
    }

    fn returns(&self, functions: &[&'a ast::StmtFunctionDef]) -> Option<Kind> {
        let checker = self.walker.checker;
        let mut kinds = Vec::new();
        for function in functions {
            if function.is_async
                || !function
                    .decorator_list
                    .iter()
                    .all(|decorator| keeps_return(checker, decorator) || is_property(checker, decorator))
            {
                return None;
            }
            kinds.push(annotation(checker, function.returns.as_deref()?)?);
        }
        shared(kinds)
    }

    fn members(&self, receiver: &'a Expr, attr: &'a str, depth: usize) -> Option<(Rc<Members<'a>>, Class<'a>)> {
        let receiver = receiver.as_name_expr()?;
        let index = self.scopes[..depth].iter().rposition(|scope| matches!(scope, Scope::Function(_)))?;
        let (Scope::Function(method), Some(Scope::Class(class))) =
            (self.scopes[index], index.checked_sub(1).map(|at| self.scopes[at]))
        else {
            return None;
        };
        // The parameter must be the only binding of the name, so it still holds the instance.
        if self_parameter(method) != Some(receiver.id.as_str())
            || self.lookup(&receiver.id, depth).is_none_or(|(bindings, at)| {
                at != index + 1
                    || !bindings.values.is_empty()
                    || !bindings.functions.is_empty()
                    || !bindings.classes.is_empty()
                    || bindings.imported
            })
        {
            return None;
        }
        Some((members_of(self.walker.checker, class, attr), Class { def: class, depth: index - 1 }))
    }

    fn member(&mut self, members: &Members<'a>, class: Class<'a>, attr: &'a str) -> Option<Kind> {
        if members.opaque || !members.methods.is_empty() {
            return None;
        }
        if members.declared.is_some() {
            return members.declared;
        }
        if !members.properties.is_empty() {
            return members.values.is_empty().then(|| self.returns(&members.properties)).flatten();
        }
        let Class { def: class, depth: class_depth } = class;
        let project = self.walker.checker.project();
        let key = (class.range, attr);
        // A subclass or another file may give it any value.
        if members.values.is_empty()
            || project.is_base_class(&class.name)
            || project.is_attribute_written(attr)
            || Visibility::of(attr) == Visibility::Public && project.is_referenced(&class.name)
            || written_outside(self.walker.checker, class, attr)
        {
            return None;
        }
        if let Some(&(_, _, started)) = self.reading.iter().find(|(range, name, _)| (*range, *name) == key) {
            self.relies_on = self.relies_on.min(started);
            return None;
        }
        self.reading.push((key.0, key.1, self.resolving.len()));
        let outer_at_spot = std::mem::replace(&mut self.at_spot, false);
        let kinds = members
            .values
            .iter()
            .map(|(value, method)| {
                let mut scopes = self.scopes[..=class_depth].to_vec();
                scopes.extend(method.map(Scope::Function));
                let depth = scopes.len();
                let outer = std::mem::replace(&mut self.scopes, scopes);
                let kind = self.kind(value, depth);
                self.scopes = outer;
                kind
            })
            .collect::<Vec<_>>();
        self.at_spot = outer_at_spot;
        self.reading.pop();
        kinds.into_iter().collect::<Option<Vec<_>>>().and_then(shared)
    }
}

// A type parameter binds its name in a scope the code here does not follow, so the name is unknown there.
fn type_params_bind(class: &ast::StmtClassDef, name: &str) -> bool {
    class
        .type_params
        .as_deref()
        .is_some_and(|type_params| type_params.type_params.iter().any(|param| param.name() == name))
}
