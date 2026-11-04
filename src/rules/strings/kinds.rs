use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Operator, Pattern, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::walker::{Scope, Walker};
use crate::rules::Visibility;
use crate::rules::for_each_child_body;
use crate::semantic::BindingKind;

// The few types the code can show for sure, enough to tell what `+` and `*` on them return.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Str,
    Int,
    Bytes,
    Path,
    // A `str` or `None`, as `str | None` declares.
    OptionalStr,
    // A list, tuple, set or `str`-keyed mapping whose items, indexed or iterated, are each a `str`.
    StrItems,
    // The same, with each item an `int`.
    IntItems,
    // An instance of the module's class defined at this range.
    Instance(TextRange),
}

// What the code shows about the type of an expression, read off the scopes around one spot in the module.
pub struct Kinds<'w, 'a> {
    walker: &'w Walker<'w, 'a>,
    // The scopes the expression being read sits in, outermost first, the module left out: those around the spot, or
    // those of the method that sets an attribute the spot reads.
    scopes: Vec<Scope<'a>>,
    // Whether `scopes` are those around the spot, where the walker knows the comprehensions too.
    at_spot: bool,
    // The names whose bindings are being checked, with the scope they resolve in and the kind assumed for a binding
    // that reads the name it binds, as `text = text + "!"` does.
    resolving: Vec<(&'a str, TextRange, Option<Kind>)>,
    // The kinds of names whose bindings read no assumption made before their own, which hold wherever the name is
    // read; caching only those keeps a chain of names that each read themselves linear.
    resolved: FxHashMap<(&'a str, TextRange), Option<Kind>>,
    // The attributes whose values are being checked, by class and name, with the length of `resolving` then: one that
    // reads itself stays unknown.
    reading: Vec<(TextRange, &'a str, usize)>,
    // The shortest length of `resolving` the kind being found relies on: a self-read assumed at index `i` relies on
    // `i + 1` entries, an attribute read while it was being checked on as many as `resolving` held when that started.
    relies_on: usize,
}

impl<'w, 'a> Kinds<'w, 'a> {
    pub fn new(walker: &'w Walker<'w, 'a>, range: TextRange) -> Self {
        // A default value or decorator sits in the scope around its def.
        let scopes =
            walker.scopes().iter().take_while(|scope| body_range(**scope).contains_range(range)).copied().collect();
        Self {
            walker,
            scopes,
            at_spot: true,
            resolving: Vec::new(),
            resolved: FxHashMap::default(),
            reading: Vec::new(),
            relies_on: usize::MAX,
        }
    }

    pub fn is_str(&mut self, expr: &'a Expr) -> bool {
        let depth = self.scopes.len();
        self.kind(expr, depth) == Some(Kind::Str)
    }

    // The kind of `expr`, read in the innermost `depth` scopes.
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

    // The kind every one of `exprs` has, if they share one.
    fn same(&mut self, exprs: impl IntoIterator<Item = &'a Expr>, depth: usize) -> Option<Kind> {
        let mut kinds = Vec::new();
        for expr in exprs {
            kinds.push(self.kind(expr, depth)?);
        }
        shared(kinds)
    }

    // The kind of each item iterating `expr` gives.
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
            Expr::Name(name) if self.lookup(&name.id, depth).is_none() => match name.id.as_str() {
                "ascii" | "bin" | "chr" | "format" | "hex" | "input" | "oct" | "repr" | "str" => Some(Kind::Str),
                "int" | "len" | "ord" => Some(Kind::Int),
                "frozenset" | "list" | "reversed" | "set" | "sorted" | "tuple" => match arguments.args.as_ref() {
                    [iterable] if arguments.keywords.is_empty() => self.item(iterable, depth).and_then(items),
                    _ => None,
                },
                "range" => Some(Kind::IntItems),
                _ => None,
            },
            Expr::Name(name) => {
                let (bindings, _) = self.lookup(&name.id, depth)?;
                if bindings.imported {
                    self.library_call(call, depth)
                } else if let [class] = bindings.classes.as_slice() {
                    bindings.only_classes().then(|| instance_of(self.walker.checker, class)).flatten()
                } else {
                    bindings.only_functions().and_then(|functions| self.returns(functions))
                }
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                let receiver = self.kind(value, depth);
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
                    let members = Members::of_class(checker, class_at(checker, range)?, attr);
                    return members.only_methods().and_then(|methods| self.returns(methods));
                }
                if let Some((members, _)) = self.members(value, attr, depth) {
                    return members.only_methods().and_then(|methods| self.returns(methods));
                }
                self.library_call(call, depth)
            }
            _ => None,
        }
    }

