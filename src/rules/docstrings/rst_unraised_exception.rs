use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use super::raised_class;
use super::rst_missing_raises::builtin_bases;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{FieldKind, Owner};
use crate::syntax::last_segment;

pub fn rst_unraised_exception(checker: &Checker) {
    let functions = module_functions(checker.body());
    for target in checker.state().rst_targets(checker).iter().filter(|target| target.public(checker)) {
        let Owner::Function { def, class } = target.owner else {
            continue;
        };
        let mut effects = Effects::default();
        effects.collect(def, &functions, class, &mut FxHashSet::default());
        if effects.unknown {
            continue;
        }
        for field in target.docstring.fields(&[FieldKind::Raises]) {
            let Some(subject) = field.subject() else {
                continue;
            };
            let name = subject.rsplit('.').next().unwrap_or(subject);
            if effects.may_raise(name) {
                continue;
            }
            let text = field.text();
            checker.report_fix(
                Rule::RstUnraisedException,
                field.range_in(&target.docstring),
                format!("`{text}` documents `{name}`, which `{}` never raises", def.name),
                field.removal(checker, &target.docstring),
                format!("Remove `{text}`"),
            );
        }
    }
}

// The module's functions by name, which a call by name reaches, so their raises count as the caller's.
fn module_functions(body: &[Stmt]) -> FxHashMap<&str, &ast::StmtFunctionDef> {
    body.iter().filter_map(Stmt::as_function_def_stmt).map(|function| (function.name.as_str(), function)).collect()
}

// What a function body can raise: the exceptions its `raise` statements name, and the operations that raise built-in
// exceptions on their own. A call this module does not define, a loop over an unknown iterable, a `with`, an `await` or
// a `raise` of a value whose class is unknown can raise anything, so the body is then unknown.
#[derive(Default)]
struct Effects<'a> {
    raised: FxHashSet<&'a str>,
    subscript: bool,
    attribute: bool,
    arithmetic: bool,
    asserts: bool,
    unknown: bool,
    caught: Vec<Vec<&'a str>>,
    bound: FxHashMap<&'a str, Vec<&'a str>>,
    functions: Option<&'a FxHashMap<&'a str, &'a ast::StmtFunctionDef>>,
    class: Option<&'a ast::StmtClassDef>,
    pending: Vec<&'a ast::StmtFunctionDef>,
}

impl<'a> Effects<'a> {
    fn collect(
        &mut self,
        function: &'a ast::StmtFunctionDef,
        functions: &'a FxHashMap<&'a str, &'a ast::StmtFunctionDef>,
        class: Option<&'a ast::StmtClassDef>,
        seen: &mut FxHashSet<*const ast::StmtFunctionDef>,
    ) {
        self.functions = Some(functions);
        self.class = class;
        self.pending.push(function);
        while let Some(next) = self.pending.pop() {
            if seen.insert(std::ptr::from_ref(next)) {
                // Python checks the defaults and decorators when it defines the function, not when it calls it.
                self.visit_body(&next.body);
            }
        }
    }

    fn may_raise(&self, name: &str) -> bool {
        matches!(name, "Exception" | "BaseException")
            || self.raised.contains(name)
            || self.raised.iter().any(|raised| builtin_bases(raised).contains(&name))
            || self.subscript && matches!(name, "KeyError" | "IndexError" | "LookupError" | "TypeError")
            || self.attribute && name == "AttributeError"
            || self.arithmetic && matches!(name, "ZeroDivisionError" | "ArithmeticError" | "OverflowError" | "TypeError")
            || self.asserts && name == "AssertionError"
    }

    fn call(&mut self, call: &'a ast::ExprCall) {
        let resolved = match call.func.as_ref() {
            Expr::Name(name) => {
                let name = name.id.as_str();
                if matches!(
                    name,
                    "isinstance" | "issubclass" | "len" | "repr" | "id" | "type" | "callable" | "hasattr" | "print"
                ) {
                    return;
                }
                self.functions.and_then(|functions| functions.get(name).copied())
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. })
                if value.as_name_expr().is_some_and(|value| matches!(value.id.as_str(), "self" | "cls")) =>
            {
                self.class.and_then(|class| {
                    class.body.iter().filter_map(Stmt::as_function_def_stmt).find(|method| method.name.as_str() == attr.as_str())
                })
            }
            _ => None,
        };
        match resolved {
            Some(function) if function.decorator_list.is_empty() => self.pending.push(function),
            _ => self.unknown = true,
        }
    }
}

impl<'a> Visitor<'a> for Effects<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::With(_) => {
                self.unknown = true;
            }
            Stmt::For(ast::StmtFor { iter, body, orelse, target, .. }) => {
                if !matches!(iter.as_ref(), Expr::List(_) | Expr::Tuple(_) | Expr::Set(_)) {
                    self.unknown = true;
                }
                self.visit_expr(iter);
                self.visit_expr(target);
                self.visit_body(body);
                self.visit_body(orelse);
            }
            Stmt::Assert(_) => {
                self.asserts = true;
                visitor::walk_stmt(self, stmt);
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                self.visit_body(body);
                for handler in handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    let types: Vec<&str> = match handler.type_.as_deref() {
                        None => vec!["BaseException"],
                        Some(Expr::Tuple(tuple)) => tuple.elts.iter().filter_map(last_segment).collect(),
                        Some(single) => last_segment(single).into_iter().collect(),
                    };
                    if let Some(name) = &handler.name {
                        self.bound.insert(name.as_str(), types.clone());
                    }
                    self.caught.push(types);
                    self.visit_body(&handler.body);
                    self.caught.pop();
                }
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            Stmt::Raise(ast::StmtRaise { exc, cause, .. }) => {
                match exc.as_deref() {
                    // A bare `raise` re-raises what the handler around it caught.
                    None => match self.caught.last() {
                        Some(types) => self.raised.extend(types.iter().copied()),
                        None => self.unknown = true,
                    },
                    Some(exc) => {
                        let class = raised_class(exc);
                        match class {
                            Expr::Name(name) if self.bound.contains_key(name.id.as_str()) => {
                                let types = self.bound[name.id.as_str()].clone();
                                self.raised.extend(types);
                            }
                            _ => match last_segment(class).filter(|name| name.starts_with(char::is_uppercase)) {
                                Some(name) => {
                                    self.raised.insert(name);
                                }
                                None => self.unknown = true,
                            },
                        }
                        if let Expr::Call(call) = exc {
                            self.visit_arguments(&call.arguments);
                        }
                    }
                }
                if let Some(cause) = cause {
                    self.visit_expr(cause);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(call) => self.call(call),
            Expr::Await(_) | Expr::YieldFrom(_) | Expr::Generator(_) => self.unknown = true,
            Expr::ListComp(ast::ExprListComp { generators, .. }) | Expr::SetComp(ast::ExprSetComp { generators, .. })
                if unknown_iteration(generators) =>
            {
                self.unknown = true;
            }
            Expr::DictComp(ast::ExprDictComp { generators, .. }) if unknown_iteration(generators) => {
                self.unknown = true;
            }
            Expr::Subscript(_) => self.subscript = true,
            Expr::Attribute(_) => self.attribute = true,
            Expr::BinOp(_) | Expr::Compare(_) | Expr::UnaryOp(_) => self.arithmetic = true,
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// Iterating anything but a literal display calls the iterable's `__iter__`, which can raise anything.
fn unknown_iteration(generators: &[ast::Comprehension]) -> bool {
    generators.iter().any(|generator| !matches!(generator.iter, Expr::List(_) | Expr::Tuple(_) | Expr::Set(_)))
}
