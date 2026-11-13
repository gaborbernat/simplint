use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt, TypeParam};
use ruff_text_size::TextRange;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::naming::{NameSite, current, except_name, parameters, pattern_name};
use crate::rules::typing::type_parameter_assignment;
use crate::syntax::import_aliases;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            check(walker, current(walker), NameSite { text: name.as_str(), range: name.range });
        }
        Stmt::Import(_) | Stmt::ImportFrom(_) => {
            for asname in import_aliases(stmt).iter().filter_map(|alias| alias.asname.as_ref()) {
                simplint_only(walker, current(walker), NameSite { text: asname.as_str(), range: asname.range });
            }
        }
        _ => {}
    }
}

// Ruff's E741, E742 and E743 flag `l`, `I` and `O` on the variables, classes and functions of their kind, so those
// letters are left to them. No Ruff rule checks imports, patterns or type parameters, which keep them.
fn check<'a>(walker: &Walker<'_, 'a>, scope: TextRange, site: NameSite<'a>) {
    report(walker, scope, site, !matches!(site.text, "l" | "I" | "O"));
}

// A comprehension keeps every name it binds next to the iterable it reads, so none needs a long name. A name that
// Ruff covers still counts as the scope's first binding, so a later import of it is not reported in its place.
fn report<'a>(walker: &Walker<'_, 'a>, scope: TextRange, site: NameSite<'a>, reportable: bool) {
    if !walker.in_comprehension()
        && site.text.chars().count() == 1
        && site.text != "_"
        && walker.checker.state().reported_names().first_in_scope(scope, site.text)
        && reportable
    {
        walker.checker.report(
            Rule::SingleLetterName,
            site.range,
            format!("Single-letter name `{}`; use a descriptive name", site.text),
        );
    }
}

fn simplint_only<'a>(walker: &Walker<'_, 'a>, scope: TextRange, site: NameSite<'a>) {
    report(walker, scope, site, true);
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
        check(walker, current(walker), NameSite { text: id.as_str(), range: *range });
    }
}

// A lambda is one expression, so like a comprehension it keeps its parameters next to every use.
pub fn scope<'a>(walker: &mut Walker<'_, 'a>, scope: &Scope<'a>) {
    if !matches!(scope, Scope::Lambda(_)) {
        parameters(walker, scope, |_| false, check);
    }
}

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    except_name(walker, handler, check);
}

pub fn pattern<'a>(walker: &mut Walker<'_, 'a>, pattern: &'a Pattern) {
    pattern_name(walker, pattern, simplint_only);
}

pub fn type_param<'a>(walker: &mut Walker<'_, 'a>, type_param: &'a TypeParam) {
    let name = type_param.name();
    if !name.as_str().chars().all(char::is_uppercase) {
        simplint_only(walker, current(walker), NameSite { text: name.as_str(), range: name.range });
    }
}
