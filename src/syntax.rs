// AST helpers that rules, the linter and the project index all read: walking statement bodies, what a name's
// underscores say, and what a module's `__all__` and strings hold.
use std::borrow::Cow;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_expression;
use ruff_text_size::{Ranged, TextRange};

pub fn for_each_body<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    walk_bodies(body, None, visit);
}

fn walk_bodies<'a>(body: &'a [Stmt], owner: Option<&'a Stmt>, visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    visit(body, owner);
    for stmt in body {
        for_each_child_body(stmt, &mut |child| walk_bodies(child, Some(stmt), visit));
    }
}

pub fn for_each_child_body<'a>(stmt: &'a Stmt, visit: &mut impl FnMut(&'a [Stmt])) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { body, .. })
        | Stmt::ClassDef(ast::StmtClassDef { body, .. })
        | Stmt::With(ast::StmtWith { body, .. }) => visit(body),
        Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
            visit(body);
            visit(orelse);
        }
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            visit(body);
            for clause in elif_else_clauses {
                visit(&clause.body);
            }
        }
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
            visit(body);
            for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                visit(&handler.body);
            }
            visit(orelse);
            visit(finalbody);
        }
        Stmt::Match(ast::StmtMatch { cases, .. }) => {
            for case in cases {
                visit(&case.body);
            }
        }
        _ => {}
    }
}

// What a name's leading underscores say about who may use it. The two readings of "public" differ on dunders:
// `from module import *` without `__all__` skips every name with a leading underscore, dunders included, while the
// typing spec and PEP 8 treat only a non-dunder underscore name as internal, so `__version__` is public.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility {
    Public,
    Dunder,
    Private,
}

impl Visibility {
    pub(crate) fn of(name: &str) -> Self {
        if !name.starts_with('_') {
            Self::Public
        } else if is_dunder(name) {
            Self::Dunder
        } else {
            Self::Private
        }
    }
}

// The name an expression such as `typing.Final` or `@name.setter` ends with.
pub fn last_segment(expr: &Expr) -> Option<&str> {
    UnqualifiedName::from_expr(expr)?.segments().last().copied()
}

// The last segment of each decorator, so `typing.override` and `@name.setter` read as `override` and `setter`.
pub fn decorator_names(decorators: &[ast::Decorator]) -> Vec<&str> {
    decorators.iter().filter_map(|decorator| last_segment(&decorator.expression)).collect()
}

// Whether a decorator's last name is one of `names`, so `typing.overload` and `overload` both match `"overload"`.
pub fn has_decorator(decorators: &[ast::Decorator], names: &[&str]) -> bool {
    decorator_names(decorators).iter().any(|name| names.contains(name))
}

// Whether a function is an `@overload` stub, which the type checker reads in place of the implementation below it.
pub fn is_overload(decorators: &[ast::Decorator]) -> bool {
    has_decorator(decorators, &["overload"])
}

// Whether `expr` is the plain name `name`.
pub fn is_name(expr: &Expr, name: &str) -> bool {
    expr.as_name_expr().is_some_and(|found| found.id.as_str() == name)
}

// The object and the name argument of a call to one of `builtins` with two positional arguments, such as `getattr`,
// which reads the attribute named by the second.
pub fn attribute_call<'a>(expr: &'a Expr, builtins: &[&str]) -> Option<(&'a Expr, &'a Expr)> {
    let ast::ExprCall { func, arguments, .. } = expr.as_call_expr()?;
    let [object, attribute, ..] = &*arguments.args else {
        return None;
    };
    builtins.contains(&func.as_name_expr()?.id.as_str()).then_some((object, attribute))
}

// The name an `import` alias binds: its `as` name, or the name it imports.
pub fn alias_binding(alias: &ast::Alias) -> &str {
    alias.asname.as_ref().unwrap_or(&alias.name).as_str()
}

// Whether `expr` runs its body once per pass of the iteration around it: a lambda or a comprehension.
pub fn is_repeating_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_))
}

// Whether `expr`, a call or a decorator, passes `arg=True`.
pub fn keyword_is_true(expr: &Expr, arg: &str) -> bool {
    expr.as_call_expr()
        .and_then(|call| call.arguments.find_keyword(arg))
        .is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| literal.value))
}