    // What a call to a standard library function returns, for a function the module imports.
    fn library_call(&mut self, call: &'a ast::ExprCall, depth: usize) -> Option<Kind> {
        let name = self.qualified(&call.func, depth)?;
        let args = &call.arguments.args;
        let segments: Vec<&str> = name.iter().map(String::as_str).collect();
        match segments.as_slice() {
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
            && self.qualified(func, depth).is_some_and(|name| {
                name.len() == 2
                    && name[0] == "uuid"
                    && matches!(name[1].as_str(), "uuid1" | "uuid3" | "uuid4" | "uuid5")
            })
        {
            return Some(Kind::Str);
        }
        if let Some((members, class)) = self.members(value, attr, depth) {
            return self.member(&members, class, attr);
        }
        let receiver = self.kind(value, depth);
        if let Some(Kind::Instance(range)) = receiver {
            let checker = self.walker.checker;
            let members = Members::of_class(checker, class_at(checker, range)?, attr);
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
        let mut name = self.qualified(value, depth)?;
        name.push(attr.to_string());
        let segments: Vec<&str> = name.iter().map(String::as_str).collect();
        match segments.as_slice() {
            ["os", "curdir" | "extsep" | "linesep" | "name" | "pardir" | "pathsep" | "sep"]
            | ["sys", "executable" | "platform" | "version"] => Some(Kind::Str),
            ["string", constant] if STRING_CONSTANTS.contains(constant) => Some(Kind::Str),
            ["os", "environ"] => Some(Kind::StrItems),
            _ => None,
        }
    }

    // The qualified name of `expr`, an imported name or an attribute of one, when the module's own import binds the
    // name it starts with.
    fn qualified(&self, expr: &Expr, depth: usize) -> Option<Vec<String>> {
        let mut head = expr;
        while let Expr::Attribute(ast::ExprAttribute { value, .. }) = head {
            head = value;
        }
        let (bindings, scope) = self.lookup(&head.as_name_expr()?.id, depth)?;
        let semantic = self.walker.checker.semantic();
        let name = (scope == 0 && bindings.only_imported()).then(|| semantic.qualified_name(expr)).flatten()?;
        Some(name.segments().iter().map(ToString::to_string).collect())
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
        let key = (name, self.scope_key(depth));
        if let Some(index) = self.resolving.iter().position(|(other, at, _)| (*other, *at) == key) {
            self.relies_on = self.relies_on.min(index + 1);
            return self.resolving[index].2;
        }
        if let Some(kind) = self.resolved.get(&key) {
            return *kind;
        }
        let floor = self.resolving.len();
        let outer = std::mem::replace(&mut self.relies_on, usize::MAX);
        let kind = self.bound(&bindings, key, depth);
        if self.relies_on > floor {
            self.resolved.insert(key, kind);
        }
        self.relies_on = self.relies_on.min(outer);
        kind
    }

    // The kind every binding of a name gives it.
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
    fn entered(&self, expr: &Expr, depth: usize) -> Option<Kind> {
        let Expr::Call(ast::ExprCall { func, .. }) = expr else {
            return None;
        };
        self.qualified(func, depth)
            .is_some_and(|name| name.len() == 2 && name[0] == "tempfile" && name[1] == "TemporaryDirectory")
            .then_some(Kind::Str)
    }

    // The scope the innermost `depth` scopes end in, the module's as an empty range.
    fn scope_key(&self, depth: usize) -> TextRange {
        depth.checked_sub(1).map_or_else(TextRange::default, |index| body_range(self.scopes[index]))
    }

    // The innermost generator of a comprehension around the spot that binds `name`.
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

    // The bindings of `name` in the innermost of the `depth` scopes, then the module, that binds it, with the depth
    // that scope ends. A class body is skipped unless innermost, since its methods do not see it.
    fn lookup(&self, name: &str, depth: usize) -> Option<(Bindings<'a>, usize)> {
        let checker = self.walker.checker;
        self.scopes[..depth]
            .iter()
            .enumerate()
            .rev()
            .filter(|(index, scope)| !matches!(scope, Scope::Class(_)) || index + 1 == depth)
            .map(|(index, scope)| (Bindings::of_scope(checker, *scope, name), index + 1))
            .chain(std::iter::once_with(|| (Bindings::of_body(checker, checker.body(), name), 0)))
            .find(|(bindings, _)| bindings.binds())
    }

    // The kind every one of `functions` declares it returns.
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

    // How the class gives `attr` to the instances its method reads as `receiver`, with the depth of the class's
    // scope.
    fn members(&self, receiver: &Expr, attr: &str, depth: usize) -> Option<(Members<'a>, Class<'a>)> {
        let receiver = receiver.as_name_expr()?;
        let index = self.scopes[..depth].iter().rposition(|scope| matches!(scope, Scope::Function(_)))?;
        let (Scope::Function(method), Some(Scope::Class(class))) =
            (self.scopes[index], index.checked_sub(1).map(|at| self.scopes[at]))
        else {
            return None;
        };
        let checker = self.walker.checker;
        // The parameter must be the only binding of the name, so it still holds the instance.
        if self_parameter(checker, method) != Some(receiver.id.as_str())
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
        Some((Members::of_class(checker, class, attr), Class { def: class, depth: index - 1 }))
    }

    // What an attribute holds, from the annotation the class gives it, the property that computes it, or the values
    // the class and its methods set.
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
        // A subclass or another file may give it any value.
        let Class { def: class, depth: class_depth } = class;
        let project = self.walker.checker.project();
        let key = (class.range, attr);
        if members.values.is_empty()
            || project.is_base_class(&class.name)
            || project.is_attribute_written(attr)
            || Visibility::of(attr) == Visibility::Public && project.is_referenced(&class.name)
            || written_outside(self.walker.checker.body(), class, attr)
        {
            return None;
        }
        if let Some(&(_, _, started)) = self.reading.iter().find(|(range, name, _)| (*range, *name) == key) {
            self.relies_on = self.relies_on.min(started);
            return None;
        }
        self.reading.push((key.0, key.1, self.resolving.len()));
        let outer_at_spot = std::mem::replace(&mut self.at_spot, false);
        let mut kinds = Vec::new();
        for (value, method) in &members.values {
            let mut scopes = self.scopes[..=class_depth].to_vec();
            scopes.extend(method.map(Scope::Function));
            let depth = scopes.len();
            let outer = std::mem::replace(&mut self.scopes, scopes);
            kinds.push(self.kind(value, depth));
            self.scopes = outer;
        }
        self.at_spot = outer_at_spot;
        self.reading.pop();
        kinds.into_iter().collect::<Option<Vec<_>>>().and_then(shared)
    }
}

// A class around the spot, with its index among the scopes.
#[derive(Clone, Copy)]
struct Class<'a> {
    def: &'a ast::StmtClassDef,
    depth: usize,
}

