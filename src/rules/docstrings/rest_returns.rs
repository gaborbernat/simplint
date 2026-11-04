use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::ReturnStatementVisitor;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{Field, FieldKind, Owner, Target, field_deletion, field_insertion, rank};
use crate::syntax::last_segment;

pub fn rest_returns(checker: &Checker) {
    for target in checker.rest_targets() {
        let Some(def) = returning_function(checker, target) else {
            continue;
        };
        // A constructor returns nothing to document.
        if def.name.as_str() == "__init__" {
            continue;
        }
        let mut visitor = ReturnStatementVisitor::default();
        visitor.visit_body(&def.body);
        let docstring = &target.docstring;
        let name = def.name.as_str();
        let (has_returns, has_yields) = (docstring.has(FieldKind::Returns), docstring.has(FieldKind::Yields));
        let parameters = target.parameters().unwrap_or_default();
        let insertion = |text: &str| {
            let added = [(rank(FieldKind::Returns, None, &parameters), text.to_owned())];
            field_insertion(checker, docstring, &parameters, &added)
        };
        if visitor.is_generator {
            if has_returns && !has_yields {
                // The `returns` of the field's name becomes `yields`, keeping its description.
                let fix = docstring.fields(&[FieldKind::Returns]).next().and_then(|field| {
                    let word = TextRange::at(field.marker?.start(), TextSize::of(field.words[0].as_str()));
                    Some(Fix::unsafe_edit(Edit::range_replacement("yields".to_owned(), word)))
                });
                checker.report_fix(
                    Rule::RestReturns,
                    docstring.range(),
                    format!("`{name}` is a generator; document what it yields with `:yields:` instead of `:returns:`"),
                    fix,
                    "Rename the field to `:yields:`",
                );
            } else if target.public && !has_yields {
                checker.report_fix(
                    Rule::RestReturns,
                    docstring.range(),
                    format!("`{name}` yields values, yet its docstring has no `:yields:` field"),
                    insertion(":yields:"),
                    "Add `:yields:`",
                );
            }
            continue;
        }
        if has_yields {
            let fields: Vec<_> = docstring.fields(&[FieldKind::Yields, FieldKind::YieldType]).collect();
            checker.report_fix(
                Rule::RestReturns,
                fields[0].marker.unwrap_or_else(|| docstring.range()),
                format!("`{name}` is not a generator, yet its docstring has a `:yields:` field"),
                field_deletion(checker, docstring, &fields),
                "Remove the yield fields",
            );
        }
        let returns_nothing = def.returns.as_deref().is_some_and(|annotation| {
            annotation.is_none_literal_expr() || matches!(last_segment(annotation), Some("NoReturn" | "Never"))
        });
        let returns_value = match def.returns.as_deref() {
            Some(_) => !returns_nothing,
            None => visitor
                .returns
                .iter()
                .any(|stmt| stmt.value.as_deref().is_some_and(|value| !value.is_none_literal_expr())),
        };
        if target.public && returns_value && !has_returns {
            checker.report_fix(
                Rule::RestReturns,
                docstring.range(),
                format!("`{name}` returns a value, yet its docstring has no `:returns:` field"),
                insertion(":returns:"),
                "Add `:returns:`",
            );
        }
        // Without an annotation, a body that is only a placeholder, as in an abstract method, says nothing of the return.
        if has_returns && !returns_value && (returns_nothing || !is_placeholder(&def.body)) {
            let fields: Vec<_> = docstring.fields(&[FieldKind::Returns, FieldKind::ReturnType]).collect();
            checker.report_fix(
                Rule::RestReturns,
                fields[0].marker.unwrap_or_else(|| docstring.range()),
                format!("`{name}` returns nothing, yet its docstring has a `:returns:` field"),
                field_deletion(checker, docstring, &fields),
                "Remove the return fields",
            );
        }
    }
}

// A body of `...`, `pass` or `raise NotImplementedError`, after the docstring.
fn is_placeholder(body: &[Stmt]) -> bool {
    body.iter().skip(1).all(|stmt| match stmt {
        Stmt::Pass(_) => true,
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.is_ellipsis_literal_expr(),
        Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) => {
            let raised = exc.as_call_expr().map_or(exc.as_ref(), |call| call.func.as_ref());
            matches!(raised, Expr::Name(name) if name.id.as_str() == "NotImplementedError")
        }
        _ => false,
    })
}

// The function whose return the docstring documents. A class's and a property's docstrings describe the class and the
// value instead, so their return fields are reported here, and moving their text into the description is for the
// author.
fn returning_function<'a>(checker: &Checker, target: &Target<'a>) -> Option<&'a ast::StmtFunctionDef> {
    let value_fields = || {
        target.docstring.fields(&[FieldKind::Returns, FieldKind::ReturnType, FieldKind::Yields, FieldKind::YieldType])
    };
    let report = |field: &Field, message: String| {
        checker.report(Rule::RestReturns, field.marker.unwrap_or_else(|| target.docstring.range()), message);
    };
    match target.owner {
        Owner::Class(class) => {
            for field in value_fields() {
                report(
                    field,
                    format!("`{}` is a class, so its docstring has no `{}` to document", class.name, field.text()),
                );
            }
            None
        }
        Owner::Function { def, .. } => {
            let is_property = def.decorator_list.iter().any(|decorator| {
                matches!(
                    last_segment(&decorator.expression),
                    Some("property" | "cached_property" | "setter" | "deleter")
                )
            });
            if !is_property {
                return Some(def);
            }
            for field in value_fields() {
                let message = format!(
                    "`{}` is a property; describe its value in the text rather than in `{}`",
                    def.name,
                    field.text()
                );
                report(field, message);
            }
            None
        }
    }
}
