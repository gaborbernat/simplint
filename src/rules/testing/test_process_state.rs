use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::recognize::is_fixture;
use crate::rules::testing::{is_test_class, is_test_function};
use crate::semantic::{Referent, binding_referent};
use crate::syntax::{is_name, is_true_literal};

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
        for decorator in &function.decorator_list {
            saves.visit_expr(&decorator.expression);
        }
        saves.visit_body(&function.body);
        for (range, state) in finder.found.into_iter().filter(|(_, state)| !saves.saved.contains(state)) {
            checker.report(
                Rule::TestProcessState,
                range,
                format!(
                    "Changing {} by hand leaks into the tests that run after this one; {}",
                    state.noun(),
                    state.remedy()
                ),
            );
        }
    }
}

// The tests, fixtures and methods of test classes, which run as part of a test. A class or module with a teardown hook,
// or a `usefixtures` mark, may restore the state, so its functions are left out.
fn functions<'a>(checker: &Checker<'a>) -> Vec<&'a ast::StmtFunctionDef> {
    let body = checker.body();
    if defines_teardown(body) || body.iter().any(is_pytestmark_usefixtures) || body.iter().any(is_restoring_autouse) {
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
                if is_test_class(checker, class)
                    && !defines_teardown(&class.body)
                    && !uses_fixtures(&class.decorator_list)
                    && !inherits_hooks(checker, class)
                    && !class.decorator_list.iter().any(|decorator| is_patcher(&decorator.expression)) =>
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
    // pytester snapshots the working directory, `sys.path` and `sys.modules`, and restores them after the test.
    functions.retain(|function| {
        !function.parameters.iter().any(|parameter| matches!(parameter.name().as_str(), "pytester" | "testdir"))
    });
    functions
}

// A teardown hook runs after the tests of its class or module, so it can put back what they change.
fn defines_teardown(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_function_def_stmt).any(|function| {
        matches!(
            function.name.as_str(),
            "tearDown"
                | "tearDownClass"
                | "teardown_method"
                | "teardown_class"
                | "teardown_module"
                | "teardown_function"
        )
    })
}

// An autouse fixture that restores what it changes runs around every test of the module, so it may put back what they
// change too.
fn is_restoring_autouse(stmt: &Stmt) -> bool {
    stmt.as_function_def_stmt().is_some_and(|function| {
        function.decorator_list.iter().any(|decorator| {
            is_fixture(decorator)
                && decorator.expression.as_call_expr().is_some_and(|call| {
                    call.arguments.find_keyword("autouse").is_some_and(|keyword| is_true_literal(&keyword.value))
                })
        }) && restores_itself(function)
    })
}

// A test class deriving from a class other than `unittest.TestCase` may inherit set-up and tear-down hooks, such as a
// base `setUp` that starts a patcher and a `tearDown` that stops it.
fn inherits_hooks(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    class.bases().iter().any(|base| {
        !checker.semantic().resolves_to(base, &["unittest", "TestCase"])
            && !checker.semantic().is_builtin(base, "object")
    })
}

// `patch`, `patch.object` or `patch.dict` as a decorator restores its target once the test ends.
fn is_patcher(expr: &Expr) -> bool {
    let func = expr.as_call_expr().map_or(expr, |call| &call.func);
    UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().contains(&"patch"))
}

fn is_pytestmark_usefixtures(stmt: &Stmt) -> bool {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return false;
    };
    targets.iter().any(|target| is_name(target, "pytestmark"))
        && match &**value {
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => {
                elts.iter().any(is_usefixtures)
            }
            other => is_usefixtures(other),
        }
}

fn is_usefixtures(expr: &Expr) -> bool {
    let func = expr.as_call_expr().map_or(expr, |call| &call.func);
    func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "usefixtures")
}

fn uses_fixtures(decorators: &[ast::Decorator]) -> bool {
    decorators.iter().any(|decorator| is_usefixtures(&decorator.expression))
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
    found: Vec<(TextRange, State)>,
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
            Stmt::AugAssign(ast::StmtAugAssign { target, .. })
            | Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(_), .. }) => self.target(target, stmt.range()),
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

    fn push(&mut self, range: TextRange, state: State) {
        if self.restored_depth == 0 {
            self.found.push((range, state));
        }
    }

    fn target(&mut self, target: &Expr, statement: TextRange) {
        match target {
            Expr::Subscript(subscript) => self.subscript(&subscript.value, statement),
            Expr::Attribute(attribute) => match State::of_object(self.checker, target) {
                // Ruff's B003 reports rebinding `os.environ`.
                Some(State::Environment) => {}
                Some(state) => self.push(statement, state),
                None if self.is_module(&attribute.value) => self.push(statement, State::ModuleAttribute),
                None => {}
            },
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                for element in elts {
                    self.target(element, statement);
                }
            }
            _ => {}
        }
    }

    fn subscript(&mut self, value: &Expr, statement: TextRange) {
        if let Some(state) = State::of_object(self.checker, value) {
            self.push(statement, state);
        }
    }

    fn call(&mut self, call: &ast::ExprCall) {
        let semantic = self.checker.semantic();
        let func = &*call.func;
        if semantic.resolves_to(func, &["os", "putenv"]) || semantic.resolves_to(func, &["os", "unsetenv"]) {
            self.push(call.range(), State::Environment);
        } else if semantic.resolves_to(func, &["os", "chdir"]) {
            self.push(call.range(), State::WorkingDirectory);
        } else if let Expr::Attribute(attribute) = func
            && let Some(state) = State::mutated_by(self.checker, attribute)
        {
            self.push(call.range(), state);
        }
    }

    fn is_module(&self, value: &Expr) -> bool {
        value.as_name_expr().is_some_and(|name| binds_module(self.checker, name))
    }
}

