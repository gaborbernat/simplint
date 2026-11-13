use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::callee::local_function;
use crate::rules::testing::collected_tests;
use crate::syntax::{alias_binding, decorator_callee, docstring, last_segment, receiver};

pub fn test_without_assertion(checker: &Checker) {
    // pytest never runs the tests of a module it does not collect, such as a type checker's input.
    if !checker.is_test_module() || !checker.settings.project.pytest.collects(checker.file_name()) {
        return;
    }
    let support = support_names(checker.body());
    let mut verdicts = FxHashMap::default();
    for test in collected_tests(checker) {
        if is_skipped(test)
            // pytest-benchmark's fixtures measure rather than check.
            || test.parameters.iter().any(|parameter| matches!(parameter.name().as_str(), "benchmark" | "aio_benchmark"))
            || only_runs(checker, test)
            || test
                .decorator_list
                .iter()
                .any(|decorator| decorator.expression.as_call_expr().is_some_and(escalates_warnings))
        {
            continue;
        }
        let parameters: FxHashSet<&str> = test.parameters.iter().map(|parameter| parameter.name().as_str()).collect();
        let class = class_of(checker, test);
        let mut finder = Checks {
            checker,
            support: &support,
            parameters,
            class,
            method: method_in(class, test),
            seen: FxHashSet::default(),
            verdicts: &mut verdicts,
            cut: false,
            depth: 0,
            checked: false,
        };
        finder.visit_body(&test.body);
        if !finder.checked {
            checker.report(
                Rule::TestWithoutAssertion,
                test.name.range(),
                "This test checks nothing, so it passes for any code that does not raise; assert on the result",
            );
        }
    }
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
                        names.insert(alias_binding(alias));
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

// Tests pytest always skips or expects to fail are placeholders, not checks that went missing.
fn is_skipped(test: &ast::StmtFunctionDef) -> bool {
    test.decorator_list.iter().any(|decorator| {
        UnqualifiedName::from_expr(decorator_callee(decorator)).is_some_and(|name| {
            matches!(name.segments(), [.., "mark", "skip" | "xfail"] | ["unittest", "skip"] | ["skip"])
        })
    })
}

// Words in a test's name, docstring or comments that say it only checks the code runs without raising, crashing or
// warning, as a smoke or regression test does.
fn only_runs(checker: &Checker, test: &ast::StmtFunctionDef) -> bool {
    let docstring = docstring(&test.body).map(|literal| literal.value.to_str());
    let comments =
        checker.comment_ranges().comments_in_range(test.range()).iter().map(|comment| checker.text(*comment));
    std::iter::once(test.name.as_str()).chain(docstring).chain(comments).any(|text| {
        let lowered = text.to_ascii_lowercase();
        ["smoke", "sanity", "crash", "segfault", "raise", "error", "hang", "leak", "works", "import"]
            .iter()
            .any(|word| lowered.contains(word))
    })
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

fn class_of<'a>(checker: &Checker<'a>, test: &ast::StmtFunctionDef) -> Option<&'a ast::StmtClassDef> {
    checker.body().iter().filter_map(Stmt::as_class_def_stmt).find(|class| class.range().contains_range(test.range()))
}

// `function` when it is a method of `class`, whose calls through its receiver reach the other methods.
fn method_in<'a>(
    class: Option<&'a ast::StmtClassDef>,
    function: &'a ast::StmtFunctionDef,
) -> Option<&'a ast::StmtFunctionDef> {
    class?.body.iter().filter_map(Stmt::as_function_def_stmt).find(|method| std::ptr::eq(*method, function))
}

struct Checks<'a, 'b> {
    checker: &'a Checker<'b>,
    support: &'a FxHashSet<&'b str>,
    parameters: FxHashSet<&'b str>,
    class: Option<&'b ast::StmtClassDef>,
    // The method of `class` whose body the walk is in.
    method: Option<&'b ast::StmtFunctionDef>,
    // The local helpers already followed for this test, so recursion ends.
    seen: FxHashSet<TextRange>,
    // Whether each helper checks, by its range and the class of the test, shared by the tests of the module, which
    // often call the same helpers.
    verdicts: &'a mut FxHashMap<(TextRange, Option<TextRange>), bool>,
    // Whether the walk of the current helper stopped at a helper already being followed, which leaves its verdict
    // unknown, so it is not kept.
    cut: bool,
    // How many helpers deep the walk is, 0 in the test itself.
    depth: usize,
    checked: bool,
}

