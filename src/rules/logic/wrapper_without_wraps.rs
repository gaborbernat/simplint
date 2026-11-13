use ruff_diagnostics::{Applicability, Edit};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::{ensure_import, with_import};

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(decorator) = stmt else {
        return;
    };
    let parameters: Vec<&str> = decorator.parameters.iter_non_variadic_params().map(|p| p.name().as_str()).collect();
    if parameters.is_empty() {
        return;
    }
    let checker = walker.checker;
    for wrapper in decorator.body.iter().filter_map(Stmt::as_function_def_stmt) {
        let name = wrapper.name.as_str();
        // A wrapper forwards whatever its caller passes; a closure with a fixed signature that calls the parameter is a
        // callback factory, which keeps a name of its own.
        let forwards = wrapper.parameters.vararg.is_some() || wrapper.parameters.kwarg.is_some();
        if !forwards || !wrapper.decorator_list.is_empty() || parameters.contains(&name) {
            continue;
        }
        let Some(wrapped) = called_parameter(wrapper, &parameters) else {
            continue;
        };
        let mut facts = DecoratorFacts { wrapper: name, returned: false, updated: false };
        facts.visit_body(&decorator.body);
        if !facts.returned || facts.updated {
            continue;
        }
        checker.report_fix(
            Rule::WrapperWithoutWraps,
            wrapper.name.range(),
            format!(
                "`{name}` replaces the decorated `{wrapped}` without `functools.wraps`, so it loses `{wrapped}`'s name, \
                 docstring and signature"
            ),
            wraps_fix(checker, wrapper, wrapped),
            format!("Decorate `{name}` with `@functools.wraps({wrapped})`"),
        );
    }
}

// The first parameter of the decorator that the wrapper calls with its own `*args` or `**kwargs` and nothing but its own
// parameters, as `func(*args, **kwargs)` does. A call that adds or reshapes arguments makes an adapter with a signature
// of its own, which `functools.wraps` would then misreport.
fn called_parameter<'a>(wrapper: &ast::StmtFunctionDef, parameters: &[&'a str]) -> Option<&'a str> {
    let mut calls = Calls { parameters, wrapper: &wrapper.parameters, called: None };
    calls.visit_body(&wrapper.body);
    calls.called
}

struct Calls<'p, 'a, 'w> {
    parameters: &'p [&'a str],
    wrapper: &'w ast::Parameters,
    called: Option<&'a str>,
}

impl<'a> Visitor<'a> for Calls<'_, '_, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.called.is_none()
            && let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr
            && let Expr::Name(name) = func.as_ref()
            && self.forwards(arguments)
        {
            self.called = self.parameters.iter().copied().find(|parameter| *parameter == name.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}

impl Calls<'_, '_, '_> {
    fn forwards(&self, arguments: &ast::Arguments) -> bool {
        let own = |expr: &Expr| expr.as_name_expr().is_some_and(|name| self.wrapper.includes(name.id.as_str()));
        let variadic = |expr: &Expr, parameter: Option<&ast::Parameter>| {
            expr.as_name_expr().zip(parameter).is_some_and(|(name, parameter)| name.id == parameter.name.id)
        };
        let vararg = self.wrapper.vararg.as_deref();
        let kwarg = self.wrapper.kwarg.as_deref();
        let spreads = arguments
            .args
            .iter()
            .any(|arg| arg.as_starred_expr().is_some_and(|starred| variadic(&starred.value, vararg)))
            || arguments.keywords.iter().any(|keyword| keyword.arg.is_none() && variadic(&keyword.value, kwarg));
        spreads
            && arguments.args.iter().all(|arg| own(arg.as_starred_expr().map_or(arg, |starred| &starred.value)))
            && arguments.keywords.iter().all(|keyword| own(&keyword.value))
    }
}

// Whether the decorator returns the wrapper, and whether it copies the wrapped function's metadata some other way, by
// `functools.update_wrapper` or by setting a dunder on the wrapper.
struct DecoratorFacts<'a> {
    wrapper: &'a str,
    returned: bool,
    updated: bool,
}

impl<'a> Visitor<'a> for DecoratorFacts<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Return(ast::StmtReturn { value: Some(value), .. }) => {
                self.returned |= value.as_name_expr().is_some_and(|name| name.id.as_str() == self.wrapper);
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(ast::ExprCall { func, .. })
                if matches!(func.as_ref(), Expr::Name(name) if name.id.as_str() == "update_wrapper")
                    || matches!(func.as_ref(), Expr::Attribute(attribute) if attribute.attr.as_str() == "update_wrapper") =>
            {
                self.updated = true;
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store, .. })
                if attr.as_str().starts_with("__")
                    && value.as_name_expr().is_some_and(|name| name.id.as_str() == self.wrapper) =>
            {
                self.updated = true;
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// The decorator goes on its own line above the wrapper, at the wrapper's indentation, spelled the way the file already
// imports `wraps`.
fn wraps_fix(checker: &Checker, wrapper: &ast::StmtFunctionDef, wrapped: &str) -> Option<ruff_diagnostics::Fix> {
    let source = checker.source;
    let line_start = source.line_start(wrapper.start());
    let indentation = &source[TextRange::new(line_start, wrapper.start())];
    if !indentation.chars().all(char::is_whitespace) {
        return None;
    }
    let (spelling, import) = if checker.state().top_level_imports(checker).contains(&("functools", Some("wraps"))) {
        ("wraps", ensure_import(checker, "functools", Some("wraps"), wrapper.range())?)
    } else {
        ("functools.wraps", ensure_import(checker, "functools", None, wrapper.range())?)
    };
    let eol = checker.line_ending();
    let edit = Edit::insertion(format!("{indentation}@{spelling}({wrapped}){eol}"), line_start);
    Some(with_import(edit, import, Applicability::Unsafe))
}