// The kind all of `kinds` share, if there is at least one.
fn shared(kinds: impl IntoIterator<Item = Kind>) -> Option<Kind> {
    let mut kinds = kinds.into_iter();
    let first = kinds.next()?;
    kinds.all(|kind| kind == first).then_some(first)
}

// The kind `op` gives on operands of these kinds.
fn binary(op: Operator, sides: (Kind, Kind)) -> Option<Kind> {
    match (op, sides) {
        (Operator::Add, (left, right))
            if left == right
                && matches!(left, Kind::Str | Kind::Int | Kind::Bytes | Kind::StrItems | Kind::IntItems) =>
        {
            Some(left)
        }
        (Operator::Mult, (Kind::Str, Kind::Int) | (Kind::Int, Kind::Str)) => Some(Kind::Str),
        (
            Operator::Mult,
            (items @ (Kind::StrItems | Kind::IntItems), Kind::Int)
            | (Kind::Int, items @ (Kind::StrItems | Kind::IntItems)),
        ) => Some(items),
        (Operator::Mult | Operator::Sub | Operator::FloorDiv | Operator::Mod, (Kind::Int, Kind::Int)) => {
            Some(Kind::Int)
        }
        (Operator::Div, (Kind::Path, Kind::Str | Kind::Path)) => Some(Kind::Path),
        _ => None,
    }
}