impl<'b> Visitor<'b> for Checks<'_, 'b> {
    fn visit_stmt(&mut self, stmt: &'b Stmt) {
        if self.checked {
            return;
        }
        if matches!(stmt, Stmt::Assert(_) | Stmt::Raise(_)) {
            self.checked = true;
            return;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'b Expr) {
        if self.checked {
            return;
        }
        if let Expr::Call(call) = expr
            && (self.call_checks(call) || self.passes_check(call))
        {
            self.checked = true;
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
            // Helpers such as `check_output`, `verify_signature` or Playwright's `expect` check a result, and
            // pytester's line matchers fail the test when the output does not match.
            if ["assert", "raises", "warns"].iter().any(|word| lowered.contains(word))
                || ["check", "verify", "expect", "validate", "ensure", "compare", "fail"]
                    .iter()
                    .any(|prefix| lowered.starts_with(prefix))
                || matches!(
                    lowered,
                    "fnmatch_lines"
                        | "fnmatch_lines_random"
                        | "re_match_lines"
                        | "re_match_lines_random"
                        | "no_fnmatch_line"
                        | "no_re_match_line"
                )
            {
                return true;
            }
        }
        let semantic = self.checker.semantic();
        if ["deprecated_call", "fail", "skip", "xfail", "importorskip"]
            .iter()
            .any(|check| semantic.resolves_to(func, &["pytest", check]))
        {
            return true;
        }
        if escalates_warnings(call) {
            return true;
        }
        match func {
            Expr::Name(name) => {
                let id = name.id.as_str();
                // A callable fixture or an imported test helper may check what it runs. The fixtures are the test's
                // own parameters, which a helper does not see.
                if self.depth == 0 && self.parameters.contains(id) || self.support.contains(id) {
                    return true;
                }
                if let Some((helper, _)) = local_function(semantic, func, None) {
                    return self.follows_function(helper);
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
                    let method = self.method.zip(self.class);
                    if method.is_some_and(|(method, _)| receiver(method) == Some(root)) {
                        // A method the class defines is followed; an inherited one may assert, as a base test class's
                        // helpers do.
                        return local_function(semantic, func, method)
                            .is_none_or(|(method, _)| self.follows_function(method));
                    }
                }
                false
            }
            _ => false,
        }
    }

    // A local function or a method of the test's class handed to a call as a callback, such as an event handler,
    // runs its checks when the code under test calls it.
    fn passes_check(&mut self, call: &'b ast::ExprCall) -> bool {
        let semantic = self.checker.semantic();
        call.arguments.args.iter().chain(call.arguments.keywords.iter().map(|keyword| &keyword.value)).any(|value| {
            let method = match value {
                Expr::Name(_) => None,
                Expr::Attribute(ast::ExprAttribute { value: root, .. })
                    if self
                        .method
                        .is_some_and(|method| receiver(method) == root.as_name_expr().map(|name| name.id.as_str())) =>
                {
                    self.method.zip(self.class)
                }
                _ => return false,
            };
            local_function(semantic, value, method).is_some_and(|(function, _)| self.follows_function(function))
        })
    }

    fn follows_function(&mut self, function: &'b ast::StmtFunctionDef) -> bool {
        let outer = std::mem::replace(&mut self.method, method_in(self.class, function));
        let checks = self.follows(&function.body, function.range());
        self.method = outer;
        checks
    }

    fn follows(&mut self, body: &'b [Stmt], range: TextRange) -> bool {
        let key = (range, self.class.map(Ranged::range));
        if let Some(&checks) = self.verdicts.get(&key) {
            return checks;
        }
        if !self.seen.insert(range) {
            self.cut = true;
            return false;
        }
        let outer_cut = std::mem::replace(&mut self.cut, false);
        self.depth += 1;
        self.visit_body(body);
        self.depth -= 1;
        let checks = std::mem::replace(&mut self.checked, false);
        if !self.cut {
            self.verdicts.insert(key, checks);
        }
        self.cut |= outer_cut;
        checks
    }
}
