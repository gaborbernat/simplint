use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::recognize::is_fixture;
use crate::rules::testing::{is_test_class, is_test_function};
use crate::semantic::{Referent, binding_referent};

const ENVIRON_METHODS: &[&str] = &["update", "pop", "popitem", "setdefault", "clear", "__setitem__", "__delitem__"];
const LIST_METHODS: &[&str] = &["append", "insert", "extend", "remove", "pop", "clear", "__setitem__", "__delitem__"];
const MODULES_METHODS: &[&str] = &["update", "pop", "popitem", "setdefault", "clear", "__setitem__", "__delitem__"];

pub fn test_process_state(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    for function in functions(checker) {
        // A fixture that yields, a function with a `finally`, or one that registers a cleanup may put the state back
        // itself.
        if restores_itself(function) {
            continue;
        }
        let mut finder = Changes { checker, found: Vec::new(), restored_depth: 0 };
        finder.visit_body(&function.body);
        let mut saves = Saves { checker, saved: FxHashSet::default() };
        saves.visit_body(&function.body);
        for (range, what) in finder.found.into_iter().filter(|(_, what)| !saves.saved.contains(what)) {
            checker.report(
                Rule::TestProcessState,
                range,
                format!("Changing {what} by hand leaks into the tests that run after this one; {}", remedy(what)),
            );
        }
    }
}

fn remedy(what: &str) -> &'static str {
    match what {
        "an environment variable" => "use `monkeypatch.setenv` or `monkeypatch.delenv`",
        "the working directory" => "use `monkeypatch.chdir`",
        "`sys.path`" => "use `monkeypatch.syspath_prepend`",
        "`sys.modules`" => "use `monkeypatch.setitem` or `mock.patch.dict`",
        "`sys.argv`" => "use `monkeypatch.setattr(sys, \"argv\", ...)`",
        _ => "use `monkeypatch.setattr` or `mock.patch.object`",
    }
}

// The teardown hooks that run after the tests of their class or module, so they can put back what the tests change.
const TEARDOWNS: &[&str] =
    &["tearDown", "tearDownClass", "teardown_method", "teardown_class", "teardown_module", "teardown_function"];
// The fixtures that snapshot the working directory, `sys.path` and `sys.modules`, and restore them after the test.
const SNAPSHOTTING_FIXTURES: &[&str] = &["pytester", "testdir"];

// The tests, fixtures and methods of test classes, which run as part of a test. A class or module with a teardown hook,
// or a `usefixtures` mark, may restore the state, so its functions are left out.
fn functions<'a>(checker: &Checker<'a>) -> Vec<&'a ast::StmtFunctionDef> {
    let body = checker.body();
    if defines_teardown(body) || body.iter().any(is_pytestmark_usefixtures) {
        return Vec::new();
    }
    let mut functions = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(function)
                if (is_test_function(function) || function.decorator_list.iter().any(is_fixture))
                    && !uses_fixtures(&function.decorator_list) =>
            {
                functions.push(function);
            }
            Stmt::ClassDef(class)
                if is_test_class(checker, class) && !defines_teardown(&class.body) && !uses_fixtures(&class.decorator_list) =>
            {
                functions.extend(
                    class
                        .body
                        .iter()
                        .filter_map(Stmt::as_function_def_stmt)
                        .filter(|method| !uses_fixtures(&method.decorator_list)),
                );
            }
            _ => {}
        }
    }
    functions.retain(|function| {
        !function.parameters.iter().any(|parameter| SNAPSHOTTING_FIXTURES.contains(&parameter.name().as_str()))
    });
    functions
}

fn defines_teardown(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_function_def_stmt).any(|function| TEARDOWNS.contains(&function.name.as_str()))
}

fn uses_fixtures(decorators: &[ast::Decorator]) -> bool {
    decorators.iter().any(|decorator| is_usefixtures(&decorator.expression))
}

fn is_usefixtures(expr: &Expr) -> bool {
    let func = expr.as_call_expr().map_or(expr, |call| &call.func);
    func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "usefixtures")
}