// What a method of a value of `receiver` kind returns.
fn method(receiver: Kind, name: &str) -> Option<Kind> {
    match (receiver, name) {
        (Kind::Str, name) if STR_METHODS.contains(&name) => Some(Kind::Str),
        (Kind::Bytes, "decode" | "hex") | (Kind::Path, "as_posix" | "as_uri" | "read_text") => Some(Kind::Str),
        (Kind::Str, "partition" | "rpartition" | "rsplit" | "split" | "splitlines")
        | (Kind::StrItems, "keys" | "values") => Some(Kind::StrItems),
        (Kind::Str, "count" | "find" | "index" | "rfind" | "rindex") => Some(Kind::Int),
        (Kind::Str, "encode") => Some(Kind::Bytes),
        (
            Kind::Path,
            "absolute" | "expanduser" | "joinpath" | "relative_to" | "resolve" | "with_name" | "with_stem"
            | "with_suffix",
        ) => Some(Kind::Path),
        _ => None,
    }
}

// Methods of `str` that return a `str`.
const STR_METHODS: [&str; 21] = [
    "capitalize",
    "casefold",
    "center",
    "expandtabs",
    "format",
    "format_map",
    "join",
    "ljust",
    "lower",
    "lstrip",
    "removeprefix",
    "removesuffix",
    "replace",
    "rjust",
    "rstrip",
    "strip",
    "swapcase",
    "title",
    "translate",
    "upper",
    "zfill",
];

const PLATFORM_FUNCTIONS: [&str; 8] =
    ["machine", "node", "platform", "python_implementation", "python_version", "release", "system", "version"];

const STRING_CONSTANTS: [&str; 9] = [
    "ascii_letters",
    "ascii_lowercase",
    "ascii_uppercase",
    "digits",
    "hexdigits",
    "octdigits",
    "printable",
    "punctuation",
    "whitespace",
];

// `os.path` functions that return the type of their arguments.
const OS_PATH_FUNCTIONS: [&str; 10] = [
    "abspath",
    "basename",
    "dirname",
    "expanduser",
    "expandvars",
    "join",
    "normcase",
    "normpath",
    "realpath",
    "relpath",
];

const PATH_CLASSES: [&str; 6] = ["Path", "PosixPath", "PurePath", "PurePosixPath", "PureWindowsPath", "WindowsPath"];

fn body_range(scope: Scope) -> TextRange {
    let (body, end) = match scope {
        Scope::Function(function) => (&function.body, function.end()),
        Scope::Class(class) => (&class.body, class.end()),
        Scope::Lambda(lambda) => return lambda.body.range(),
    };
    TextRange::new(body.first().map_or(end, Ranged::start), end)
}

fn target_binds(target: &Expr, name: &str) -> bool {
    match target {
        Expr::Name(target) => target.id == name,
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().any(|elt| target_binds(elt, name))
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => target_binds(value, name),
        _ => false,
    }
}

// The name a method calls its instance, unless it is a static or class method.
fn self_parameter<'a>(checker: &Checker, method: &'a ast::StmtFunctionDef) -> Option<&'a str> {
    if method
        .decorator_list
        .iter()
        .any(|decorator| is_builtin_call(checker, &decorator.expression, &["staticmethod", "classmethod"]))
    {
        return None;
    }
    method
        .parameters
        .posonlyargs
        .iter()
        .chain(&method.parameters.args)
        .next()
        .map(|parameter| parameter.name().as_str())
}

fn is_builtin_call(checker: &Checker, expr: &Expr, names: &[&str]) -> bool {
    expr.as_name_expr().is_some_and(|name| names.contains(&name.id.as_str()) && !checker.semantic().is_bound(&name.id))
}

// Whether a decorator hands back the function's own result: `staticmethod`, `classmethod`, a `functools` cache or
// `typing.override`.
fn keeps_return(checker: &Checker, decorator: &ast::Decorator) -> bool {
    let expr = match &decorator.expression {
        Expr::Call(ast::ExprCall { func, .. }) => func.as_ref(),
        expr => expr,
    };
    is_builtin_call(checker, expr, &["staticmethod", "classmethod"])
        || checker.semantic().qualified_name(expr).is_some_and(|name| {
            matches!(
                name.segments(),
                ["functools", "cache" | "lru_cache"] | ["typing" | "typing_extensions", "override"]
            )
        })
}

