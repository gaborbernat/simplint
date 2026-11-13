use ruff_python_ast::helpers::map_callable;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_expression;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::{Field, FieldKind, Owner, Target, field_deletion};
use crate::syntax::{ReturnVisitor, has_decorator, last_segment};

pub fn rst_returns(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let Some(def) = returning_function(checker, target) else {
            continue;
        };
        // A constructor returns nothing to document.
        if def.name.as_str() == "__init__" {
            continue;
        }
        let mut visitor = ReturnVisitor::default();
        visitor.visit_body(&def.body);
        if visitor.is_generator {
            generator_fields(checker, target, def.name.as_str());
        } else {
            return_fields(checker, target, def, &visitor);
        }
    }
}

// The function whose return the docstring documents. A class's and a property's docstrings describe the class and the
// value instead, so their return fields are reported here, and moving their text into the description is for the
// author.
fn returning_function<'a>(checker: &Checker, target: &Target<'a>) -> Option<&'a ast::StmtFunctionDef> {
    let value_fields = || {
        target.docstring.fields(&[FieldKind::Returns, FieldKind::ReturnType, FieldKind::Yields, FieldKind::YieldType])
    };
    let report = |field: &Field, message: String| {
        checker.report(Rule::RstReturns, field.range_in(&target.docstring), message);
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
            if !has_decorator(&def.decorator_list, &["property", "cached_property", "setter", "deleter"]) {
                return Some(def);
            }
            for field in value_fields() {
                report(
                    field,
                    format!(
                        "`{}` is a property; describe its value in the text rather than in `{}`",
                        def.name,
                        field.text()
                    ),
                );
            }
            None
        }
    }
}

// A generator documents what it yields, so its `:returns:` field is `:yields:` and the field is missing when absent.
fn generator_fields(checker: &Checker, target: &Target, name: &str) {
    let docstring = &target.docstring;
    let (has_returns, has_yields) = (docstring.has(FieldKind::Returns), docstring.has(FieldKind::Yields));
    if has_returns && !has_yields {
        checker.report_fix(
            Rule::RstReturns,
            docstring.range(),
            format!("`{name}` is a generator; document what it yields with `:yields:` instead of `:returns:`"),
            docstring.fields(&[FieldKind::Returns]).next().and_then(|field| field.rename("yields")),
            "Rename the field to `:yields:`",
        );
    } else if target.public(checker) && !has_yields {
        checker.report(
            Rule::RstReturns,
            docstring.range(),
            format!("`{name}` yields values, yet its docstring has no `:yields:` field"),
        );
    }
}

// A function that is not a generator documents its return value and not its yields.
fn return_fields(checker: &Checker, target: &Target, def: &ast::StmtFunctionDef, visitor: &ReturnVisitor) {
    let docstring = &target.docstring;
    let name = def.name.as_str();
    let (has_returns, has_yields) = (docstring.has(FieldKind::Returns), docstring.has(FieldKind::Yields));
    let placeholder = is_placeholder(&def.body);
    if has_yields && !placeholder {
        let fields: Vec<_> = docstring.fields(&[FieldKind::Yields, FieldKind::YieldType]).collect();
        checker.report_fix(
            Rule::RstReturns,
            fields[0].range_in(docstring),
            format!("`{name}` is not a generator, yet its docstring has a `:yields:` field"),
            field_deletion(checker, docstring, &fields),
            "Remove the yield fields",
        );
    }
    let returns_nothing = def.returns.as_deref().is_some_and(is_nothing_annotation);
    let returns_value = match def.returns.as_deref() {
        Some(_) => !returns_nothing,
        None => {
            visitor.returns.iter().any(|stmt| stmt.value.as_deref().is_some_and(|value| !value.is_none_literal_expr()))
        }
    };
    if target.public(checker) && returns_value && !has_returns && !placeholder {
        checker.report(
            Rule::RstReturns,
            docstring.range(),
            format!("`{name}` returns a value, yet its docstring has no `:returns:` field"),
        );
    }
    // Without an annotation, a body that is only a placeholder, as in an abstract method, says nothing of the return.
    if has_returns && !returns_value && (returns_nothing || !placeholder) {
        let fields: Vec<_> = docstring.fields(&[FieldKind::Returns, FieldKind::ReturnType]).collect();
        checker.report_fix(
            Rule::RstReturns,
            fields[0].range_in(docstring),
            format!("`{name}` returns nothing, yet its docstring has a `:returns:` field"),
            field_deletion(checker, docstring, &fields),
            "Remove the return fields",
        );
    }
}

fn is_placeholder(body: &[Stmt]) -> bool {
    body.iter().skip(1).all(|stmt| match stmt {
        Stmt::Pass(_) => true,
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.is_ellipsis_literal_expr(),
        Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) => {
            matches!(map_callable(exc), Expr::Name(name) if name.id.as_str() == "NotImplementedError")
        }
        _ => false,
    })
}

// A quoted annotation, as in `-> "None"`, returns what its unquoted form does.
fn is_nothing_annotation(annotation: &Expr) -> bool {
    if let Expr::StringLiteral(literal) = annotation {
        return parse_expression(literal.value.to_str()).is_ok_and(|parsed| is_nothing_annotation(parsed.expr()));
    }
    annotation.is_none_literal_expr() || matches!(last_segment(annotation), Some("NoReturn" | "Never"))
}
