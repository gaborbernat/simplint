use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::body_without_leading_docstring;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{delete_statement, lines_with_trailing_blanks};
use crate::rules::logic::kinds::class_named;
use crate::syntax::{alias_binding, is_name};

pub fn super_forwarding_method(checker: &Checker) {
    for stmt in checker.body() {
        let Stmt::ClassDef(class) = stmt else { continue };
        // A class decorator or a metaclass may rebuild the class, such as a dataclass generating `__init__`, so
        // deleting a method changes what it gets.
        if !class.decorator_list.is_empty() || !class.keywords().is_empty() {
            continue;
        }
        let Some(parent) = parent_class(checker, class) else { continue };
        for stmt in &class.body {
            check_method(checker, &class.body, parent, stmt);
        }
    }
}

fn check_method<'a>(checker: &Checker<'a>, body: &[Stmt], parent: &'a ast::StmtClassDef, stmt: &Stmt) {
    let Stmt::FunctionDef(function) = stmt else { return };
    let name = function.name.as_str();
    // A comment above the method or in its body says why it is there, such as a workaround for a type checker, and the
    // deletion would take it away. A class that defines `__eq__` without `__hash__` gets `__hash__ = None`, so either
    // of the two may be there to keep the pair in step.
    if !function.decorator_list.is_empty()
        || function.is_async
        || matches!(name, "__eq__" | "__hash__")
        || checker.comment_above(function).is_some()
        || checker.has_comment_in(TextRange::new(function.start(), checker.source.line_end(function.end())))
        || !forwards_to_parent(function)
    {
        return;
    }
    // Once the method is deleted, the class body looks the name up in the globals, so another binding of the name
    // there, or a read of it, would change what it gets.
    let mut uses = ClassBodyUses { name, method: function.range, found: false };
    uses.visit_body(body);
    if uses.found {
        return;
    }
    // The parent's method must take the same arguments: a forwarder for an abstract method makes the class concrete,
    // and one that narrows the signature or renames a parameter rejects calls the parent's method takes.
    let Some(inherited) = inherited_method(checker, parent, name) else { return };
    if !inherited.decorator_list.is_empty()
        || inherited.is_async
        || !same_signature(&function.parameters, &inherited.parameters)
    {
        return;
    }
    // The annotations are the override's declared types, which the parent's method does not declare.
    let annotated =
        function.returns.is_some() || function.parameters.iter().any(|parameter| parameter.annotation().is_some());
    let fix = (!annotated).then(|| Fix::safe_edit(deletion(checker, stmt, body)));
    checker.report_fix(
        Rule::SuperForwardingMethod,
        function.name.range(),
        format!(
            "`{name}` only returns `super().{name}()` called with its own arguments; the parent's method is inherited"
        ),
        fix,
        "Delete the method",
    );
}

// The single base class, when the module defines it at its top level, where the method `super()` reaches can be read.
fn parent_class<'a>(checker: &Checker<'a>, class: &ast::StmtClassDef) -> Option<&'a ast::StmtClassDef> {
    let [Expr::Name(base)] = class.bases() else {
        return None;
    };
    class_named(checker, &base.id).filter(|parent| parent.range != class.range)
}

// The method `super().name` reaches from a subclass of `parent`: the first one defined up the chain of module classes
// with one base each. `None` when the chain leaves the module, or a class on it has a decorator or binds the name
// another way. Classes can name each other as bases, so the walk stops after as many classes as the module holds.
fn inherited_method<'a>(
    checker: &Checker<'a>,
    parent: &'a ast::StmtClassDef,
    name: &str,
) -> Option<&'a ast::StmtFunctionDef> {
    for class in std::iter::successors(Some(parent), |class| parent_class(checker, class)).take(checker.body().len()) {
        if !class.decorator_list.is_empty() {
            return None;
        }
        let bindings: Vec<&Stmt> = class.body.iter().filter(|stmt| binds(stmt, name)).collect();
        match bindings.as_slice() {
            [] => {}
            [Stmt::FunctionDef(function)] => return Some(function),
            _ => return None,
        }
    }
    None
}

// Both methods take the same parameters after the receiver, of the same kinds and under the same names, none with a
// default. The names of `*args` and `**kwargs` are the methods' own, so only whether each is there counts.
fn same_signature(own: &ast::Parameters, inherited: &ast::Parameters) -> bool {
    !inherited.iter_non_variadic_params().any(|parameter| parameter.default.is_some())
        && positional(own) == positional(inherited)
        && keyword_only(own) == keyword_only(inherited)
        && own.vararg.is_some() == inherited.vararg.is_some()
        && own.kwarg.is_some() == inherited.kwarg.is_some()
}

// Each positional parameter after the receiver, and whether it is positional-only.
fn positional(parameters: &ast::Parameters) -> Vec<(bool, &str)> {
    let only = parameters.posonlyargs.iter().map(|parameter| (true, parameter));
    let either = parameters.args.iter().map(|parameter| (false, parameter));
    only.chain(either).skip(1).map(|(only, parameter)| (only, parameter.name().as_str())).collect()
}