fn is_property(checker: &Checker, decorator: &ast::Decorator) -> bool {
    is_builtin_call(checker, &decorator.expression, &["property"])
        || checker
            .semantic()
            .qualified_name(&decorator.expression)
            .is_some_and(|name| name.segments() == ["functools", "cached_property"])
}

// The kind an annotation declares.
fn annotation(checker: &Checker, expr: &Expr) -> Option<Kind> {
    let semantic = checker.semantic();
    let simple = |name: &str| match name {
        "str" => Some(Kind::Str),
        "int" => Some(Kind::Int),
        "bytes" => Some(Kind::Bytes),
        _ => None,
    };
    match expr {
        Expr::Name(name) if !semantic.is_bound(&name.id) => simple(&name.id),
        Expr::Name(name) => class_named(checker, &name.id)
            .map(|class| Kind::Instance(class.range))
            .or_else(|| path_class(checker, expr)),
        Expr::StringLiteral(string) => {
            let name = string.value.to_str();
            simple(name).or_else(|| class_named(checker, name).map(|class| Kind::Instance(class.range)))
        }
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::BitOr, right, .. })
            if right.is_none_literal_expr() && annotation(checker, left) == Some(Kind::Str) =>
        {
            Some(Kind::OptionalStr)
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            if semantic.match_typing_expr(value, "Final") {
                return annotation(checker, slice);
            }
            if semantic.match_typing_expr(value, "Optional") {
                return (annotation(checker, slice) == Some(Kind::Str)).then_some(Kind::OptionalStr);
            }
            let elts = match slice.as_ref() {
                Expr::Tuple(tuple) => tuple.elts.as_slice(),
                single => std::slice::from_ref(single),
            };
            if semantic.match_typing_expr(value, "Annotated") {
                return annotation(checker, elts.first()?);
            }
            let container = match value.as_ref() {
                Expr::Name(name) if !semantic.is_bound(&name.id) => name.id.to_string(),
                _ => {
                    let name = semantic.qualified_name(value)?;
                    match name.segments() {
                        ["typing" | "typing_extensions", container] | ["collections", "abc", container] => {
                            (*container).to_string()
                        }
                        _ => return None,
                    }
                }
            };
            if !CONTAINERS.contains(&container.as_str()) {
                return None;
            }
            let mut kinds = Vec::new();
            for elt in elts.iter().filter(|elt| !elt.is_ellipsis_literal_expr()) {
                kinds.push(annotation(checker, elt)?);
            }
            shared(kinds).and_then(items)
        }
        _ => path_class(checker, expr),
    }
}

fn path_class(checker: &Checker, expr: &Expr) -> Option<Kind> {
    checker.semantic().qualified_name(expr).and_then(|name| match name.segments() {
        ["pathlib", class] if PATH_CLASSES.contains(class) => Some(Kind::Path),
        _ => None,
    })
}

// The class the module defines once under `name`, at its top level.
fn class_named<'a>(checker: &Checker<'a>, name: &str) -> Option<&'a ast::StmtClassDef> {
    let [binding] = checker.semantic().bindings(name) else {
        return None;
    };
    if binding.kind != BindingKind::Class {
        return None;
    }
    checker.body().iter().find_map(|stmt| stmt.as_class_def_stmt().filter(|class| class.name.as_str() == name))
}

fn class_at<'a>(checker: &Checker<'a>, range: TextRange) -> Option<&'a ast::StmtClassDef> {
    checker.body().iter().find_map(|stmt| stmt.as_class_def_stmt().filter(|class| class.range == range))
}

