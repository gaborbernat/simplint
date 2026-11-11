use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::public_api::Need;
use crate::syntax::ExprCollector;
use crate::syntax::module_statements;

pub fn indistinct_error_class(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let project = checker.project();
    let api = checker.state().public_api(checker);
    for stmt in module_statements(checker.body()) {
        let Stmt::ClassDef(class) = stmt else {
            continue;
        };
        let name = class.name.as_str();
        if !project.is_uncaught_exception(name) || api.is_public(stmt.range(), Need::Any) || has_state(class) {
            continue;
        }
        checker.report(
            Rule::IndistinctErrorClass,
            class.name.range(),
            format!("No handler catches `{name}` by name; raise a built-in exception such as `ValueError` instead"),
        );
    }
}

// Whether the class stores attributes or takes arguments in `__init__`: a handler may read them off the exception, and
// a built-in exception could not carry them. Such a class is kept rather than following each attribute read.
fn has_state(class: &ast::StmtClassDef) -> bool {
    let mut stores = ExprCollector::new(|expr: &Expr| {
        let attribute = expr.as_attribute_expr()?;
        (matches!(attribute.ctx, ExprContext::Store) && attribute.value.as_name_expr()?.id.as_str() == "self")
            .then_some(())
    });
    stores.visit_body(&class.body);
    !stores.found.is_empty()
        || class.body.iter().any(|stmt| {
            matches!(stmt, Stmt::Assign(_) | Stmt::AnnAssign(_) | Stmt::AugAssign(_))
                || stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__init__")
        })
}
