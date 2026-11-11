use std::ptr;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, CmpOp, Expr, Stmt, UnaryOp};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::syntax::alias_binding;

const MODULES: [&str; 2] = ["re", "regex"];

// The module functions that take a pattern first, and the methods of a compiled pattern with the same names.
const FUNCTIONS: [&str; 9] = ["compile", "match", "search", "fullmatch", "finditer", "findall", "split", "sub", "subn"];

// The methods that return match objects, whose groups a read of the result reaches.
const MATCH_METHODS: [&str; 4] = ["match", "search", "fullmatch", "finditer"];

// The methods of a match object that read a group, whose call leaves no group readable beyond the fix's reach.
const GROUP_READS: [&str; 4] = ["group", "start", "end", "span"];

// The flags of `re` and `re.RegexFlag`, other than `VERBOSE` and `X`, that leave verbose mode off.
const NON_VERBOSE_FLAGS: &[&str] = &[
    "A",
    "ASCII",
    "DEBUG",
    "DOTALL",
    "I",
    "IGNORECASE",
    "L",
    "LOCALE",
    "M",
    "MULTILINE",
    "NOFLAG",
    "S",
    "T",
    "TEMPLATE",
    "U",
    "UNICODE",
];

// Where the value of an expression goes. A name is bound in the scope of the function, lambda or module it is in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context<'a> {
    // Only tested for truth, or compared to `None`, so no group reaches the code through it.
    Test,
    Discard,
    Bind { name: &'a str, scope: Scope },
    Other,
}

// A function or lambda, by its range, or the module when `None`.
pub(crate) type Scope = Option<TextRange>;

// Which groups a read of a match object or a pattern reaches.
#[derive(Clone, Copy)]
struct Reads {
    // A numbered group from 1 on, which the code reads by its position.
    index: bool,
    // Any group, numbered or named.
    groups: bool,
}

impl Reads {
    const NONE: Self = Self { index: false, groups: false };
    const NAMED: Self = Self { index: false, groups: true };
    const NUMBERED: Self = Self { index: true, groups: true };
}

// A use of a name, as far as the groups of its value go.
enum Usage {
    Test,
    Method { call: TextRange, reads: Reads },
    Subscript(Reads),
    Other,
}

// A string or bytes literal, which implicit concatenation makes of several parts.
pub(crate) struct Literal<'a> {
    pub(crate) range: TextRange,
    // Each part's decoded bytes and the range of its source text between the quotes.
    pub(crate) parts: Vec<(&'a [u8], TextRange)>,
}

impl<'a> Literal<'a> {
    fn of(expr: &'a Expr) -> Option<Self> {
        let parts = match expr {
            Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
                value.iter().map(|part| (part.value.as_bytes(), part.content_range())).collect()
            }
            Expr::BytesLiteral(ast::ExprBytesLiteral { value, .. }) => {
                value.iter().map(|part| (&part.value[..], part.content_range())).collect()
            }
            _ => return None,
        };
        Some(Self { range: expr.range(), parts })
    }

    pub(crate) fn decoded(&self) -> Vec<u8> {
        self.parts.iter().flat_map(|(value, _)| value.iter().copied()).collect()
    }
}

// A call of a `re` function with a literal pattern, or a method called on a name.
pub(crate) struct Call<'a> {
    pub(crate) attr: &'a str,
    pub(crate) context: Context<'a>,
    // The pattern a `re` call takes; `None` for a method call on a name.
    pub(crate) literal: Option<Literal<'a>>,
    // Whether verbose mode is on, or `None` when the flags argument is one the checker cannot resolve.
    pub(crate) verbose: Option<bool>,
    // Whether the replacement of a `sub` call is a literal without a backslash, so it names no group.
    plain_replacement: bool,
}

