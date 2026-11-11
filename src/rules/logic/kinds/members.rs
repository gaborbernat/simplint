use std::rc::Rc;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use super::Kind;
use super::bindings::{Value, bindings_in};
use super::declared::{annotation, is_property, self_parameter};
use crate::lint::checker::Checker;
use crate::lint::walker::Scope;
use crate::syntax::is_name;

pub(super) fn members_of<'a>(checker: &Checker<'a>, class: &'a ast::StmtClassDef, attr: &'a str) -> Rc<Members<'a>> {
    if let Some(members) = checker.state().kinds().members.borrow().get(&(class.range, attr)) {
        return Rc::clone(members);
    }
    let members = Rc::new(Members::of_class(checker, class, attr));
    checker.state().kinds().members.borrow_mut().insert((class.range, attr), Rc::clone(&members));
    members
}

// How a class gives an attribute to its instances: annotations, values set in its body or on the instance in its
// methods, and methods or properties of that name.
pub(super) struct Members<'a> {
    pub(super) declared: Option<Kind>,
    // Each value, with the method that sets it on the instance, or none for the class body.
    pub(super) values: Vec<(&'a Expr, Option<&'a ast::StmtFunctionDef>)>,
    pub(super) properties: Vec<&'a ast::StmtFunctionDef>,
    pub(super) methods: Vec<&'a ast::StmtFunctionDef>,
    pub(super) opaque: bool,
}

impl<'a> Members<'a> {
    pub(super) fn declare(&mut self, kind: Option<Kind>) {
        match kind {
            Some(kind) if self.declared.is_none_or(|declared| declared == kind) => self.declared = Some(kind),
            _ => self.opaque = true,
        }
    }

    pub(super) fn of_class(checker: &Checker<'a>, class: &'a ast::StmtClassDef, attr: &'a str) -> Self {
        let body = bindings_in(checker, Some(Scope::Class(class)), attr);
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
                && let Some(receiver) = self_parameter(method)
            {
                let mut writes = Writes { checker, receiver, attr, method, members: &mut members, nested: 0 };
                writes.visit_body(&method.body);
            }
        }
        members
    }

    pub(super) fn only_methods(&self) -> Option<&[&'a ast::StmtFunctionDef]> {
        (!self.opaque && self.declared.is_none() && self.values.is_empty() && self.properties.is_empty())
            .then_some(self.methods.as_slice())
    }
}

// Whether code outside the class writes or deletes an attribute of this name, on any object.
pub(super) fn written_outside(checker: &Checker, class: &ast::StmtClassDef, attr: &str) -> bool {
    struct Writes<'a>(FxHashMap<&'a str, Vec<TextRange>>);
    impl<'a> Visitor<'a> for Writes<'a> {
        fn visit_expr(&mut self, expr: &'a Expr) {
            if let Expr::Attribute(ast::ExprAttribute { ctx, attr, range, .. }) = expr
                && *ctx != ExprContext::Load
            {
                self.0.entry(attr.as_str()).or_default().push(*range);
            }
            visitor::walk_expr(self, expr);
        }
    }
    let writes = checker.state().kinds().attribute_writes.get_or_init(|| {
        let mut writes = Writes(FxHashMap::default());
        writes.visit_body(checker.body());
        writes.0
    });
    writes.get(attr).is_some_and(|ranges| ranges.iter().any(|range| !class.range.contains_range(*range)))
}

// The writes a method makes to an attribute of its instance.
pub(super) struct Writes<'c, 'a, 'm> {
    pub(super) checker: &'c Checker<'a>,
    pub(super) receiver: &'a str,
    pub(super) attr: &'c str,
    pub(super) method: &'a ast::StmtFunctionDef,
    pub(super) members: &'m mut Members<'a>,
    // Inside a nested def the value would read names of a scope this does not track.
    pub(super) nested: usize,
}

impl Writes<'_, '_, '_> {
    pub(super) fn is_member(&self, target: &Expr) -> bool {
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
                self.members.declare(self::annotation(self.checker, annotation));
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
