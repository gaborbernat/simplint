use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, AnyParameterRef, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use super::is_callable;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::{ExprCollector, for_each_child_body, last_segment};

pub fn statement(walker: &mut Walker, stmt: &Stmt) {
    match stmt {
        Stmt::ClassDef(class) => report_type_params(walker, class.type_params.as_deref()),
        Stmt::TypeAlias(alias) => {
            report_type_params(walker, alias.type_params.as_deref());
            report_imprecise(walker, &alias.value, false);
        }
        Stmt::FunctionDef(function) => {
            report_type_params(walker, function.type_params.as_deref());
            let checked = ann401_checks(walker, function);
            let is_guard = function.returns.as_deref().is_some_and(|returns| {
                let Expr::Subscript(ast::ExprSubscript { value, .. }) = returns else {
                    return false;
                };
                matches!(last_segment(value), Some("TypeIs" | "TypeGuard"))
            });
            if !is_guard && !is_dunder(function.name.as_str()) {
                let receiver = receiver_name(walker, function);
                for parameter in &function.parameters {
                    if let Some(annotation) = parameter.annotation() {
                        let variadic = matches!(parameter, AnyParameterRef::Variadic(_));
                        // ANN401 rejects `Any` here, so `object` is the one annotation both tools accept.
                        if variadic && is_object(walker, annotation) {
                            continue;
                        }
                        // ANN401 leaves the `self` or `cls` a method receives unchecked, unless the method is static,
                        // and the `*args` and `**kwargs` alone when `allow-star-arg-any` is set.
                        let by_ann401 = checked
                            && !(variadic && walker.checker.settings.from_ruff.allow_star_arg_any)
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

// ANN401 skips `@override` functions and the implementations of an `@overload` set, whose signature the type checker
// reads from the stubs.
fn ann401_checks(walker: &Walker, function: &ast::StmtFunctionDef) -> bool {
    let semantic = walker.checker.semantic();
    let overridden =
        function.decorator_list.iter().any(|decorator| semantic.match_typing_expr(&decorator.expression, "override"));
    !overridden && !walker.checker.state().overload_implementations(walker.checker).contains(&function.start())
}

// The first parameter of a non-static method receives the instance or the class, which ANN401 skips.
fn receiver_name(walker: &Walker, function: &ast::StmtFunctionDef) -> Option<TextRange> {
    let semantic = walker.checker.semantic();
    if !matches!(walker.scopes().last(), Some(Scope::Class(_)))
        || function.decorator_list.iter().any(|decorator| semantic.is_builtin(&decorator.expression, "staticmethod"))
    {
        return None;
    }
    function.parameters.iter_non_variadic_params().next().map(|parameter| parameter.name().range())
}

fn is_object(walker: &Walker, expr: &Expr) -> bool {
    let semantic = walker.checker.semantic();
    semantic.is_builtin(expr, "object") || semantic.resolves_to(expr, &["builtins", "object"])
}

fn report_imprecise(walker: &Walker, annotation: &Expr, ann401: bool) {
    let semantic = walker.checker.semantic();
    let mut left_to_ann401 = FxHashSet::default();
    if ann401 {
        ann401_anys(walker, annotation, &mut left_to_ann401);
    }
    // The collector visits a subscript before its elements, so the return of a `Callable` is known before `object` is
    // picked. `Callable[..., object]` says the return value is ignored, which is the point of `object` there.
    let mut callable_returns = FxHashSet::default();
    // The arguments of `Annotated` after the first are metadata, where `object` is a value rather than a type.
    let mut metadata = Vec::new();
    let mut finder = ExprCollector::new(|expr: &Expr| {
        if let Expr::Subscript(ast::ExprSubscript { value, slice, .. }) = expr {
            if is_callable(semantic, value)
                && let Some(returns) = slice_elements(slice).last()
            {
                callable_returns.insert(returns.range());
            }
            if semantic.match_typing_expr(value, "Annotated") {
                metadata.extend(slice_elements(slice).iter().skip(1).map(Ranged::range));
            }
        }
        if is_object(walker, expr) {
            if metadata.iter().any(|range| range.contains_range(expr.range())) {
                return None;
            }
            return (!callable_returns.contains(&expr.range()))
                .then_some((expr.range(), "`object` annotation; use the precise type"));
        }
        (semantic.match_typing_expr(expr, "Any") && !left_to_ann401.contains(&expr.range()))
            .then(|| (expr.range(), "`Any` accepts any value and turns off type checking for it; use a precise type"))
    });
    finder.visit_expr(annotation);
    for (range, message) in finder.found {
        walker.checker.report(Rule::ImpreciseAnnotation, range, message);
    }
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

fn slice_elements(slice: &Expr) -> &[Expr] {
    match slice {
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts,
        _ => std::slice::from_ref(slice),
    }
}

// The `def`s that implement an `@overload` set. Ruff's ANN401 skips them, since the type checker reads the signature
// from the stubs. Ruff reads the definitions in order, so a stub carries on through the bodies of an `if` or a `try`
// that follow it, until a `def` or a class resets it.
pub(crate) fn overload_implementations(checker: &Checker) -> FxHashSet<TextSize> {
    let mut implementations = FxHashSet::default();
    overload_implementations_in(checker, checker.body(), &mut None, &mut implementations);
    implementations
}

fn overload_implementations_in<'a>(
    checker: &Checker,
    body: &'a [Stmt],
    stub: &mut Option<&'a str>,
    implementations: &mut FxHashSet<TextSize>,
) {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(function) => {
                if stub.is_some_and(|name| name == function.name.as_str()) && !is_overload(checker, function) {
                    implementations.insert(function.start());
                }
                *stub = is_overload(checker, function).then_some(function.name.as_str());
                overload_implementations_in(checker, &function.body, &mut None, implementations);
            }
            Stmt::ClassDef(class) => {
                *stub = None;
                overload_implementations_in(checker, &class.body, &mut None, implementations);
            }
            _ => for_each_child_body(stmt, &mut |child| {
                overload_implementations_in(checker, child, stub, implementations);
            }),
        }
    }
}

fn is_overload(checker: &Checker, function: &ast::StmtFunctionDef) -> bool {
    let semantic = checker.semantic();
    function.decorator_list.iter().any(|decorator| semantic.match_typing_expr(&decorator.expression, "overload"))
}