// The `@dataclass` decorator of `class`, bare or called, where `is_dataclass` resolves the decorator's function.
pub fn dataclass_decorator<'a>(
    class: &'a ast::StmtClassDef,
    is_dataclass: &impl Fn(&Expr) -> bool,
) -> Option<&'a Expr> {
    class
        .decorator_list
        .iter()
        .map(|decorator| &decorator.expression)
        .find(|expr| is_dataclass(expr.as_call_expr().map_or(expr, |call| &call.func)))
}

// The parameter a method receives its instance or class in: the first positional one, unless the method is static.
pub fn receiver(method: &ast::StmtFunctionDef) -> Option<&str> {
    if decorator_names(&method.decorator_list).contains(&"staticmethod") {
        return None;
    }
    first_positional(&method.parameters)
}

pub fn first_positional(parameters: &ast::Parameters) -> Option<&str> {
    parameters.posonlyargs.iter().chain(&parameters.args).next().map(|parameter| parameter.name().as_str())
}

// An attribute a method writes on its instance through its receiver, with the statement that writes it, the one after
// it in the same body, and whether the write is an `object.__setattr__` call, which a frozen dataclass makes.
pub struct InstanceWrite<'a> {
    pub attr: &'a str,
    pub stmt: &'a Stmt,
    pub next: Option<&'a Stmt>,
    pub method: &'a ast::StmtFunctionDef,
    pub through_object: bool,
}

// Every write to `receiver.attr` in the methods of `class`, nested defs included, for whatever the receiver is named.
// A class method's receiver is the class, so callers that want instances skip those.
pub fn instance_writes(class: &ast::StmtClassDef) -> Vec<InstanceWrite<'_>> {
    let mut writes = Vec::new();
    for method in module_statements(&class.body).filter_map(Stmt::as_function_def_stmt) {
        let Some(receiver) = receiver(method) else {
            continue;
        };
        for_each_body(&method.body, &mut |body, _| {
            for (index, stmt) in body.iter().enumerate() {
                let next = body.get(index + 1);
                if let Some(attr) = set_through_object(stmt, receiver) {
                    writes.push(InstanceWrite { attr, stmt, next, method, through_object: true });
                }
                for target in write_targets(stmt) {
                    if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = target
                        && is_name(value, receiver)
                    {
                        writes.push(InstanceWrite { attr: attr.as_str(), stmt, next, method, through_object: false });
                    }
                }
            }
        });
    }
    writes
}

// The attribute an `object.__setattr__(receiver, "name", value)` call sets, which a frozen dataclass uses to set its
// own attributes.
fn set_through_object<'a>(stmt: &'a Stmt, receiver: &str) -> Option<&'a str> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr()?;
    let is_object = is_name(&attribute.value, "object");
    match call.arguments.args.as_ref() {
        [Expr::Name(target), Expr::StringLiteral(name), ..]
            if is_object && attribute.attr.as_str() == "__setattr__" && target.id.as_str() == receiver =>
        {
            Some(name.value.to_str())
        }
        _ => None,
    }
}

// The expressions a statement stores into, with tuples, lists and starred targets taken apart, in the order they are
// written.
fn write_targets(stmt: &Stmt) -> Vec<&Expr> {
    let mut pending: Vec<&Expr> = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) | Stmt::Delete(ast::StmtDelete { targets, .. }) => {
            targets.iter().collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })
        | Stmt::AugAssign(ast::StmtAugAssign { target, .. })
        | Stmt::For(ast::StmtFor { target, .. }) => vec![target.as_ref()],
        Stmt::With(ast::StmtWith { items, .. }) => {
            items.iter().filter_map(|item| item.optional_vars.as_deref()).collect()
        }
        _ => Vec::new(),
    };
    let mut targets = Vec::new();
    let mut next = 0;
    while let Some(&target) = pending.get(next) {
        next += 1;
        match target {
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                pending.extend(elts);
            }
            Expr::Starred(ast::ExprStarred { value, .. }) => pending.push(value),
            _ => targets.push(target),
        }
    }
    targets
}

// Each expression `pick` maps to a value, in the order a `Visitor` walks them.
pub struct ExprCollector<F, T> {
    pick: F,
    pub found: Vec<T>,
}

