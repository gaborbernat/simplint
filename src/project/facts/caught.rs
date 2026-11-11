use ruff_python_ast::{self as ast, Expr, Stmt};

use super::Parts;
use super::visitor::FactVisitor;

impl<'a> FactVisitor<'a> {
    // A handler that names a module-level tuple catches each of its classes, so the tuple's classes are recorded.
    pub(super) fn bind_class_tuple(&mut self, stmt: &'a Stmt) {
        let (target, value) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => match targets.as_slice() {
                [target] => (target, value),
                _ => return,
            },
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => (target.as_ref(), value),
            _ => return,
        };
        if let Expr::Name(target) = target
            && self.top_level
            && self.parts.has(Parts::CAUGHT)
            && value.is_tuple_expr()
        {
            self.class_tuples.push((target.id.as_str().to_owned(), self.tuple_classes(value)));
        }
    }

    fn tuple_classes(&self, value: &Expr) -> Vec<(String, String)> {
        match value {
            Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().flat_map(|elt| self.tuple_classes(elt)).collect(),
            Expr::Starred(ast::ExprStarred { value, .. }) => self.tuple_classes(value),
            _ => self.class_path(value).into_iter().collect(),
        }
    }

    pub(super) fn catch_suppressed(&mut self, expr: &Expr) {
        if let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr
            && self.qualified(func).as_deref() == Some("contextlib.suppress")
        {
            for argument in &arguments.args {
                self.catch(argument);
            }
        }
    }

    pub(super) fn catch(&mut self, expr: &Expr) {
        if !self.parts.has(Parts::CAUGHT) {
            return;
        }
        if let Expr::Tuple(ast::ExprTuple { elts, .. }) = expr {
            for elt in elts {
                self.catch(elt);
            }
        } else if let Expr::Starred(ast::ExprStarred { value, .. }) = expr {
            self.catch(value);
        } else if let Some(path) = self.class_path(expr) {
            self.caught.push(path);
        }
    }
}

pub(super) fn class_argument<'a>(func: &Expr, arguments: &'a ast::Arguments) -> Option<&'a Expr> {
    if func.as_name_expr().is_some_and(|name| name.id.as_str() == "isinstance") {
        arguments.args.get(1)
    } else if func.as_attribute_expr().is_some_and(|attribute| matches!(attribute.attr.as_str(), "split" | "subgroup"))
    {
        arguments.args.first()
    } else {
        None
    }
}
