use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Stmt};
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
            report_imprecise(walker, &alias.value, false);
        }
        Stmt::FunctionDef(function) => {
            report_type_params(walker, function.type_params.as_deref());
            let is_guard = function.returns.as_deref().is_some_and(|returns| {
                let Expr::Subscript(ast::ExprSubscript { value, .. }) = returns else {
                    return false;
                };
                matches!(last_segment(value), Some("TypeIs" | "TypeGuard"))
            });
            let settings = &walker.checker.settings.from_ruff;
            let checked = settings.ruff_runs(RuffRule::AnyType) && ann401_checks(walker, function);
            // ANN401 leaves the `self` or `cls` a method receives unchecked, unless the method is static, and the
            // `*args` and `**kwargs` alone when `allow-star-arg-any` is set.
            let receiver = receiver_name(walker, function);
            if !is_guard && !is_dunder(function.name.as_str()) {
                for parameter in &function.parameters {
                    if let Some(annotation) = parameter.annotation() {
                        let variadic = matches!(parameter, AnyParameterRef::Variadic(_));
                        let by_ann401 = checked
                            && !(variadic && settings.allow_star_arg_any)
                            && Some(parameter.name().range()) != receiver;
                        report_imprecise(walker, annotation, by_ann401);
                    }
                }
            }
            if let Some(returns) = &function.returns {
                report_imprecise(walker, returns, checked);
            }
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => report_imprecise(walker, annotation, false),
        _ => {}
    }
}

// Bounds and defaults are not annotations, but they name the type a parameter stands for, so they are checked too.
fn report_type_params(walker: &Walker, type_params: Option<&ast::TypeParams>) {
    for type_param in type_params.into_iter().flat_map(|type_params| &type_params.type_params) {
        if let ast::TypeParam::TypeVar(ast::TypeParamTypeVar { bound: Some(bound), .. }) = type_param {
            report_imprecise(walker, bound, false);
        }
        if let Some(default) = type_param.default() {
            report_imprecise(walker, default, false);
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

// The `Any` nodes ANN401 reports in an annotation, which this rule leaves out while ANN401 checks the function. Ruff
// reports the whole annotation once, when `Any` is the annotation or sits in one of the wrappers it searches: an
// `Optional`, a `Union` or `|` element, or the first argument of `Annotated`. Its search does not enter other
// subscripts, such as `list[Any]`, nor a quoted annotation, which this rule does not check either.
fn ann401_anys(walker: &Walker, expr: &Expr, found: &mut FxHashSet<TextRange>) {
    let semantic = walker.checker.semantic();
    match expr {
        Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::BitOr, right, .. }) => {
            ann401_anys(walker, left, found);
            ann401_anys(walker, right, found);
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            if semantic.match_typing_expr(value, "Optional") {
                ann401_anys(walker, slice, found);
            } else if semantic.match_typing_expr(value, "Union") {
                for element in slice_elements(slice) {
                    ann401_anys(walker, element, found);
                }
            } else if semantic.match_typing_expr(value, "Annotated")
                && let Some(first) = slice_elements(slice).first()
            {
                ann401_anys(walker, first, found);
            }
        }
        _ if semantic.match_typing_expr(expr, "Any") => {
            found.insert(expr.range());
        }
        _ => {}
    }
}

// The elements of a subscript's slice: those of a tuple, or the slice itself.
fn slice_elements(slice: &Expr) -> &[Expr] {
    match slice {
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts,
        _ => std::slice::from_ref(slice),
    }
}

fn report_imprecise(walker: &Walker, annotation: &Expr, ann401: bool) {
    let semantic = walker.checker.semantic();
    let mut left_to_ann401 = FxHashSet::default();
    if ann401 {
        ann401_anys(walker, annotation, &mut left_to_ann401);
    }
    let mut finder = ExprCollector::new(|expr: &Expr| {
        if semantic.is_builtin(expr, "object") || semantic.resolves_to(expr, &["builtins", "object"]) {
            return Some((expr.range(), OBJECT_MESSAGE));
        }
        (semantic.match_typing_expr(expr, "Any") && !left_to_ann401.contains(&expr.range()))
            .then(|| (expr.range(), ANY_MESSAGE))
    });
    finder.visit_expr(annotation);
    for (range, message) in finder.found {
        walker.checker.report(Rule::ImpreciseAnnotation, range, message);
    }
}