fn is_pytestmark_usefixtures(stmt: &Stmt) -> bool {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return false;
    };
    targets.iter().any(|target| crate::syntax::is_name(target, "pytestmark"))
        && match &**value {
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().any(is_usefixtures),
            other => is_usefixtures(other),
        }
}

fn restores_itself(function: &ast::StmtFunctionDef) -> bool {
    struct Finder {
        found: bool,
    }
    impl<'a> Visitor<'a> for Finder {
        fn visit_stmt(&mut self, stmt: &'a Stmt) {
            if matches!(stmt, Stmt::Try(ast::StmtTry { finalbody, .. }) if !finalbody.is_empty()) {
                self.found = true;
            }
            if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
                visitor::walk_stmt(self, stmt);
            }
        }
        fn visit_expr(&mut self, expr: &'a Expr) {
            self.found |= matches!(expr, Expr::Yield(_) | Expr::YieldFrom(_))
                || expr.as_call_expr().and_then(|call| call.func.as_attribute_expr()).is_some_and(|attribute| {
                    matches!(attribute.attr.as_str(), "addCleanup" | "addfinalizer" | "enterContext" | "callback")
                });
            if !matches!(expr, Expr::Lambda(_)) {
                visitor::walk_expr(self, expr);
            }
        }
    }
    let mut finder = Finder { found: false };
    finder.visit_body(&function.body);
    finder.found
}

struct Changes<'a, 'b> {
    checker: &'a Checker<'b>,
    found: Vec<(TextRange, &'static str)>,
    // Inside a `with` block whose context manager may put the state back on exit.
    restored_depth: usize,
}

impl<'a> Visitor<'a> for Changes<'_, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::With(with) if with.items.iter().any(|item| self.restores(&item.context_expr)) => {
                self.restored_depth += 1;
                visitor::walk_stmt(self, stmt);
                self.restored_depth -= 1;
                return;
            }
            Stmt::Assign(ast::StmtAssign { targets, .. }) => {
                for target in targets {
                    self.target(target, stmt.range());
                }
            }
            Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => self.target(target, stmt.range()),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(_), .. }) => self.target(target, stmt.range()),
            Stmt::Delete(ast::StmtDelete { targets, .. }) => {
                for target in targets {
                    if let Expr::Subscript(subscript) = target {
                        self.subscript(&subscript.value, stmt.range());
                    }
                }
            }
            _ => {}
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

impl Changes<'_, '_> {
    // A context manager, such as `patch.dict`, `monkeypatch.context()` or a project's own `fresh_env()`, may put the
    // state back on exit; only `open` is known not to.
    fn restores(&self, context: &Expr) -> bool {
        context.as_call_expr().is_some_and(|call| !self.checker.semantic().is_builtin(&call.func, "open"))
    }

    fn push(&mut self, range: TextRange, what: &'static str) {
        if self.restored_depth == 0 {
            self.found.push((range, what));
        }
    }