// The calls, bindings and name uses of a module, gathered in one walk so the decisions read them without another.
pub(crate) struct Facts<'c, 'a> {
    checker: &'c Checker<'a>,
    calls: FxHashMap<TextRange, Call<'a>>,
    // How many times each name is bound in each scope.
    bindings: FxHashMap<(Scope, &'a str), usize>,
    // The bindings a read the walk does not follow may reach: class attributes, and names a function declares `global`
    // or `nonlocal`.
    escaping: FxHashSet<(Scope, &'a str)>,
    // Whether the walk is directly in a class body, where a name it binds is an attribute.
    class_body: bool,
    // Each name use with the scopes it sits in, innermost first, until it is resolved to the scope that binds it.
    pending: Vec<(Vec<Scope>, &'a str, Usage)>,
    usages: FxHashMap<(Scope, &'a str), Vec<Usage>>,
    // The names whose match objects the code reads by index, which each group site of a name asks about.
    read_by_index: FxHashSet<(Scope, &'a str)>,
    // The expressions holding the current one, outermost first, ending with the current one.
    exprs: Vec<&'a Expr>,
    // The statements holding the current expression, innermost last.
    stmts: Vec<&'a Stmt>,
    // The scopes the current node sits in, innermost last, starting with the module.
    scopes: Vec<Scope>,
}

impl<'c, 'a> Facts<'c, 'a> {
    pub(crate) fn collect(checker: &'c Checker<'a>) -> Self {
        let mut facts = Facts {
            checker,
            calls: FxHashMap::default(),
            bindings: FxHashMap::default(),
            escaping: FxHashSet::default(),
            class_body: false,
            pending: Vec::new(),
            usages: FxHashMap::default(),
            read_by_index: FxHashSet::default(),
            exprs: Vec::new(),
            stmts: Vec::new(),
            scopes: vec![None],
        };
        // A `re` function resolves only through an import that spells its module, so a file that never spells `re` or
        // `regex` has no pattern to read.
        if MODULES.iter().any(|module| checker.name_token_count(module) > 0) {
            facts.visit_body(checker.body());
            facts.resolve_usages();
            facts.read_by_index =
                facts.usages.keys().copied().filter(|&(scope, name)| facts.reads_index_of(scope, name)).collect();
        }
        facts
    }

    // The calls of `re` functions with a literal pattern, in the order they sit in the source.
    pub(crate) fn sites(&self) -> Vec<(&Literal<'a>, &Call<'a>)> {
        let mut sites: Vec<(&Literal<'a>, &Call<'a>)> =
            self.calls.values().filter_map(|call| Some((call.literal.as_ref()?, call))).collect();
        sites.sort_by_key(|(literal, _)| literal.range.start());
        sites
    }
}

impl<'a> Visitor<'a> for Facts<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        // The expressions of an enclosing statement are not parents of this one's, so they wait outside.
        let exprs = std::mem::take(&mut self.exprs);
        self.stmts.push(stmt);
        match stmt {
            Stmt::FunctionDef(function) => {
                self.bind(function.name.as_str());
                // Decorators, defaults and annotations run in the enclosing scope, so they are walked before the
                // function's own scope and parameters exist.
                for decorator in &function.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = &function.type_params {
                    self.visit_type_params(type_params);
                }
                self.visit_parameters(&function.parameters);
                if let Some(returns) = &function.returns {
                    self.visit_annotation(returns);
                }
                self.scopes.push(Some(function.range));
                let class_body = std::mem::replace(&mut self.class_body, false);
                self.bind_parameters(&function.parameters);
                self.visit_body(&function.body);
                self.class_body = class_body;
                self.scopes.pop();
            }
            Stmt::ClassDef(class) => {
                self.bind(class.name.as_str());
                let class_body = std::mem::replace(&mut self.class_body, true);
                visitor::walk_stmt(self, stmt);
                self.class_body = class_body;
            }
            // The declared name is shared with the module, or with an enclosing function, so a pattern bound there
            // escapes too.
            Stmt::Global(ast::StmtGlobal { names, .. }) => {
                let scope = self.scope();
                self.escaping.extend(names.iter().flat_map(|name| [(scope, name.as_str()), (None, name.as_str())]));
            }
            Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                for range in self.scopes.iter().flatten() {
                    self.escaping.extend(names.iter().map(|name| (Some(*range), name.as_str())));
                }
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                for alias in names {
                    self.bind(alias_binding(alias));
                }
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
        self.stmts.pop();
        self.exprs = exprs;
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        if let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), .. }) = handler {
            self.bind(name.as_str());
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.exprs.push(expr);
        match expr {
            Expr::Call(call) => self.record_call(expr, call),
            Expr::Name(name) if name.ctx.is_load() => self.record_usage(name.id.as_str()),
            Expr::Name(name) if name.ctx.is_store() => self.bind(name.id.as_str()),
            _ => {}
        }
        if let Expr::Lambda(lambda) = expr {
            // A lambda's defaults run in the enclosing scope, as a function's do.
            if let Some(parameters) = &lambda.parameters {
                self.visit_parameters(parameters);
            }
            self.scopes.push(Some(lambda.range));
            let class_body = std::mem::replace(&mut self.class_body, false);
            if let Some(parameters) = &lambda.parameters {
                self.bind_parameters(parameters);
            }
            self.visit_expr(&lambda.body);
            self.class_body = class_body;
            self.scopes.pop();
        } else {
            visitor::walk_expr(self, expr);
        }
        self.exprs.pop();
    }
}

impl<'a> Facts<'_, 'a> {
    // The expression `up` levels above the current one, where 0 is the current one.
    fn ancestor(&self, up: usize) -> Option<&'a Expr> {
        self.exprs.len().checked_sub(up + 1).map(|index| self.exprs[index])
    }