// A name every binding of which imports a module, as `import json` does; `from x import y` may bind anything.
fn binds_module(checker: &Checker, name: &ast::ExprName) -> bool {
    let bindings = checker.semantic().resolved_bindings(name);
    !bindings.is_empty() && bindings.iter().all(|binding| matches!(binding_referent(binding), Referent::Module))
}

// What the function copies before changing it, as `old = os.getcwd()` or `saved = sys.path[:]` do, to put back later.
struct Saves<'a, 'b> {
    checker: &'a Checker<'b>,
    saved: FxHashSet<State>,
}

impl<'a> Visitor<'a> for Saves<'_, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Assign(ast::StmtAssign { value, .. }) = stmt {
            if let Some(state) = self.copied(value) {
                self.saved.insert(state);
            }
            // `original = module.attribute` keeps the value a test puts back.
            if let Expr::Attribute(attribute) = &**value
                && let Expr::Name(name) = &*attribute.value
                && binds_module(self.checker, name)
            {
                self.saved.insert(State::ModuleAttribute);
            }
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            if self.checker.semantic().resolves_to(&call.func, &["os", "getcwd"]) {
                self.saved.insert(State::WorkingDirectory);
            }
            self.saved.extend(self.restored_by(call));
        }
        visitor::walk_expr(self, expr);
    }
}

impl Saves<'_, '_> {
    // The state a patcher or the monkeypatch fixture puts back once the test ends: `patch.dict(os.environ)` restores
    // the whole mapping, and `monkeypatch.setenv`, `delenv`, `syspath_prepend` and `chdir` record the value they
    // replace.
    fn restored_by(&self, call: &ast::ExprCall) -> Option<State> {
        let name = UnqualifiedName::from_expr(&call.func)?;
        match name.segments() {
            [.., "patch", "dict"] => State::of_object(self.checker, call.arguments.find_argument_value("in_dict", 0)?),
            [.., "monkeypatch", "setenv" | "delenv"] => Some(State::Environment),
            [.., "monkeypatch", "syspath_prepend"] => Some(State::SysPath),
            [.., "monkeypatch", "chdir"] => Some(State::WorkingDirectory),
            _ => None,
        }
    }

    fn copied(&self, value: &Expr) -> Option<State> {
        let source = match value {
            Expr::Subscript(subscript) if subscript.slice.is_slice_expr() => &*subscript.value,
            Expr::Call(call) => match &*call.func {
                Expr::Attribute(attribute) if attribute.attr.as_str() == "copy" => &*attribute.value,
                func if self.checker.semantic().is_builtin(func, "list")
                    || self.checker.semantic().is_builtin(func, "dict") =>
                {
                    call.arguments.args.first()?
                }
                _ => return None,
            },
            _ => return None,
        };
        State::of_object(self.checker, source)
    }
}

// The state of the test process a test changes for the tests that run after it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum State {
    Environment,
    WorkingDirectory,
    SysPath,
    SysModules,
    SysArgv,
    ModuleAttribute,
}

impl State {
    // The process-wide objects, by their qualified name, with the methods that change each in place.
    const OBJECTS: [([&str; 2], Self, &[&str]); 4] = [
        (["os", "environ"], Self::Environment, &Self::DICT_MUTATORS),
        (["sys", "path"], Self::SysPath, &Self::LIST_MUTATORS),
        (["sys", "modules"], Self::SysModules, &Self::DICT_MUTATORS),
        (["sys", "argv"], Self::SysArgv, &Self::LIST_MUTATORS),
    ];
    const DICT_MUTATORS: [&str; 7] = ["update", "pop", "popitem", "setdefault", "clear", "__setitem__", "__delitem__"];
    const LIST_MUTATORS: [&str; 8] =
        ["append", "insert", "extend", "remove", "pop", "clear", "__setitem__", "__delitem__"];

    fn of_object(checker: &Checker, expr: &Expr) -> Option<Self> {
        let semantic = checker.semantic();
        Self::OBJECTS
            .into_iter()
            .find_map(|(segments, state, _)| semantic.resolves_to(expr, &segments).then_some(state))
    }

    // The object a method call such as `sys.path.append` changes in place.
    fn mutated_by(checker: &Checker, method: &ast::ExprAttribute) -> Option<Self> {
        let semantic = checker.semantic();
        Self::OBJECTS.into_iter().find_map(|(segments, state, mutators)| {
            (mutators.contains(&method.attr.as_str()) && semantic.resolves_to(&method.value, &segments))
                .then_some(state)
        })
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Environment => "an environment variable",
            Self::WorkingDirectory => "the working directory",
            Self::SysPath => "`sys.path`",
            Self::SysModules => "`sys.modules`",
            Self::SysArgv => "`sys.argv`",
            Self::ModuleAttribute => "a module attribute",
        }
    }

    fn remedy(self) -> &'static str {
        match self {
            Self::Environment => "use `monkeypatch.setenv` or `monkeypatch.delenv`",
            Self::WorkingDirectory => "use `monkeypatch.chdir`",
            Self::SysPath => "use `monkeypatch.syspath_prepend`",
            Self::SysModules => "use `monkeypatch.setitem` or `mock.patch.dict`",
            Self::SysArgv => "use `monkeypatch.setattr(sys, \"argv\", ...)`",
            Self::ModuleAttribute => "use `monkeypatch.setattr` or `mock.patch.object`",
        }
    }
}
