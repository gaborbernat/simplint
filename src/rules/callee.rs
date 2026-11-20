use std::borrow::Cow;

use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::walker::{Scope, Walker};
use crate::project::Signature;
use crate::semantic::Semantic;
use crate::syntax::receiver;

// Imported functions have a signature in the project index. Only local functions also have a syntax tree here.
pub(crate) struct Callee<'a> {
    signature: Cow<'a, Signature>,
    pub(crate) function: Option<&'a ast::StmtFunctionDef>,
    pub(crate) bound: bool,
}

impl<'a> Callee<'a> {
    pub(crate) fn of(walker: &Walker<'_, 'a>, expr: &'a Expr) -> Option<Self> {
        let semantic = walker.checker.semantic();
        if let Some((function, bound)) = local_function(semantic, expr, enclosing_method(walker)) {
            return Some(Self { signature: Cow::Owned(Signature::of(function)), function: Some(function), bound });
        }
        let name = expr.as_name_expr()?;
        if !semantic.reads_from_import(name) {
            return None;
        }
        let signature = walker.checker.project().imported_signature(name.id.as_str())?;
        Some(Self { signature: Cow::Borrowed(signature), function: None, bound: false })
    }

    pub(crate) fn name(&self) -> &str {
        &self.signature.name
    }

    // A decorator can replace the function and change its signature.
    pub(crate) fn is_decorated(&self) -> bool {
        self.signature.decorated
    }

    pub(crate) fn makes_coroutine(&self) -> bool {
        self.signature.coroutine
    }

    pub(crate) fn positional(&self) -> Vec<&str> {
        self.signature.positional.iter().skip(usize::from(self.bound)).map(String::as_str).collect()
    }
}

// The flag distinguishes bound methods from plain functions and static methods when matching positional arguments.
pub(crate) fn local_function<'a>(
    semantic: &Semantic<'a>,
    expr: &'a Expr,
    method: Option<(&'a ast::StmtFunctionDef, &'a ast::StmtClassDef)>,
) -> Option<(&'a ast::StmtFunctionDef, bool)> {
    match expr {
        Expr::Name(name) => semantic.function_of(name).map(|function| (function, false)),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
            let Expr::Name(object) = value.as_ref() else {
                return None;
            };
            let (method, class) = method?;
            if receiver(method) != Some(object.id.as_str()) || semantic.is_rebound(object.id.as_str()) {
                return None;
            }
            // Inherited helpers can occur in every test. Skip the class scan when the file has no binding for the
            // method name.
            if semantic.local_bindings(attr.as_str()).is_empty() && semantic.bindings(attr.as_str()).is_empty() {
                return None;
            }
            let mut namesakes = class.body.iter().filter(|stmt| binds(stmt, attr.as_str()));
            let function = namesakes.next()?.as_function_def_stmt()?;
            if namesakes.next().is_some() {
                return None;
            }
            Some((function, !semantic.decorated_by(&function.decorator_list, &[&["", "staticmethod"]])))
        }
        _ => None,
    }
}

fn binds(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::FunctionDef(function) => function.name.as_str() == name,
        Stmt::ClassDef(class) => class.name.as_str() == name,
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().any(|target| target.as_name_expr().is_some_and(|target| target.id.as_str() == name))
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            target.as_name_expr().is_some_and(|target| target.id.as_str() == name)
        }
        _ => false,
    }
}

fn enclosing_method<'a>(walker: &Walker<'_, 'a>) -> Option<(&'a ast::StmtFunctionDef, &'a ast::StmtClassDef)> {
    match walker.scopes() {
        [.., Scope::Class(class), Scope::Function(method)] => Some((method, class)),
        _ => None,
    }
}
