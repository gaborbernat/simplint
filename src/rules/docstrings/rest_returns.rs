use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::ReturnStatementVisitor;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rest::{FieldKind, Owner, field_deletion, field_insertion, rank, targets};
use crate::rules::last_segment;

pub fn rest_returns(checker: &Checker) {
    for target in targets(checker) {
        let Owner::Function { def, .. } = target.owner else {
            continue;
        };
        // A property's docstring describes the value it holds, and a constructor returns nothing to document.
        let is_property = def.decorator_list.iter().any(|decorator| {
            matches!(last_segment(&decorator.expression), Some("property" | "cached_property" | "setter" | "deleter"))
        });
        if is_property || def.name.as_str() == "__init__" {
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
                let field = docstring.fields(FieldKind::Returns).next();
                let fix = field.and_then(|field| field.marker).map(|marker| {
                    Fix::unsafe_edit(Edit::range_replacement(
                        "yields".to_owned(),
                        TextRange::at(marker.start(), TextSize::of(field.map_or("", |field| field.words[0].as_str()))),
                    ))
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
            let fields: Vec<_> = docstring
                .fields
                .iter()
                .filter(|field| matches!(field.kind, FieldKind::Returns | FieldKind::ReturnType))
                .collect();
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
