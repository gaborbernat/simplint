use std::cell::OnceCell;

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::rename::rename_fix;
use crate::syntax::target_leaves;

pub fn module_variable_case(checker: &Checker) {
    let named_tuples = OnceCell::new();
    for variable in checker.state().module_variables(checker) {
        let stem = variable.name.trim_start_matches('_');
        let prefix = &variable.name[..variable.name.len() - stem.len()];
        let expected = format!("{prefix}{}", stem.to_uppercase());
        if expected == variable.name {
            continue;
        }
        let fix = rename_fix(checker, variable.name, &expected);
        if fix.is_none() && ruff_reports(checker, variable.name, &named_tuples) {
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

// Ruff's mixed-case-variable-in-global-scope (N816) reports a mixedCase name at each of its stores in the module scope,
// except in a namedtuple assignment. A name Ruff reports is left to it when this rule offers no rename for it, and kept
// here when it does.
fn ruff_reports(checker: &Checker, name: &str, named_tuples: &OnceCell<FxHashSet<TextRange>>) -> bool {
    is_mixed_case(name)
        && !checker.settings.from_ruff.ignored_names.matches(name)
        && checker.semantic().bindings(name).iter().any(|binding| {
            checker.semantic().is_module_store(binding.range)
                && !named_tuples
                    .get_or_init(|| {
                        let mut targets = FxHashSet::default();
                        collect_named_tuples(checker, checker.body(), &mut targets);
                        targets
                    })
                    .contains(&binding.range)
        })
}

// Ruff's `mixedCase`: an uppercase letter, and a lowercase first letter after one leading underscore.
fn is_mixed_case(name: &str) -> bool {
    name.contains(char::is_uppercase) && name.strip_prefix('_').unwrap_or(name).starts_with(char::is_lowercase)
}

// N816 skips names bound by a `namedtuple` or `NamedTuple` call. A function or class body binds in its own scope, so
// only the blocks of the module body are walked.
fn collect_named_tuples(checker: &Checker, body: &[Stmt], targets: &mut FxHashSet<TextRange>) {
    for stmt in body {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets: bound, value, .. }) if is_named_tuple_call(checker, value) => {
                for target in bound {
                    targets.extend(target_leaves(target).into_iter().filter_map(Expr::as_name_expr).map(Ranged::range));
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
