use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt, TypeParam};
use ruff_text_size::TextRange;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::naming::{current, except_name, parameters, pattern_name};

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            check(walker, current(walker), name.as_str(), name.range);
        }
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            for asname in names.iter().filter_map(|alias| alias.asname.as_ref()) {
                check(walker, current(walker), asname.as_str(), asname.range);
            }
        }
        _ => {}
    }
}

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, range, .. }) = expr {
        check(walker, current(walker), id.as_str(), *range);
    }
}

pub fn scope<'a>(walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
    parameters(walker, scope, |_| false, check);
}

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    except_name(walker, handler, check);
}

pub fn pattern<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
    pattern_name(walker, pattern, check);
}

pub fn type_param<'a>(walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
    let name = type_param.name();
    check(walker, current(walker), name.as_str(), name.range);
}

// A comprehension keeps every name it binds next to the iterable it reads, so none needs a long name.
fn check<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange) {
    if !walker.in_comprehension()
        && name.chars().count() == 1
        && name != "_"
        && walker.checker.state().reported_names().first_in_scope(scope, name)
    {
        walker.checker.report(
            Rule::SingleLetterName,
            range,
            format!("Single-letter name `{name}`; only comprehensions may use one"),
        );
    }
}