// What calling the class gives: an instance, unless the class has a `__new__`, a metaclass or a decorator that may
// return something else.
fn instance_of(checker: &Checker, class: &ast::StmtClassDef) -> Option<Kind> {
    let plain = class.keywords().is_empty()
        && class.decorator_list.iter().all(|decorator| {
            let expr = match &decorator.expression {
                Expr::Call(ast::ExprCall { func, .. }) => func.as_ref(),
                expr => expr,
            };
            checker.semantic().qualified_name(expr).is_some_and(|name| {
                matches!(
                    name.segments(),
                    ["dataclasses", "dataclass"] | ["functools", "total_ordering"] | ["typing", "final"]
                )
            })
        })
        && !class
            .body
            .iter()
            .any(|stmt| stmt.as_function_def_stmt().is_some_and(|function| function.name.as_str() == "__new__"));
    (plain && class_named(checker, &class.name).is_some()).then_some(Kind::Instance(class.range))
}

// What a container of items of `item` kind is, if a kind tells it.
fn items(item: Kind) -> Option<Kind> {
    match item {
        Kind::Str => Some(Kind::StrItems),
        Kind::Int => Some(Kind::IntItems),
        _ => None,
    }
}

// Generic containers whose item type their parameters give; a mapping's keys and values must share it.
const CONTAINERS: [&str; 22] = [
    "AbstractSet",
    "Collection",
    "Dict",
    "FrozenSet",
    "Iterable",
    "Iterator",
    "List",
    "Mapping",
    "MutableMapping",
    "MutableSequence",
    "MutableSet",
    "Sequence",
    "Set",
    "Tuple",
    "dict",
    "frozenset",
    "list",
    "set",
    "tuple",
    "Reversible",
    "KeysView",
    "ValuesView",
];

#[derive(Clone, Copy)]
enum Value<'a> {
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
struct Bindings<'a> {
    values: Vec<Value<'a>>,
    declared: Option<Kind>,
    functions: Vec<&'a ast::StmtFunctionDef>,
    classes: Vec<&'a ast::StmtClassDef>,
    imported: bool,
    opaque: bool,
}

impl<'a> Bindings<'a> {
    fn of_scope(checker: &Checker<'a>, scope: Scope<'a>, name: &str) -> Self {
        match scope {
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
        }
    }

    fn of_body(checker: &Checker<'a>, body: &'a [Stmt], name: &str) -> Self {
        let mut bindings = Self::default();
        bindings.collect(checker, body, name);
        bindings
    }

    fn of_parameters(checker: &Checker, parameters: &ast::Parameters, name: &str) -> Self {
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

    fn collect(&mut self, checker: &Checker<'a>, body: &'a [Stmt], name: &str) {
        Collector { checker, name, bindings: self }.visit_body(body);
    }

    // An annotation the code shows no kind for, or two that disagree, leave the name unknown.
    fn declare(&mut self, kind: Option<Kind>) {
        match kind {
            Some(kind) if self.declared.is_none_or(|declared| declared == kind) => self.declared = Some(kind),
            _ => self.opaque = true,
        }
    }

    fn binds(&self) -> bool {
        self.declared.is_some()
            || self.opaque
            || self.imported
            || !self.values.is_empty()
            || !self.functions.is_empty()
            || !self.classes.is_empty()
    }

    fn only_imported(&self) -> bool {
        self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.functions.is_empty()
    }

    fn only_classes(&self) -> bool {
        !self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.functions.is_empty()
    }

    fn only_functions(&self) -> Option<&[&'a ast::StmtFunctionDef]> {
        (!self.imported && !self.opaque && self.values.is_empty() && self.declared.is_none() && self.classes.is_empty())
            .then_some(self.functions.as_slice())
    }
}

struct Collector<'c, 'a, 'b> {
    checker: &'c Checker<'a>,
    name: &'c str,
    bindings: &'b mut Bindings<'a>,
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
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
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
    fn visit_generators(&mut self, generators: &'a [ast::Comprehension]) {
        for generator in generators {
            self.visit_expr(&generator.iter);
            for condition in &generator.ifs {
                self.visit_expr(condition);
            }
        }
    }
}

fn is_name(target: &Expr, name: &str) -> bool {
    target.as_name_expr().is_some_and(|target| target.id == name)
}

// Whether `target` unpacks an item straight into `name`, as `a, b = ...` does.
fn unpacks(target: &Expr, name: &str) -> bool {
    match target {
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().any(|elt| is_name(elt, name))
        }
        _ => false,
    }
}