    fn scope(&self) -> Scope {
        self.scopes.last().copied().flatten()
    }

    fn bind(&mut self, name: &'a str) {
        let key = (self.scope(), name);
        *self.bindings.entry(key).or_default() += 1;
        if self.class_body {
            self.escaping.insert(key);
        }
    }

    fn bind_parameters(&mut self, parameters: &'a ast::Parameters) {
        for parameter in parameters {
            self.bind(parameter.name().as_str());
        }
    }

    fn record_call(&mut self, expr: &'a Expr, call: &'a ast::ExprCall) {
        let record = match module_function(self.checker, &call.func) {
            Some(attr) => {
                let context = self.context();
                call.arguments.find_argument_value("pattern", 0).and_then(Literal::of).map(|literal| Call {
                    attr,
                    context,
                    literal: Some(literal),
                    verbose: call
                        .arguments
                        .find_argument_value("flags", flags_position(attr))
                        .map_or(Some(false), |flags| is_verbose(self.checker, flags)),
                    plain_replacement: call.arguments.find_argument_value("repl", 1).is_some_and(is_plain),
                })
            }
            None => match call.func.as_ref() {
                Expr::Attribute(ast::ExprAttribute { attr, .. })
                    if FUNCTIONS.contains(&attr.as_str()) || GROUP_READS.contains(&attr.as_str()) =>
                {
                    let context = self.context();
                    Some(Call {
                        attr: attr.as_str(),
                        context,
                        literal: None,
                        verbose: Some(false),
                        plain_replacement: call.arguments.find_argument_value("repl", 0).is_some_and(is_plain),
                    })
                }
                _ => None,
            },
        };
        if let Some(record) = record {
            self.calls.insert(expr.range(), record);
        }
    }

    // A name is only ever the value of an attribute, so an attribute parent is a read of the name's groups.
    fn record_usage(&mut self, name: &'a str) {
        let (parent, grandparent, top) = (self.ancestor(1), self.ancestor(2), self.ancestor(0));
        let usage = match parent {
            Some(Expr::Attribute(attribute)) => match grandparent {
                Some(Expr::Call(call)) if parent.is_some_and(|parent| ptr::eq(parent, call.func.as_ref())) => {
                    Usage::Method { call: call.range(), reads: method_reads(attribute.attr.as_str(), &call.arguments) }
                }
                _ => Usage::Other,
            },
            Some(Expr::Subscript(subscript)) if top.is_some_and(|top| ptr::eq(top, subscript.value.as_ref())) => {
                Usage::Subscript(subscript_reads(&subscript.slice))
            }
            _ if self.context() == Context::Test => Usage::Test,
            _ => Usage::Other,
        };
        let scopes = self.scopes.iter().rev().copied().collect();
        self.pending.push((scopes, name, usage));
    }

    // Each use goes to the innermost scope that binds its name, as a lookup in Python does. A use no scope binds names
    // no binding in the module, and is left out.
    fn resolve_usages(&mut self) {
        for (scopes, name, usage) in std::mem::take(&mut self.pending) {
            if let Some(scope) = scopes.into_iter().find(|scope| self.bindings.contains_key(&(*scope, name))) {
                self.usages.entry((scope, name)).or_default().push(usage);
            }
        }
    }

