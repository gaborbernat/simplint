mod edit;
mod text;

use std::cell::OnceCell;

use ruff_diagnostics::Fix;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Decorator, Expr, ExprContext, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::{ruff_ignores_function, ruff_reports_empty_docstring};
use crate::rules::public_api::Need;
use crate::syntax::docstring;
use edit::{block_levels, comment_replacement, removal};
use text::{cleaned_lines, drop_fields};

pub fn function_docstring(checker: &Checker) {
    // Code that reads docstrings off objects, such as a test asserting `func.__doc__` or a framework turning them into
    // help text or a schema, may read any of them, and which ones it reads does not show.
    if checker.project().reads_docstrings() {
        return;
    }
    let api = checker.state().public_api(checker);
    // A function or class an autodoc directive documents renders its docstring for its users, as public API does.
    let owners = checker
        .function_defs()
        .iter()
        .filter(|function| !api.covers(function.range, Need::Any))
        .map(|function| (function.range, function.body.as_slice(), function.decorator_list.as_slice(), "Function"))
        .chain(
            checker
                .class_defs()
                .iter()
                .filter(|class| !api.covers(class.range, Need::Any) && !reads_own_docstring(class))
                .map(|class| (class.range, class.body.as_slice(), class.decorator_list.as_slice(), "Class")),
        );
    let levels = OnceCell::new();
    let file_noqa = checker.state().has_file_noqa(checker);
    for (owner, owner_body, decorators, kind) in owners {
        let Some(literal) = docstring(owner_body) else {
            continue;
        };
        // A decorator receives the object and may read its docstring, as `click.command` does for help text, unless it
        // is one known to leave it unread.
        if decorators.iter().any(|decorator| !is_doc_blind(checker, &decorator.expression))
            || literal.value.to_str().lines().any(|line| line.trim_start().starts_with(">>>"))
        {
            continue;
        }
        let stmt = &owner_body[0];
        // The text as written, since an escape such as `\n` would otherwise end the comment line.
        let written: String = literal.value.iter().map(|part| checker.text(part.content_range())).collect();
        let lines = cleaned_lines(&written);
        // One short line rarely says more than the name; anything longer may state a contract the code does not show,
        // such as a tie-break rule, so its text stays as a comment.
        let restates = lines.len() <= 1 && lines.iter().all(|line| line.split_whitespace().count() <= 5);
        // The signature documents the parameters and results, so the comment leaves their field lists out.
        let documented = drop_fields(&lines);
        // Rewriting the docstring can leave a file-level `noqa` code unused, as with the lines it shortened.
        let (fix, title) = if file_noqa {
            (None, "Remove the docstring")
        } else if restates || documented.is_empty() {
            (removal(checker, stmt, owner_body).map(Fix::unsafe_edit), "Remove the docstring")
        } else {
            (
                comment_replacement(
                    checker,
                    owner,
                    owner_body,
                    &documented,
                    levels.get_or_init(|| block_levels(checker)),
                )
                .map(Fix::unsafe_edit),
                "Turn the docstring into a comment",
            )
        };
        if fix.is_none() && left_to_ruff(checker, decorators, kind == "Function", literal) {
            continue;
        }
        checker.report_fix(
            Rule::FunctionDocstring,
            stmt.range(),
            format!("{kind} docstring on internal code; let the name, signature and types document it"),
            fix,
            title,
        );
    }
}

// Ruff's D418 and D419 report the docstring of a definition this rule has no fix for. Ruff skips a definition that
// `ignore-decorators` lists, and it leaves a stub's `@overload` docstrings alone.
fn left_to_ruff(
    checker: &Checker,
    decorators: &[Decorator],
    is_function: bool,
    literal: &ast::ExprStringLiteral,
) -> bool {
    if is_function && ruff_ignores_function(checker, decorators) {
        return false;
    }
    if ruff_reports_empty_docstring(checker, literal) {
        return true;
    }
    is_function
        && checker.path.extension().is_none_or(|extension| extension != "pyi")
        && literal.as_single_part_string().is_some_and(|part| !checker.text(part.content_range()).trim().is_empty())
        && decorators.iter().any(|decorator| checker.semantic().match_typing_expr(&decorator.expression, "overload"))
}

// A class body that names `__doc__` reads its own docstring, which the compiler binds in the class namespace before the
// body runs. The headers of methods, lambdas and nested classes run in that body, as their decorators, defaults and
// bases do, while their bodies have a namespace of their own.
fn reads_own_docstring(class: &ast::StmtClassDef) -> bool {
    if docstring(&class.body).is_none() {
        return false;
    }
    let mut reads = ClassDocstringReads(false);
    reads.visit_body(&class.body);
    reads.0
}

struct ClassDocstringReads(bool);

impl<'a> Visitor<'a> for ClassDocstringReads {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, parameters, returns, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                self.visit_parameters(parameters);
                if let Some(returns) = returns {
                    self.visit_annotation(returns);
                }
            }
            Stmt::ClassDef(ast::StmtClassDef { decorator_list, arguments, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(arguments) = arguments {
                    self.visit_arguments(arguments);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, .. }) => self.0 |= id.as_str() == "__doc__",
            Expr::Lambda(ast::ExprLambda { parameters, .. }) => {
                if let Some(parameters) = parameters {
                    self.visit_parameters(parameters);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// Decorators that wrap or register the object without reading its docstring; copying it, as `functools.wraps` or
// `property` do, only hands it to `help()`. pytest shows a fixture's docstring in `pytest --fixtures`, a listing of the
// suite's own helpers like `help()`.
fn is_doc_blind(checker: &Checker, decorator: &Expr) -> bool {
    let decorator = decorator.as_call_expr().map_or(decorator, |call| &call.func);
    // `name.setter` on a property, whose name the module scope does not resolve.
    if decorator
        .as_attribute_expr()
        .is_some_and(|attribute| matches!(attribute.attr.as_str(), "setter" | "getter" | "deleter"))
    {
        return true;
    }
    let semantic = checker.semantic();
    if ["override", "overload", "final", "no_type_check"]
        .iter()
        .any(|member| semantic.match_typing_expr(decorator, member))
    {
        return true;
    }
    semantic.qualified_name(decorator).is_some_and(|name| {
        matches!(
            name.segments(),
            ["", "staticmethod" | "classmethod" | "property"]
                | ["abc", "abstractmethod"]
                | ["functools", "cache" | "lru_cache" | "cached_property" | "wraps" | "total_ordering"]
                | ["contextlib", "contextmanager" | "asynccontextmanager"]
                | ["dataclasses", "dataclass"]
                | ["enum", "unique"]
                | ["pytest", "fixture"]
                | ["pytest", "mark", ..]
        )
    })
}