// Whether a `global` or `nonlocal` anywhere in `body`, nested defs included, declares `name`.
fn declares(body: &[Stmt], name: &str) -> bool {
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

// How a class gives an attribute to its instances: annotations, values set in its body or on the instance in its
// methods, and methods or properties of that name.
struct Members<'a> {
    declared: Option<Kind>,
    // Each value, with the method that sets it on the instance, or none for the class body.
    values: Vec<(&'a Expr, Option<&'a ast::StmtFunctionDef>)>,
    properties: Vec<&'a ast::StmtFunctionDef>,
    methods: Vec<&'a ast::StmtFunctionDef>,
    opaque: bool,
}

impl<'a> Members<'a> {
    fn of_class(checker: &Checker<'a>, class: &'a ast::StmtClassDef, attr: &str) -> Self {
        let body = Bindings::of_body(checker, &class.body, attr);
        let (properties, methods) = body
            .functions
            .iter()
            .partition(|function| function.decorator_list.iter().any(|decorator| is_property(checker, decorator)));
        let mut values = Vec::new();
        let mut opaque = body.opaque || body.imported;
        for value in &body.values {
            match *value {
                Value::Expr(expr) => values.push((expr, None)),
                Value::Item(_) | Value::Entered(_) => opaque = true,
            }
        }
        let mut members = Self { declared: body.declared, values, properties, methods, opaque };
        for stmt in &class.body {
            if let Stmt::FunctionDef(method) = stmt
                && let Some(receiver) = self_parameter(checker, method)
            {
                let mut writes = Writes { checker, receiver, attr, method, members: &mut members, nested: 0 };
                writes.visit_body(&method.body);
            }
        }
        members
    }

    fn only_methods(&self) -> Option<&[&'a ast::StmtFunctionDef]> {
        (!self.opaque && self.declared.is_none() && self.values.is_empty() && self.properties.is_empty())
            .then_some(self.methods.as_slice())
    }
}

// Whether code outside the class writes or deletes an attribute of this name, on any object.
fn written_outside(body: &[Stmt], class: &ast::StmtClassDef, attr: &str) -> bool {
    struct Outside<'c> {
        class: TextRange,
        attr: &'c str,
        found: bool,
    }
    impl<'a> Visitor<'a> for Outside<'_> {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if stmt.range() != self.class {
                visitor::walk_stmt(self, stmt);
            }
        }

        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Attribute(ast::ExprAttribute { ctx, attr, .. }) = expr {
                self.found |= *ctx != ExprContext::Load && attr == self.attr;
            }
            visitor::walk_expr(self, expr);
        }
    }
    let mut outside = Outside { class: class.range, attr, found: false };
    outside.visit_body(body);
    outside.found
}

// The writes a method makes to an attribute of its instance.
struct Writes<'c, 'a, 'm> {
    checker: &'c Checker<'a>,
    receiver: &'a str,
    attr: &'c str,
    method: &'a ast::StmtFunctionDef,
    members: &'m mut Members<'a>,
    // Inside a nested def the value would read names of a scope this does not track.
    nested: usize,
}

impl Writes<'_, '_, '_> {
    fn is_member(&self, target: &Expr) -> bool {
        matches!(target, Expr::Attribute(ast::ExprAttribute { value, attr, .. })
            if attr == self.attr && is_name(value, self.receiver))
    }
}

impl<'a> Visitor<'a> for Writes<'_, 'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                for target in targets {
                    if self.is_member(target) {
                        if self.nested == 0 {
                            self.members.values.push((value, Some(self.method)));
                        } else {
                            self.members.opaque = true;
                        }
                    } else {
                        self.visit_expr(target);
                    }
                }
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) if self.is_member(target) => {
                match self::annotation(self.checker, annotation) {
                    Some(kind) if self.members.declared.is_none_or(|declared| declared == kind) => {
                        self.members.declared = Some(kind);
                    }
                    _ => self.members.opaque = true,
                }
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {
                self.nested += 1;
                visitor::walk_stmt(self, stmt);
                self.nested -= 1;
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    // Any other write to the attribute, on the instance or on another object of the class, leaves it unknown.
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(ast::ExprAttribute { ctx, attr, .. }) = expr
            && *ctx != ExprContext::Load
            && attr == self.attr
        {
            self.members.opaque = true;
        }
        visitor::walk_expr(self, expr);
    }
}