impl<F, T> ExprCollector<F, T> {
    pub fn new<'a>(pick: F) -> Self
    where
        F: FnMut(&'a Expr) -> Option<T>,
    {
        Self { pick, found: Vec::new() }
    }
}

impl<'a, F: FnMut(&'a Expr) -> Option<T>, T> Visitor<'a> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(value) = (self.pick)(expr) {
            self.found.push(value);
        }
        visitor::walk_expr(self, expr);
    }
}

pub fn is_classmethod(method: &ast::StmtFunctionDef) -> bool {
    decorator_names(&method.decorator_list).contains(&"classmethod")
}

// The statements that run when the module loads, including those of its top-level `if`, `try` and `match` blocks, where
// modules pick an implementation by version, availability or platform.
pub fn module_statements(body: &[Stmt]) -> impl Iterator<Item = &Stmt> {
    body.iter().flat_map(|stmt| -> Box<dyn Iterator<Item = &Stmt> + '_> {
        match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => Box::new(
                module_statements(body)
                    .chain(elif_else_clauses.iter().flat_map(|clause| module_statements(&clause.body))),
            ),
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                Box::new(cases.iter().flat_map(|case| module_statements(&case.body)))
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => Box::new(
                module_statements(body)
                    .chain(handlers.iter().flat_map(|handler| match handler {
                        ast::ExceptHandler::ExceptHandler(handler) => module_statements(&handler.body),
                    }))
                    .chain(module_statements(orelse))
                    .chain(module_statements(finalbody)),
            ),
            _ => Box::new(std::iter::once(stmt)),
        }
    })
}

pub enum Exports<'a> {
    Absent,
    Listed(Vec<&'a str>),
    Dynamic,
}

impl Exports<'_> {
    // Whether `__all__` lists `name`, or may list it because it cannot be read statically.
    pub fn may_list(&self, name: &str) -> bool {
        match self {
            Self::Listed(names) => names.contains(&name),
            Self::Dynamic => true,
            Self::Absent => false,
        }
    }
}

pub fn dunder_all(body: &[Stmt]) -> Exports<'_> {
    // An `__all__` bound inside an `if` or `try` depends on how the module runs, so its contents are unknown.
    let mut nested = false;
    for_each_body(body, &mut |statements, owner| {
        nested |= owner.is_some() && statements.iter().any(|stmt| all_binding(stmt).is_some());
    });
    if nested {
        return Exports::Dynamic;
    }
    // `from ._impl import __all__` re-exports a list this module cannot see.
    let imported = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias_binding(alias) == "__all__")
        }
        _ => false,
    });
    if imported {
        return Exports::Dynamic;
    }
    let mut exports = Exports::Absent;
    for stmt in body {
        let (resets, elements): (bool, Option<Vec<&Expr>>) = if let Some(binding) = all_binding(stmt) {
            (binding.operator.is_none(), all_literal(stmt).map(|(_, elts)| elts.iter().collect()))
        } else if let Some((method, arguments)) = all_mutation(stmt) {
            let elements = match (method, &*arguments.args) {
                ("append", [element]) => Some(vec![element]),
                ("extend", [Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })]) => {
                    Some(elts.iter().collect())
                }
                _ => None,
            };
            (false, elements)
        } else {
            continue;
        };
        let names: Option<Vec<&str>> = elements.and_then(|elements| {
            elements.iter().map(|element| Some(element.as_string_literal_expr()?.value.to_str())).collect()
        });
        exports = match (exports, names) {
            (_, None) | (Exports::Dynamic, _) => Exports::Dynamic,
            (Exports::Listed(mut listed), Some(names)) if !resets => {
                listed.extend(names);
                Exports::Listed(listed)
            }
            (_, Some(names)) => Exports::Listed(names),
        };
    }
    exports
}

pub struct AllBinding<'a> {
    pub value: Option<&'a Expr>,
    // Set for `__all__ += ...`, which extends the list instead of replacing it.
    pub operator: Option<ast::Operator>,
}

pub fn all_binding(stmt: &Stmt) -> Option<AllBinding<'_>> {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.iter().any(is_all_target) => {
            Some(AllBinding { value: Some(value), operator: None })
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: value.as_deref(), operator: None })
        }
        Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: Some(value), operator: Some(*op) })
        }
        _ => None,
    }
}

// A method call on `__all__` as a statement, such as `__all__.append("name")`: the method and its arguments.
pub fn all_mutation(stmt: &Stmt) -> Option<(&str, &ast::Arguments)> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr().filter(|attribute| is_all_target(&attribute.value))?;
    Some((attribute.attr.as_str(), &call.arguments))
}

