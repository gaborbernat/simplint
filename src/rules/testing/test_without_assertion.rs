use rustc_hash::FxHashSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::collected_tests;
use crate::syntax::{decorator_callee, last_segment};

// Prefixes of the helpers that check a result, such as `check_output`, `verify_signature` or Playwright's `expect`.
const CHECKING_PREFIXES: &[&str] = &["check", "verify", "expect", "validate", "ensure", "compare", "fail"];
// pytest's own calls that make a test fail, skip or expect an error.
const PYTEST_CHECKS: &[&str] = &["deprecated_call", "fail", "skip", "xfail", "importorskip"];
// The fixtures whose tests measure rather than check, such as pytest-benchmark's.
const MEASURING_FIXTURES: &[&str] = &["benchmark", "aio_benchmark"];
// The `pytester` line matchers, which fail the test when the output does not match.
const MATCHERS: &[&str] = &[
    "fnmatch_lines",
    "fnmatch_lines_random",
    "re_match_lines",
    "re_match_lines_random",
    "no_fnmatch_line",
    "no_re_match_line",
];
// Words in a test's name, docstring or comments that say it only checks the code runs without raising, crashing or
// warning, as a smoke or regression test does.
const RUNS_CLEANLY: &[&str] =
    &["smoke", "sanity", "crash", "segfault", "raise", "error", "hang", "leak", "works", "import"];

pub fn test_without_assertion(checker: &Checker) {
    // pytest never runs the tests of a module it does not collect, such as a type checker's input.
    if !checker.is_test_module() || !checker.settings.project.pytest.collects(checker.file_name()) {
        return;
    }
    let support = support_names(checker.body());
    for test in collected_tests(checker) {
        if is_skipped(test)
            || test.parameters.iter().any(|parameter| MEASURING_FIXTURES.contains(&parameter.name().as_str()))
            || only_runs(checker, test)
            || test
                .decorator_list
                .iter()
                .any(|decorator| decorator.expression.as_call_expr().is_some_and(escalates_warnings))
        {
            continue;
        }
        let parameters: FxHashSet<&str> = test.parameters.iter().map(|parameter| parameter.name().as_str()).collect();
        let mut finder = Checks {
            checker,
            support: &support,
            parameters,
            class: class_of(checker, test),
            seen: FxHashSet::default(),
            checks: false,
        };
        finder.visit_body(&test.body);
        if !finder.checks {
            checker.report(
                Rule::TestWithoutAssertion,
                test.name.range(),
                "This test checks nothing, so it passes for any code that does not raise; assert on the result",
            );
        }
    }
}

// Tests pytest always skips or expects to fail are placeholders, not checks that went missing.
fn is_skipped(test: &ast::StmtFunctionDef) -> bool {
    test.decorator_list.iter().any(|decorator| {
        UnqualifiedName::from_expr(decorator_callee(decorator)).is_some_and(|name| {
            matches!(name.segments(), [.., "mark", "skip" | "xfail"] | ["unittest", "skip"] | ["skip"])
        })
    })
}

fn only_runs(checker: &Checker, test: &ast::StmtFunctionDef) -> bool {
    let docstring = crate::syntax::docstring(&test.body).map(|literal| literal.value.to_str());
    let comments =
        checker.comment_ranges().comments_in_range(test.range()).iter().map(|comment| checker.text(*comment));
    std::iter::once(test.name.as_str()).chain(docstring).chain(comments).any(|text| {
        let lowered = text.to_ascii_lowercase();
        RUNS_CLEANLY.iter().any(|word| lowered.contains(word))
    })
}

fn class_of<'a>(checker: &Checker<'a>, test: &ast::StmtFunctionDef) -> Option<&'a ast::StmtClassDef> {
    checker.body().iter().filter_map(Stmt::as_class_def_stmt).find(|class| class.range().contains_range(test.range()))
}

// The names the module imports from its own test package or from a testing module, whose helpers may assert: a
// relative import, or a module path with a segment such as `tests`, `_testutils`, `conftest` or `test_*`.
fn support_names(body: &[Stmt]) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for stmt in body {
        match stmt {
            Stmt::ImportFrom(import) => {
                let module = import.module.as_ref().map_or("", |module| module.as_str());
                if import.level > 0 || is_testing_path(module) {
                    names
                        .extend(import.names.iter().map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()));
                }
            }
            Stmt::Import(import) => {
                for alias in &import.names {
                    if is_testing_path(alias.name.as_str()) {
                        names.insert(crate::syntax::alias_binding(alias));
                    }
                }
            }
            _ => {}
        }
    }
    names
}

