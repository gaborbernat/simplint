use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt, TypeParam};
use ruff_text_size::TextRange;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::naming::{current, except_name, parameters, pattern_name};
use crate::rules::typing::type_parameter_assignment;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => {
            check(walker, current(walker), name.as_str(), name.range, RuffRule::AmbiguousFunctionName);
        }
        Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            check(walker, current(walker), name.as_str(), name.range, RuffRule::AmbiguousClassName);
        }
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            for asname in names.iter().filter_map(|alias| alias.asname.as_ref()) {
                simplint_only(walker, current(walker), asname.as_str(), asname.range);
            }
        }
        _ => {}
    }
}

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, range, .. }) = expr {
        if id.as_str().chars().count() == 1
            && id.as_str().chars().all(char::is_uppercase)
            && walker
                .statement()
                .and_then(|stmt| type_parameter_assignment(walker.checker, stmt))
                .is_some_and(|target| target.range == *range)
        {
            return;
        }
        variable(walker, current(walker), id.as_str(), *range);
    }
}

pub fn scope<'a>(walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
    parameters(walker, scope, |_| false, variable);
}

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    except_name(walker, handler, variable);
}

pub fn pattern<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
    pattern_name(walker, pattern, simplint_only);
}

pub fn type_param<'a>(walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
    let name = type_param.name();
    if !name.as_str().chars().all(char::is_uppercase) {
        simplint_only(walker, current(walker), name.as_str(), name.range);
    }
}

// Ruff's E741, E742 and E743 flag `l`, `I` and `O` on the bindings of their kind, except imports, patterns and type
// parameters. Those three letters are left to the Ruff rule of the binding's kind while it runs.
fn check<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange, ruff: RuffRule) {
    let left_to_ruff = matches!(name, "l" | "I" | "O") && walker.checker.settings.from_ruff.ruff_runs(ruff);
    report(walker, scope, name, range, !left_to_ruff);
}

fn variable<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange) {
    check(walker, scope, name, range, RuffRule::AmbiguousVariableName);
}

// Imports, patterns and type parameters, which no Ruff rule checks for these letters.
fn simplint_only<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange) {
    report(walker, scope, name, range, true);
}

// A comprehension keeps every name it binds next to the iterable it reads, so none needs a long name. A name that
// Ruff covers still counts as the scope's first binding, so a later import of it is not reported in its place.
fn report<'a>(walker: &Walker<'_, 'a>, scope: TextRange, name: &'a str, range: TextRange, reportable: bool) {
    if !walker.in_comprehension()
        && name.chars().count() == 1
        && name != "_"
        && walker.checker.state().reported_names().first_in_scope(scope, name)
        && reportable
    {
        walker.checker.report(
            Rule::SingleLetterName,
            range,
            format!("Single-letter name `{name}`; use a descriptive name"),
        );
    }
}