// The keyword-only parameters' names, sorted, since a caller passes them in any order.
fn keyword_only(parameters: &ast::Parameters) -> Vec<&str> {
    let mut names: Vec<&str> = parameters.kwonlyargs.iter().map(|parameter| parameter.name().as_str()).collect();
    names.sort_unstable();
    names
}

// A method that leaves other statements in its class goes with the blank lines after it. A class left with nothing gets
// a `pass` in its place.
fn deletion(checker: &Checker, stmt: &Stmt, body: &[Stmt]) -> Edit {
    if body.len() == 1 {
        return delete_statement(checker.source, stmt, body);
    }
    Edit::range_deletion(lines_with_trailing_blanks(checker.source, stmt.range()))
}

// The body, apart from a docstring, is a single `return super().name(...)` of the method's own name, which passes every
// parameter but the receiver on as it was given.
fn forwards_to_parent(function: &ast::StmtFunctionDef) -> bool {
    let [Stmt::Return(ast::StmtReturn { value: Some(value), .. })] = body_without_leading_docstring(&function.body)
    else {
        return false;
    };
    let Expr::Call(call) = value.as_ref() else {
        return false;
    };
    let Expr::Attribute(ast::ExprAttribute { value: receiver, attr, .. }) = call.func.as_ref() else {
        return false;
    };
    let Expr::Call(super_call) = receiver.as_ref() else {
        return false;
    };
    attr.as_str() == function.name.as_str()
        && is_name(&super_call.func, "super")
        && super_call.arguments.args.is_empty()
        && super_call.arguments.keywords.is_empty()
        && forwards_own_arguments(&function.parameters, call)
}

// The call passes each positional parameter after the receiver in order, then `*args`, then each keyword-only parameter
// by its name, then `**kwargs`, and nothing else. Parameters with defaults are left out, since the call would take its
// defaults from the parent's method rather than from these.
fn forwards_own_arguments(parameters: &ast::Parameters, call: &ast::ExprCall) -> bool {
    if parameters.iter_non_variadic_params().any(|parameter| parameter.default.is_some()) {
        return false;
    }
    let parameters_in_order: Vec<&ast::ParameterWithDefault> =
        parameters.posonlyargs.iter().chain(&parameters.args).collect();
    let Some((_, positional)) = parameters_in_order.split_first() else {
        return false;
    };
    let mut args = call.arguments.args.iter();
    let positional_ok =
        positional.iter().all(|parameter| args.next().is_some_and(|arg| is_name(arg, parameter.name().as_str())));
    let vararg_ok = parameters.vararg.as_deref().is_none_or(|vararg| {
        args.next().is_some_and(
            |arg| matches!(arg, Expr::Starred(ast::ExprStarred { value, .. }) if is_name(value, vararg.name.as_str())),
        )
    });
    let mut keywords = call.arguments.keywords.iter();
    let keyword_only_ok = parameters.kwonlyargs.iter().all(|parameter| {
        keywords.next().is_some_and(|keyword| {
            keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == parameter.name().as_str())
                && is_name(&keyword.value, parameter.name().as_str())
        })
    });
    let kwarg_ok = parameters.kwarg.as_deref().is_none_or(|kwarg| {
        keywords.next().is_some_and(|keyword| keyword.arg.is_none() && is_name(&keyword.value, kwarg.name.as_str()))
    });
    positional_ok && vararg_ok && args.next().is_none() && keyword_only_ok && kwarg_ok && keywords.next().is_none()
}

// Whether a statement of a class body binds `name` on its own, so that it is the one the class keeps.
fn binds(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::FunctionDef(function) => function.name.as_str() == name,
        Stmt::ClassDef(class) => class.name.as_str() == name,
        Stmt::Assign(assign) => assign.targets.iter().any(|target| is_name(target, name)),
        Stmt::AnnAssign(assign) => is_name(&assign.target, name),
        _ => false,
    }
}

// Whether the class body binds or reads `name` outside the body of the `method` being deleted. The bodies of other
// methods and of nested scopes run in their own scope, which does not see the class body's names, so they are left out.
struct ClassBodyUses<'n> {
    name: &'n str,
    method: TextRange,
    found: bool,
}

impl<'a> Visitor<'a> for ClassBodyUses<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                if function.range == self.method {
                    return;
                }
                self.found |= function.name.as_str() == self.name;
                for decorator in &function.decorator_list {
                    self.visit_decorator(decorator);
                }
                self.visit_parameters(&function.parameters);
                if let Some(returns) = &function.returns {
                    self.visit_annotation(returns);
                }
            }
            Stmt::ClassDef(class) => {
                self.found |= class.name.as_str() == self.name;
                for decorator in &class.decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(arguments) = &class.arguments {
                    self.visit_arguments(arguments);
                }
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.found |= names.iter().any(|alias| alias_binding(alias).split('.').next() == Some(self.name));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => self.found |= name.id.as_str() == self.name,
            Expr::Lambda(lambda) => {
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameters(parameters);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = handler;
        self.found |= name.as_ref().is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        let captured = match pattern {
            ast::Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | ast::Pattern::MatchStar(ast::PatternMatchStar { name, .. })
            | ast::Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => name.as_ref(),
            _ => None,
        };
        self.found |= captured.is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_pattern(self, pattern);
    }
}
