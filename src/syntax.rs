// AST helpers that rules, the linter and the project index all read: walking statement bodies, what a name's
// underscores say, and what a module's `__all__` and strings hold.
use std::borrow::Cow;

use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
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

// The parameter a method receives its instance or class in: the first positional one, unless the method is static.
pub fn receiver(method: &ast::StmtFunctionDef) -> Option<&str> {
    if decorator_names(&method.decorator_list).contains(&"staticmethod") {
        return None;
    }
    let parameters = &method.parameters;
    parameters.posonlyargs.iter().chain(&parameters.args).next().map(|parameter| parameter.name().as_str())
}

pub fn is_classmethod(method: &ast::StmtFunctionDef) -> bool {
    decorator_names(&method.decorator_list).contains(&"classmethod")
}

// The statements that run when the module loads, including those of its top-level `if` and `try` blocks, where
// modules pick an implementation by version or availability.
pub fn module_statements(body: &[Stmt]) -> impl Iterator<Item = &Stmt> {
    body.iter().flat_map(|stmt| -> Box<dyn Iterator<Item = &Stmt> + '_> {
        match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => Box::new(
                module_statements(body)
                    .chain(elif_else_clauses.iter().flat_map(|clause| module_statements(&clause.body))),
            ),
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
            names.iter().any(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str() == "__all__")
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
    target.as_name_expr().is_some_and(|name| name.id.as_str() == "__all__")
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
