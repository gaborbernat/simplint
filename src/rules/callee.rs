use std::borrow::Cow;

use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::walker::{Scope, Walker};
use crate::project::Signature;
use crate::semantic::Semantic;
use crate::syntax::receiver;

// A function that a name or a `self.method` reference reaches, by its definition in this file or by the signature the
// index records for a function another module of the run defines.
pub(crate) struct Callee<'a> {
    signature: Cow<'a, Signature>,
    // The `def` of this file, which the index's signature of an imported function does not stand in for.
    pub(crate) function: Option<&'a ast::StmtFunctionDef>,
    // Whether the reference binds the first parameter, as `self.method` does.
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

    // A decorator may replace the function with anything, so what the definition says no longer holds for the name.
    pub(crate) fn is_decorated(&self) -> bool {
        self.signature.decorated
    }

    pub(crate) fn makes_coroutine(&self) -> bool {
        self.signature.coroutine
    }

    // The names of the parameters a positional argument fills, in order, after the bound one.
    pub(crate) fn positional(&self) -> Vec<&str> {
        self.signature.positional.iter().skip(usize::from(self.bound)).map(String::as_str).collect()
    }
}

// The `def` of this file that a name, or a `self.method` reference in `method` of `class`, reaches, and whether the
// reference binds its first parameter, as `self.method` does.
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

// The method a reference sits in, directly in a class body, with that class.
fn enclosing_method<'a>(walker: &Walker<'_, 'a>) -> Option<(&'a ast::StmtFunctionDef, &'a ast::StmtClassDef)> {
    match walker.scopes() {
        [.., Scope::Class(class), Scope::Function(method)] => Some((method, class)),
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
