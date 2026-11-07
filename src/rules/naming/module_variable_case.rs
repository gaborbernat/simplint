use std::cell::OnceCell;

use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::module_variable::{ModuleVariable, module_variables};
use crate::rules::rename::rename_fix;

pub fn module_variable_case(checker: &Checker) {
    if checker.in_nested_project() {
        return;
    }
    let named_tuples = OnceCell::new();
    for variable in module_variables(checker) {
        let stem = variable.name.trim_start_matches('_');
        let prefix = &variable.name[..variable.name.len() - stem.len()];
        let expected = format!("{prefix}{}", stem.to_ascii_uppercase());
        if expected == variable.name {
            continue;
        }
        let fix = rename_fix(checker, variable.name, &expected);
        if fix.is_none()
            && checker.settings.from_ruff.ruff_runs(RuffRule::MixedCaseVariableInGlobalScope)
            && ruff_reports(checker, &variable, &named_tuples)
        {
            continue;
        }
        checker.report_fix(
            Rule::ModuleVariableCase,
            variable.range,
            format!("Module-scope variable `{}` should be named `{expected}`", variable.name),
            fix,
            format!("Rename `{}` to `{expected}`", variable.name),
        );
    }
}

// Ruff's mixed-case-variable-in-global-scope (N816) reports a mixedCase module name at its binding and never offers a
// fix. Where this rule has no fix and N816 does not run, this rule reports the name itself, so the checks N816 makes
// are repeated here to decide whether N816 reports it.
fn ruff_reports(checker: &Checker, variable: &ModuleVariable, named_tuples: &OnceCell<FxHashSet<TextRange>>) -> bool {
    let first = checker.semantic().bindings(variable.name).first();
    is_mixed_case(variable.name)
        && first.is_some_and(|binding| binding.module_scope)
        && !checker.settings.from_ruff.ignored_names.matches(variable.name)
        && !named_tuples
            .get_or_init(|| {
                let mut targets = FxHashSet::default();
                collect_named_tuples(checker, checker.body(), &mut targets);
                targets
            })
            .contains(&variable.range)
}

// Ruff's `mixedCase`: an uppercase letter, and a lowercase first letter after one leading underscore.
fn is_mixed_case(name: &str) -> bool {
    name.contains(char::is_uppercase) && name.strip_prefix('_').unwrap_or(name).starts_with(char::is_lowercase)
}

// The names bound by an assignment that calls `namedtuple` or `NamedTuple`, which N816 skips. The assignment may sit in
// a block of the module body, but not in a function or class body, which binds its names in its own scope.
fn collect_named_tuples(checker: &Checker, body: &[Stmt], targets: &mut FxHashSet<TextRange>) {
    for stmt in body {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets: bound, value, .. }) if is_named_tuple_call(checker, value) => {
                for target in bound {
                    store_names(target, targets);
                }
            }
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                collect_named_tuples(checker, body, targets);
                for clause in elif_else_clauses {
                    collect_named_tuples(checker, &clause.body, targets);
                }
            }
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                collect_named_tuples(checker, body, targets);
                collect_named_tuples(checker, orelse, targets);
            }
            Stmt::With(ast::StmtWith { body, .. }) => collect_named_tuples(checker, body, targets),
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                collect_named_tuples(checker, body, targets);
                for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                    collect_named_tuples(checker, &handler.body, targets);
                }
                collect_named_tuples(checker, orelse, targets);
                collect_named_tuples(checker, finalbody, targets);
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                for case in cases {
                    collect_named_tuples(checker, &case.body, targets);
                }
            }
            _ => {}
        }
    }
}

fn is_named_tuple_call(checker: &Checker, value: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = value else {
        return false;
    };
    let semantic = checker.semantic();
    semantic.resolves_to(func, &["collections", "namedtuple"]) || semantic.match_typing_expr(func, "NamedTuple")
}

// The names an assignment target binds, where a tuple, list or starred target binds the names inside it.
fn store_names(target: &Expr, names: &mut FxHashSet<TextRange>) {
    match target {
        Expr::Name(name) => {
            names.insert(name.range());
        }
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            for elt in elts {
                store_names(elt, names);
            }
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => store_names(value, names),
        _ => {}
    }
}