    fn target(&mut self, target: &Expr, statement: TextRange) {
        match target {
            Expr::Subscript(subscript) => self.subscript(&subscript.value, statement),
            Expr::Attribute(attribute) => {
                let semantic = self.checker.semantic();
                // Ruff's B003 reports rebinding `os.environ`.
                if semantic.resolves_to(target, &["os", "environ"]) {
                    return;
                }
                if semantic.resolves_to(target, &["sys", "path"]) {
                    self.push(statement, "`sys.path`");
                } else if semantic.resolves_to(target, &["sys", "modules"]) {
                    self.push(statement, "`sys.modules`");
                } else if semantic.resolves_to(target, &["sys", "argv"]) {
                    self.push(statement, "`sys.argv`");
                } else if self.is_module(&attribute.value) {
                    self.push(statement, "a module attribute");
                }
            }
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                for element in elts {
                    self.target(element, statement);
                }
            }
            _ => {}
        }
    }

    fn subscript(&mut self, value: &Expr, statement: TextRange) {
        let semantic = self.checker.semantic();
        if semantic.resolves_to(value, &["os", "environ"]) {
            self.push(statement, "an environment variable");
        } else if semantic.resolves_to(value, &["sys", "path"]) {
            self.push(statement, "`sys.path`");
        } else if semantic.resolves_to(value, &["sys", "modules"]) {
            self.push(statement, "`sys.modules`");
        } else if semantic.resolves_to(value, &["sys", "argv"]) {
            self.push(statement, "`sys.argv`");
        }
    }

    fn call(&mut self, call: &ast::ExprCall) {
        let semantic = self.checker.semantic();
        let func = &*call.func;
        if semantic.resolves_to(func, &["os", "putenv"]) || semantic.resolves_to(func, &["os", "unsetenv"]) {
            self.push(call.range(), "an environment variable");
        } else if semantic.resolves_to(func, &["os", "chdir"]) {
            self.push(call.range(), "the working directory");
        } else if let Expr::Attribute(attribute) = func {
            let method = attribute.attr.as_str();
            if ENVIRON_METHODS.contains(&method) && semantic.resolves_to(&attribute.value, &["os", "environ"]) {
                self.push(call.range(), "an environment variable");
            } else if LIST_METHODS.contains(&method) && semantic.resolves_to(&attribute.value, &["sys", "path"]) {
                self.push(call.range(), "`sys.path`");
            } else if MODULES_METHODS.contains(&method) && semantic.resolves_to(&attribute.value, &["sys", "modules"])
            {
                self.push(call.range(), "`sys.modules`");
            }
        }
    }

    // A name every binding of which imports a module, as `import json` does; `from x import y` may bind anything.
    fn is_module(&self, value: &Expr) -> bool {
        let Expr::Name(name) = value else {
            return false;
        };
        let bindings = self.checker.semantic().resolved_bindings(name);
        !bindings.is_empty() && bindings.iter().all(|binding| matches!(binding_referent(binding), Referent::Module))
    }
}

// What the function copies before changing it, as `old = os.getcwd()` or `saved = sys.path[:]` do, to put back later.
struct Saves<'a, 'b> {
    checker: &'a Checker<'b>,
    saved: FxHashSet<&'static str>,
}

impl<'a> Visitor<'a> for Saves<'_, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Assign(ast::StmtAssign { value, .. }) = stmt {
            if let Some(what) = self.copied(value) {
                self.saved.insert(what);
            }
            // `original = module.attribute` keeps the value a test puts back.
            if let Expr::Attribute(attribute) = &**value
                && let Expr::Name(name) = &*attribute.value
                && self.checker.semantic().resolved_bindings(name).iter().all(|binding| matches!(binding_referent(binding), Referent::Module))
            {
                self.saved.insert("a module attribute");
            }
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr
            && self.checker.semantic().resolves_to(&call.func, &["os", "getcwd"])
        {
            self.saved.insert("the working directory");
        }
        visitor::walk_expr(self, expr);
    }
}

impl Saves<'_, '_> {
    fn copied(&self, value: &Expr) -> Option<&'static str> {
        let source = match value {
            Expr::Subscript(subscript) if subscript.slice.is_slice_expr() => &*subscript.value,
            Expr::Call(call) => match &*call.func {
                Expr::Attribute(attribute) if attribute.attr.as_str() == "copy" => &*attribute.value,
                func if self.checker.semantic().is_builtin(func, "list") || self.checker.semantic().is_builtin(func, "dict") => {
                    call.arguments.args.first()?
                }
                _ => return None,
            },
            _ => return None,
        };
        let semantic = self.checker.semantic();
        [
            (["os", "environ"], "an environment variable"),
            (["sys", "path"], "`sys.path`"),
            (["sys", "modules"], "`sys.modules`"),
            (["sys", "argv"], "`sys.argv`"),
        ]
        .into_iter()
        .find_map(|(segments, what)| semantic.resolves_to(source, &segments).then_some(what))
    }
}
