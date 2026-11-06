use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::{ExprCollector, for_each_body, last_segment};

const OBJECT_MESSAGE: &str = "`object` annotation; use the precise type";
const ANY_MESSAGE: &str = "`Any` accepts any value and turns off type checking for it; use a precise type";

pub fn statement(walker: &mut Walker, stmt: &Stmt) {
    match stmt {
        Stmt::ClassDef(class) => report_type_params(walker, class.type_params.as_deref()),
        Stmt::TypeAlias(alias) => {
            report_type_params(walker, alias.type_params.as_deref());
            report_imprecise(walker, &alias.value, Skip::NONE);
        }
        Stmt::FunctionDef(function) => {
            report_type_params(walker, function.type_params.as_deref());
            let is_guard = function.returns.as_deref().is_some_and(|returns| {
                let Expr::Subscript(ast::ExprSubscript { value, .. }) = returns else {
                    return false;
                };
                matches!(last_segment(value), Some("TypeIs" | "TypeGuard"))
            });
            let ann401 = walker.checker.settings.from_ruff.ruff_runs(RuffRule::AnyType);
            let checked = ann401 && ann401_checks(walker, function);
            // ANN401 leaves the `self` or `cls` a method receives unchecked, unless the method is static.
            let receiver = receiver_name(walker, function);
            if !is_guard && !is_dunder(function.name.as_str()) {
                for parameter in &function.parameters {
                    if let Some(annotation) = parameter.annotation() {
                        let by_ann401 = checked && Some(parameter.name().range()) != receiver;
                        report_imprecise(walker, annotation, Skip { whole_any: ann401, ann401: by_ann401 });
                    }
                }
            }
            if let Some(returns) = &function.returns {
                report_imprecise(walker, returns, Skip { whole_any: ann401, ann401: checked });
            }
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => report_imprecise(walker, annotation, Skip::NONE),
        _ => {}
    }
}

// Bounds and defaults are not annotations, but they name the type a parameter stands for, so they are checked too.
fn report_type_params(walker: &Walker, type_params: Option<&ast::TypeParams>) {
    for type_param in type_params.into_iter().flat_map(|type_params| &type_params.type_params) {
        if let ast::TypeParam::TypeVar(ast::TypeParamTypeVar { bound: Some(bound), .. }) = type_param {
            report_imprecise(walker, bound, Skip::NONE);
        }
        if let Some(default) = type_param.default() {
            report_imprecise(walker, default, Skip::NONE);
        }
    }
}

// Whether Ruff's ANN401 checks the annotations of `function`: it skips functions decorated with `@override`, and the
// implementation of an `@overload` set, whose signature the type checker reads from the stubs above it.
fn ann401_checks(walker: &Walker, function: &ast::StmtFunctionDef) -> bool {
    let semantic = walker.checker.semantic();
    let overridden =
        function.decorator_list.iter().any(|decorator| semantic.match_typing_expr(&decorator.expression, "override"));
    !overridden && !walker.checker.state().overload_implementations(walker.checker).contains(&function.start())
}

// The name of the first parameter of a method that is not static, which ANN401 skips since it receives the instance
// or the class.
fn receiver_name(walker: &Walker, function: &ast::StmtFunctionDef) -> Option<TextRange> {
    let is_method = matches!(walker.scopes().last(), Some(Scope::Class(_)));
    let semantic = walker.checker.semantic();
    let is_static =
        function.decorator_list.iter().any(|decorator| semantic.is_builtin(&decorator.expression, "staticmethod"));
    if !is_method || is_static {
        return None;
    }
    function.parameters.iter_non_variadic_params().next().map(|parameter| parameter.name().range())
}

// The `def`s that implement an `@overload` set: the `def` right after the last `@overload` stub of its name, with no
// class in between. Ruff's ANN401 skips them, since the type checker reads the signature from the stubs.
pub(crate) fn overload_implementations(checker: &Checker) -> FxHashSet<TextSize> {
    let semantic = checker.semantic();
    let is_overload = |function: &ast::StmtFunctionDef| {
        function.decorator_list.iter().any(|decorator| semantic.match_typing_expr(&decorator.expression, "overload"))
    };
    let mut implementations = FxHashSet::default();
    for_each_body(checker.body(), &mut |body, _| {
        let mut stub: Option<&ast::StmtFunctionDef> = None;
        for stmt in body {
            match stmt {
                Stmt::FunctionDef(function) => {
                    if let Some(stub) = stub
                        && stub.name.as_str() == function.name.as_str()
                        && !is_overload(function)
                    {
                        implementations.insert(function.start());
                    }
                    stub = is_overload(function).then_some(function);
                }
                Stmt::ClassDef(_) => stub = None,
                _ => {}
            }
        }
    });
    implementations
}

// Which `Any` reports the rule leaves out while Ruff's ANN401 runs: a whole-annotation `Any`, and an `Any` at the start
// of an annotation, since ANN401 reports from the first character, as in `Any | None`.
#[derive(Clone, Copy)]
struct Skip {
    whole_any: bool,
    ann401: bool,
}

impl Skip {
    const NONE: Self = Self { whole_any: false, ann401: false };
}

fn report_imprecise(walker: &Walker, annotation: &Expr, skip: Skip) {
    let semantic = walker.checker.semantic();
    let mut finder = ExprCollector::new(|expr: &Expr| {
        if semantic.is_builtin(expr, "object") || semantic.resolves_to(expr, &["builtins", "object"]) {
            return Some((expr.range(), OBJECT_MESSAGE));
        }
        let whole = skip.whole_any && expr.range() == annotation.range();
        let by_ann401 = skip.ann401 && expr.start() == annotation.start();
        (!(whole || by_ann401) && semantic.match_typing_expr(expr, "Any")).then(|| (expr.range(), ANY_MESSAGE))
    });
    finder.visit_expr(annotation);
    for (range, message) in finder.found {
        walker.checker.report(Rule::ImpreciseAnnotation, range, message);
    }
}