    // The context of the current expression. The operands of `and` and `or` share the context of the expression they
    // are in, so the walk goes up through them first.
    fn context(&self) -> Context<'a> {
        let mut up = 0;
        while let Some(Expr::BoolOp(_)) = self.ancestor(up + 1) {
            up += 1;
        }
        self.ancestor(up).map_or(Context::Other, |child| self.context_under(child, up))
    }

    // The context of `child`, which is `up` levels below the current expression.
    fn context_under(&self, child: &'a Expr, up: usize) -> Context<'a> {
        match self.ancestor(up + 1) {
            None => self.statement_context(child),
            Some(Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, .. })) => Context::Test,
            Some(Expr::Call(ast::ExprCall { func, arguments, .. }))
                if self.checker.semantic().is_builtin(func, "bool")
                    && arguments.args.first().is_some_and(|arg| ptr::eq(arg, child)) =>
            {
                Context::Test
            }
            Some(Expr::Compare(compare)) if is_none_test(compare, child) => Context::Test,
            Some(Expr::If(ast::ExprIf { test, .. })) if ptr::eq(test.as_ref(), child) => Context::Test,
            Some(_) => Context::Other,
        }
    }

    fn statement_context(&self, expr: &'a Expr) -> Context<'a> {
        let is = |candidate: &Expr| ptr::eq(candidate, expr);
        match self.stmts.last().copied() {
            Some(Stmt::Expr(ast::StmtExpr { value, .. })) if is(value) => Context::Discard,
            Some(Stmt::Assign(ast::StmtAssign { targets, value, .. })) if is(value) => match targets.as_slice() {
                [target] => self.bound_name(target),
                _ => Context::Other,
            },
            Some(Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. })) if is(value) => {
                self.bound_name(target)
            }
            Some(Stmt::For(ast::StmtFor { target, iter, .. })) if is(iter) => self.bound_name(target),
            Some(Stmt::If(ast::StmtIf { test, elif_else_clauses, .. }))
                if is(test) || elif_else_clauses.iter().any(|clause| clause.test.as_ref().is_some_and(is)) =>
            {
                Context::Test
            }
            Some(Stmt::While(ast::StmtWhile { test, .. }) | Stmt::Assert(ast::StmtAssert { test, .. })) if is(test) => {
                Context::Test
            }
            _ => Context::Other,
        }
    }

    fn bound_name(&self, target: &'a Expr) -> Context<'a> {
        match target {
            Expr::Name(name) => Context::Bind { name: name.id.as_str(), scope: self.scope() },
            _ => Context::Other,
        }
    }

    // Whether the value of a context reaches the code only as a truth test, a discarded value, or a name whose uses
    // read no group. A name has one binding, so the names followed from a call form a tree and never loop.
    fn result_flows(&self, context: Context<'a>) -> bool {
        match context {
            Context::Test | Context::Discard => true,
            Context::Bind { name, scope } => self.name_flows(scope, name),
            Context::Other => false,
        }
    }

    fn name_flows(&self, scope: Scope, name: &'a str) -> bool {
        !self.escaping.contains(&(scope, name))
            && !self.exported(scope, name)
            && self.bindings.get(&(scope, name)) == Some(&1)
            && self.usages_of(scope, name).iter().all(|usage| match usage {
                Usage::Test => true,
                Usage::Method { call, reads } => {
                    !reads.groups && self.calls.get(call).is_some_and(|call| self.call_flows(call))
                }
                Usage::Subscript(reads) => !reads.groups,
                Usage::Other => false,
            })
    }

    // Whether a call, a `re` function or a method on a name, leaves no group readable beyond the fix's reach.
    pub(crate) fn call_flows(&self, call: &Call<'a>) -> bool {
        match call.attr {
            "compile" | "match" | "search" | "fullmatch" => self.result_flows(call.context),
            "sub" | "subn" => call.plain_replacement,
            attr => GROUP_READS.contains(&attr),
        }
    }

    // A module name another module imports or `__all__` lists, whose reads this file does not all contain. Asked only
    // here, since reading the project is a cross-file read that the other rules sharing the walk do not declare.
    fn exported(&self, scope: Scope, name: &str) -> bool {
        scope.is_none() && self.checker.is_exported(name)
    }

    fn usages_of(&self, scope: Scope, name: &'a str) -> &[Usage] {
        self.usages.get(&(scope, name)).map_or(&[], Vec::as_slice)
    }

    // Whether the code reads a numbered group of the match objects `name` holds, or of the matches its pattern makes.
    pub(crate) fn reads_index(&self, scope: Scope, name: &'a str) -> bool {
        self.read_by_index.contains(&(scope, name))
    }

    fn reads_index_of(&self, scope: Scope, name: &'a str) -> bool {
        self.indexed(scope, name)
            || self.usages_of(scope, name).iter().any(|usage| match usage {
                Usage::Method { call, .. } => self.calls.get(call).is_some_and(|call| {
                    MATCH_METHODS.contains(&call.attr)
                        && matches!(call.context, Context::Bind { name, scope } if self.indexed(scope, name))
                }),
                _ => false,
            })
    }

    fn indexed(&self, scope: Scope, name: &'a str) -> bool {
        self.usages_of(scope, name).iter().any(|usage| match usage {
            Usage::Method { reads, .. } | Usage::Subscript(reads) => reads.index,
            Usage::Test | Usage::Other => false,
        })
    }
}

