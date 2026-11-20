mod edit;
mod text;

use std::cell::OnceCell;

use ruff_diagnostics::Fix;
use ruff_python_ast::helpers::map_callable;
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
    // A dynamic docstring lookup can read any object's documentation in the project.
    if checker.project().reads_docstrings() {
        return;
    }
    let api = checker.state().public_api(checker);
    // Autodoc can expose definitions even in an application or a private module.
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
        // For example, Click uses the command function's docstring as help text.
        if decorators.iter().any(|decorator| !is_doc_blind(checker, &decorator.expression))
            || literal.value.to_str().lines().any(|line| line.trim_start().starts_with(">>>"))
        {
            continue;
        }
        let stmt = &owner_body[0];
        // Preserve escapes such as `\n` so they cannot introduce an uncommented line.
        let written: String = literal.value.iter().map(|part| checker.text(part.content_range())).collect();
        let lines = cleaned_lines(&written);
        // Preserve longer explanations, which may document behavior such as a tie-break rule.
        let restates = lines.len() <= 1 && lines.iter().all(|line| line.split_whitespace().count() <= 5);
        let documented = drop_fields(&lines);
        // The docstring may be the only reason a file-level suppression is needed.
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

// A class body can read its own docstring by name. Check expressions in nested definition headers without entering
// their bodies.
fn reads_own_docstring(class: &ast::StmtClassDef) -> bool {
    if docstring(&class.body).is_none() {
        return false;
    }
    let mut reads = ClassDocstringReads { reads_docstring: false };
    reads.visit_body(&class.body);
    reads.reads_docstring
}

struct ClassDocstringReads {
    reads_docstring: bool,
}

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
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, .. }) => {
                self.reads_docstring |= id.as_str() == "__doc__";
            }
            Expr::Lambda(ast::ExprLambda { parameters, .. }) => {
                if let Some(parameters) = parameters {
                    self.visit_parameters(parameters);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// These decorators are allowed on internal definitions even when they copy or display the docstring. Removing it still
// makes the fix unsafe.
fn is_doc_blind(checker: &Checker, decorator: &Expr) -> bool {
    let decorator = map_callable(decorator);
    // A property accessor in a class body may have no module-level binding to resolve.
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

// Suppress a duplicate report only where Ruff's D418 or D419 would apply.
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