fn is_testing_path(module: &str) -> bool {
    module.split('.').any(|segment| segment != "pytest" && segment.contains("test"))
}

struct Checks<'a, 'b> {
    checker: &'a Checker<'b>,
    support: &'a FxHashSet<&'b str>,
    parameters: FxHashSet<&'b str>,
    class: Option<&'b ast::StmtClassDef>,
    // The local helpers already followed, so recursion ends.
    seen: FxHashSet<TextRange>,
    checks: bool,
}

impl<'b> Visitor<'b> for Checks<'_, 'b> {
    fn visit_stmt(&mut self, stmt: &'b Stmt) {
        if self.checks {
            return;
        }
        if matches!(stmt, Stmt::Assert(_) | Stmt::Raise(_)) {
            self.checks = true;
            return;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'b Expr) {
        if self.checks {
            return;
        }
        if let Expr::Call(call) = expr
            && self.call_checks(call)
        {
            self.checks = true;
            return;
        }
        visitor::walk_expr(self, expr);
    }
}

impl<'b> Checks<'_, 'b> {
    fn call_checks(&mut self, call: &'b ast::ExprCall) -> bool {
        let func = &*call.func;
        let called = match func {
            Expr::Name(name) => Some(name.id.as_str()),
            Expr::Attribute(attribute) => Some(attribute.attr.as_str()),
            _ => None,
        };
        if let Some(name) = called {
            let lowered = name.to_ascii_lowercase();
            let lowered = lowered.trim_start_matches('_');
            if ["assert", "raises", "warns"].iter().any(|word| lowered.contains(word))
                || CHECKING_PREFIXES.iter().any(|prefix| lowered.starts_with(prefix))
                || MATCHERS.contains(&lowered)
            {
                return true;
            }
        }
        let semantic = self.checker.semantic();
        if PYTEST_CHECKS.iter().any(|check| semantic.resolves_to(func, &["pytest", check])) {
            return true;
        }
        if escalates_warnings(call) {
            return true;
        }
        match func {
            Expr::Name(name) => {
                let id = name.id.as_str();
                // A callable fixture or an imported test helper may check what it runs.
                if self.parameters.contains(id) || self.support.contains(id) {
                    return true;
                }
                if let Some(helper) = self.checker.semantic().module_function(name) {
                    return self.follows(&helper.body, helper.range());
                }
                // A helper class the module defines may check in its methods.
                self.checker.module_classes().get(id).is_some_and(|class| self.follows(&class.body, class.range()))
            }
            // A base class's test, reached through `super()`, may assert.
            Expr::Attribute(attribute)
                if attribute
                    .value
                    .as_call_expr()
                    .is_some_and(|inner| inner.func.as_name_expr().is_some_and(|name| name.id == "super")) =>
            {
                true
            }
            Expr::Attribute(attribute) => {
                if let Expr::Name(root) = &*attribute.value {
                    let root = root.id.as_str();
                    if self.support.contains(root) {
                        return true;
                    }
                    if root == "self" {
                        // A method the class defines is followed; an inherited one may assert, as a base test class's
                        // helpers do.
                        let method = self.class.and_then(|class| {
                            class
                                .body
                                .iter()
                                .filter_map(Stmt::as_function_def_stmt)
                                .find(|method| method.name.as_str() == attribute.attr.as_str())
                        });
                        return method.is_none_or(|method| self.follows(&method.body, method.range()));
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn follows(&mut self, body: &'b [Stmt], range: TextRange) -> bool {
        if !self.seen.insert(range) {
            return false;
        }
        self.visit_body(body);
        let checks = self.checks;
        self.checks = false;
        checks
    }
}

// A warning filter set to `error` fails the test on any warning the code emits.
fn escalates_warnings(call: &ast::ExprCall) -> bool {
    let is_error =
        |expr: &Expr| expr.as_string_literal_expr().is_some_and(|literal| literal.value.to_str().starts_with("error"));
    match last_segment(&call.func) {
        Some("simplefilter" | "filterwarnings") => {
            call.arguments.args.first().is_some_and(is_error)
                || call.arguments.find_keyword("action").is_some_and(|keyword| is_error(&keyword.value))
        }
        Some("catch_warnings") => call.arguments.find_keyword("action").is_some_and(|keyword| is_error(&keyword.value)),
        _ => false,
    }
}
