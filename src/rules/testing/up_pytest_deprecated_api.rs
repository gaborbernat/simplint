use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_class;
use crate::rules::testing::recognize::is_fixture;
use crate::syntax::decorator_callee;

// The `py.path` fixtures pytest keeps only in its `legacypath` plugin, and the pathlib fixture that replaces each.
const LEGACY_FIXTURES: &[(&str, &str)] =
    &[("tmpdir", "tmp_path"), ("tmpdir_factory", "tmp_path_factory"), ("testdir", "pytester")];
// The nose-style hooks pytest 8 no longer calls, and the pytest hook that runs at the same point.
const NOSE_METHODS: &[(&str, &str, &str)] =
    &[("setup", "setup_method", "before each test method"), ("teardown", "teardown_method", "after each test method")];
const NOSE_FUNCTIONS: &[(&str, &str, &str)] = &[
    ("setup", "setup_module", "before the module's tests"),
    ("teardown", "teardown_module", "after the module's tests"),
];
// The outcome functions whose `msg` pytest 8 removed in favor of `reason`.
const OUTCOMES: &[&str] = &["fail", "skip", "exit"];

pub fn up_pytest_deprecated_api(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    for stmt in checker.body() {
        match stmt {
            Stmt::FunctionDef(function) => {
                nose_hook(checker, function, NOSE_FUNCTIONS);
                function_checks(checker, function);
            }
            Stmt::ClassDef(class)
                if is_test_class(checker, class) && !crate::rules::testing::is_test_case(checker, class) =>
            {
                for method in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    nose_hook(checker, method, NOSE_METHODS);
                    function_checks(checker, method);
                }
            }
            Stmt::ClassDef(class) => {
                for method in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    function_checks(checker, method);
                }
            }
            _ => {}
        }
    }
    let mut calls = Calls { checker };
    calls.visit_body(checker.body());
}

fn nose_hook(checker: &Checker, function: &ast::StmtFunctionDef, hooks: &[(&str, &str, &str)]) {
    let Some((name, replacement, when)) = hooks.iter().find(|(name, ..)| function.name.as_str() == *name) else {
        return;
    };
    // A `setup` the module calls itself is a helper, and a decorated one, such as a fixture, is not a hook either.
    if checker.name_token_count(name) != 1
        || checker.file_name() == "conftest.py"
        || !function.decorator_list.is_empty()
    {
        return;
    }
    // pytest stopped calling it, so renaming it makes it run again: a change in behavior.
    let fix = Some(Fix::unsafe_edit(Edit::range_replacement((*replacement).to_owned(), function.name.range())));
    checker.report_fix(
        Rule::PytestDeprecatedApi,
        function.name.range(),
        format!("pytest 8 no longer calls the nose-style `{name}`; name it `{replacement}` to run it {when}"),
        fix,
        format!("Rename to `{replacement}`"),
    );
}

fn function_checks(checker: &Checker, function: &ast::StmtFunctionDef) {
    let fixture = function.decorator_list.iter().any(is_fixture);
    if fixture {
        for decorator in &function.decorator_list {
            if let Some(name) = UnqualifiedName::from_expr(decorator_callee(decorator))
                && let [.., "mark", mark] = name.segments()
                // Ruff's PT025 reports `usefixtures` on a fixture.
                && *mark != "usefixtures"
            {
                checker.report(
                    Rule::PytestDeprecatedApi,
                    decorator.range(),
                    format!(
                        "A mark on a fixture does nothing, and pytest 9 rejects it; mark the tests that request `{}`",
                        function.name
                    ),
                );
            }
        }
    }
    if !(fixture || function.name.as_str().starts_with("test")) || tests_legacypath(checker) {
        return;
    }
    for parameter in &function.parameters {
        let name = parameter.name().as_str();
        if let Some((_, replacement)) = LEGACY_FIXTURES.iter().find(|(legacy, _)| *legacy == name) {
            checker.report(
                Rule::PytestDeprecatedApi,
                parameter.name().range(),
                format!("`{name}` returns a legacy `py.path` object that only pytest's `legacypath` plugin keeps; request `{replacement}` for a `pathlib` path"),
            );
        }
    }
}

// A module that imports the `legacypath` plugin tests its fixtures, so it requests them on purpose.
fn tests_legacypath(checker: &Checker) -> bool {
    checker.body().iter().any(|stmt| match stmt {
        Stmt::Import(import) => import.names.iter().any(|alias| alias.name.as_str().contains("legacypath")),
        Stmt::ImportFrom(import) => import.module.as_ref().is_some_and(|module| module.as_str().contains("legacypath")),
        _ => false,
    })
}

struct Calls<'a, 'b> {
    checker: &'a Checker<'b>,
}

impl<'a> Visitor<'a> for Calls<'_, '_> {
    // A call in the body of `pytest.raises` checks that pytest rejects it.
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::With(with) = stmt
            && with.items.iter().any(|item| {
                item.context_expr
                    .as_call_expr()
                    .is_some_and(|call| self.checker.semantic().resolves_to(&call.func, &["pytest", "raises"]))
            })
        {
            for item in &with.items {
                self.visit_expr(&item.context_expr);
            }
            return;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.call(call);
        }
        visitor::walk_expr(self, expr);
    }
}

impl Calls<'_, '_> {
    fn call(&self, call: &ast::ExprCall) {
        let semantic = self.checker.semantic();
        if let Some(outcome) = OUTCOMES.iter().find(|outcome| semantic.resolves_to(&call.func, &["pytest", outcome]))
            && let Some(keyword) = call.arguments.find_keyword("msg")
            && call.arguments.find_keyword("reason").is_none()
        {
            let name = keyword.arg.as_ref().expect("a found keyword has a name");
            self.checker.report_fix(
                Rule::PytestDeprecatedApi,
                keyword.range(),
                format!("pytest 8 removed the `msg` argument of `pytest.{outcome}`; pass `reason`"),
                Some(Fix::safe_edit(Edit::range_replacement("reason".to_owned(), name.range()))),
                "Rename to `reason`",
            );
        }
        if semantic.resolves_to(&call.func, &["pytest", "raises"])
            && let Some(keyword) = call.arguments.find_keyword("message")
        {
            self.checker.report(
                Rule::PytestDeprecatedApi,
                keyword.range(),
                "pytest 5 removed `message` from `pytest.raises`; call `pytest.fail` after the block to fail when nothing is raised",
            );
        }
        if semantic.resolves_to(&call.func, &["pytest", "warns"])
            && let [argument] = call.arguments.args.as_ref()
            && argument.is_none_literal_expr()
            && call.arguments.keywords.is_empty()
        {
            self.checker.report(
                Rule::PytestDeprecatedApi,
                call.range(),
                "pytest 8 removed `pytest.warns(None)`; use `warnings.catch_warnings(record=True)` to record warnings",
            );
        }
    }
}