pub fn all_literal(stmt: &Stmt) -> Option<(TextRange, &[Expr])> {
    // In `alias = __all__ = [...]` the list has a second name that can change it out of sight.
    if let Stmt::Assign(ast::StmtAssign { targets, .. }) = stmt
        && !targets.first().is_some_and(is_all_target)
    {
        return None;
    }
    match all_binding(stmt)? {
        AllBinding {
            value: Some(value @ (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }))),
            operator: None | Some(ast::Operator::Add),
        } => Some((value.range(), elts)),
        _ => None,
    }
}

pub fn is_all_target(target: &Expr) -> bool {
    is_name(target, "__all__")
}

// The names an assignment target binds, through tuple and list unpacking and starred targets.
pub fn target_names(target: &Expr) -> Vec<&str> {
    match target {
        Expr::Name(ast::ExprName { id, .. }) => vec![id.as_str()],
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().flat_map(target_names).collect()
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => target_names(value),
        _ => Vec::new(),
    }
}

// The text of every string and bytes literal of the module, and of each literal part of an f-string or t-string, with
// its range.
pub(crate) fn string_values(body: &[Stmt]) -> Vec<(TextRange, Cow<'_, str>)> {
    let mut values = StringValues(Vec::new());
    values.visit_body(body);
    values.0
}

struct StringValues<'a>(Vec<(TextRange, Cow<'a, str>)>);

impl<'a> Visitor<'a> for StringValues<'a> {
    fn visit_string_literal(&mut self, string_literal: &'a ast::StringLiteral) {
        self.0.push((string_literal.range, Cow::Borrowed(&string_literal.value)));
    }

    fn visit_bytes_literal(&mut self, bytes_literal: &'a ast::BytesLiteral) {
        self.0.push((bytes_literal.range, String::from_utf8_lossy(&bytes_literal.value)));
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        if let ast::InterpolatedStringElement::Literal(literal) = element {
            self.0.push((literal.range, Cow::Borrowed(&literal.value)));
        }
        visitor::walk_interpolated_string_element(self, element);
    }
}

// Ruff's own return visitor stopped reporting generators, so a function's `return` statements and whether it yields
// are collected here, skipping nested scopes.
#[derive(Default)]
pub struct ReturnVisitor<'a> {
    pub returns: Vec<&'a ast::StmtReturn>,
    pub is_generator: bool,
}

impl<'a> Visitor<'a> for ReturnVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Return(stmt) => self.returns.push(stmt),
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Yield(_) | Expr::YieldFrom(_) = expr {
            self.is_generator = true;
        } else {
            visitor::walk_expr(self, expr);
        }
    }
}

pub fn is_main_guard(stmt: &Stmt) -> bool {
    stmt.as_if_stmt().is_some_and(|stmt| is_main_test(&stmt.test))
}

// Whether a test is `__name__ == "__main__"`, either way round.
pub fn is_main_test(test: &Expr) -> bool {
    let Expr::Compare(ast::ExprCompare { ops, operands, .. }) = test else {
        return false;
    };
    let is_main =
        |expr: &Expr| expr.as_string_literal_expr().is_some_and(|literal| literal.value.to_str() == "__main__");
    match (&**ops, &**operands) {
        ([ast::CmpOp::Eq], [left, right]) => {
            (is_name(left, "__name__") && is_main(right)) || (is_main(left) && is_name(right, "__name__"))
        }
        _ => false,
    }
}

// The callables that build a mock, which the testing rules and the receiver tracking of the project share.
pub(crate) const MOCK_CLASSES: &[&str] =
    &["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock", "PropertyMock", "create_autospec"];

// The names an annotation or bound refers to, through subscripts, unions and string annotations.
pub(crate) fn names_in(expr: &Expr) -> Vec<String> {
    let mut names = Vec::new();
    TypeNames { names: &mut names }.visit_expr(expr);
    names
}

struct TypeNames<'n> {
    names: &'n mut Vec<String>,
}

impl<'a> Visitor<'a> for TypeNames<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => self.names.push(name.id.to_string()),
            Expr::StringLiteral(literal) => {
                if let Ok(parsed) = parse_expression(literal.value.to_str()) {
                    TypeNames { names: self.names }.visit_expr(parsed.expr());
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}
