use std::cell::RefCell;

use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt, TypeParam};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::last_segment;

// The names reported in each def, class or lambda, by its range, the module's an empty one: the rule reports a name
// once per scope.
#[derive(Default)]
pub(crate) struct ReportedNames<'a>(RefCell<FxHashSet<(TextRange, &'a str)>>);

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
    let parameters = match scope {
        // An override must keep the parameter names of the method it replaces.
        Scope::Function(function)
            if !function
                .decorator_list
                .iter()
                .any(|decorator| last_segment(&decorator.expression) == Some("override")) =>
        {
            &*function.parameters
        }
        Scope::Lambda(ast::ExprLambda { parameters: Some(parameters), .. }) => parameters,
        _ => return,
    };
    for parameter in parameters {
        check(walker, scope.range(), parameter.name().as_str(), parameter.name().range);
    }
}

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), .. }) = handler else {
        return;
    };
    check(walker, current(walker), name.as_str(), name.range);
}

pub fn pattern<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
    if let Pattern::MatchAs(ast::PatternMatchAs { name: Some(name), .. })
    | Pattern::MatchStar(ast::PatternMatchStar { name: Some(name), .. }) = pattern
    {
        check(walker, current(walker), name.as_str(), name.range);
    }
}

pub fn type_param<'a>(walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
    let name = type_param.name();
    check(walker, current(walker), name.as_str(), name.range);
}

fn current(walker: &Walker) -> TextRange {
    walker.scopes().last().map_or_else(TextRange::default, Ranged::range)
}

// A comprehension keeps every name it binds next to the iterable it reads, so none needs a long name.
fn check<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange) {
    if !walker.in_for_target()
        && !walker.in_comprehension()
        && name.chars().count() == 1
        && name != "_"
        && walker.checker.reported_names().0.borrow_mut().insert((scope, name))
    {
        walker.checker.report(
            Rule::SingleLetterName,
            range,
            format!("Single-letter name `{name}`; only loop targets and comprehensions may use one"),
        );
    }
}