// The name of a `re` or `regex` function the callee resolves to, through an alias or a `from` import.
fn module_function(checker: &Checker, func: &Expr) -> Option<&'static str> {
    match checker.semantic().qualified_name(func)?.segments() {
        [module, attr] if MODULES.contains(module) => FUNCTIONS.into_iter().find(|function| function == attr),
        _ => None,
    }
}

// The position of the `flags` argument, which the `re` functions take after their other arguments.
fn flags_position(attr: &str) -> usize {
    match attr {
        "compile" => 1,
        "split" => 3,
        "sub" | "subn" => 4,
        _ => 2,
    }
}

// Whether a `flags` argument turns verbose mode on: `re.VERBOSE` or `re.X` in any `|` combination, or the value 64
// that `re.VERBOSE` holds. `None` when the flags are a name or other expression that does not resolve to a `re` flag,
// since the value it holds may turn verbose mode on.
fn is_verbose(checker: &Checker, flags: &Expr) -> Option<bool> {
    match flags {
        Expr::BinOp(ast::ExprBinOp { left, right, op: ast::Operator::BitOr, .. }) => {
            match (is_verbose(checker, left), is_verbose(checker, right)) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            }
        }
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(int), .. }) => {
            Some(int.as_u64().is_some_and(|value| value & 64 != 0))
        }
        _ => named_flag(checker, flags),
    }
}

// Whether a flag the `re` or `regex` module names turns verbose mode on or off. The last segment decides, so
// `re.RegexFlag.VERBOSE` is `re.VERBOSE`; a flag this does not know, such as one of `regex` alone, is `None`.
fn named_flag(checker: &Checker, flags: &Expr) -> Option<bool> {
    match checker.semantic().qualified_name(flags)?.segments() {
        [module, .., flag] if MODULES.contains(module) => match *flag {
            "VERBOSE" | "X" => Some(true),
            flag => NON_VERBOSE_FLAGS.contains(&flag).then_some(false),
        },
        _ => None,
    }
}

// A replacement that is a literal with no backslash names no group, so a fixed pattern cannot change what it reads.
fn is_plain(expr: &Expr) -> bool {
    Literal::of(expr).is_some_and(|literal| literal.parts.iter().all(|(value, _)| !value.contains(&b'\\')))
}

fn int_value(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(int), .. }) => int.as_u64(),
        _ => None,
    }
}

// The reads a method called on a name makes. The pattern methods return matches that are read elsewhere, so they
// read no group here.
fn method_reads(attr: &str, arguments: &ast::Arguments) -> Reads {
    match attr {
        "match" | "search" | "fullmatch" | "finditer" | "sub" | "subn" => Reads::NONE,
        "group" | "start" | "end" | "span" => Reads {
            index: arguments.args.iter().any(|arg| int_value(arg).is_some_and(|index| index >= 1)),
            groups: arguments.args.iter().any(|arg| int_value(arg) != Some(0)),
        },
        "groups" => Reads::NUMBERED,
        _ => Reads::NAMED,
    }
}

fn subscript_reads(slice: &Expr) -> Reads {
    match int_value(slice) {
        Some(0) => Reads::NONE,
        Some(_) => Reads::NUMBERED,
        None => Reads::NAMED,
    }
}

// `expr is None` or `expr is not None`, which a match object reaches only as a truth test.
fn is_none_test(compare: &ast::ExprCompare, operand: &Expr) -> bool {
    compare.as_single().is_some_and(|(left, op, right)| {
        ptr::eq(left, operand) && matches!(op, CmpOp::Is | CmpOp::IsNot) && right.is_none_literal_expr()
    })
}
