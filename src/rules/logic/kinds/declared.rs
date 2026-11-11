use ruff_python_ast::{self as ast, Expr, Operator};
use ruff_text_size::{Ranged, TextRange};

use super::Kind;
use super::tables::{CONTAINERS, PATH_CLASSES, items, shared};
use crate::lint::checker::Checker;
use crate::rules::is_class_like;
use crate::semantic::BindingKind;
use crate::syntax::{is_classmethod, is_name, receiver, target_leaves};

// A class around the spot, with its index among the scopes.
#[derive(Clone, Copy)]
pub(super) struct Class<'a> {
    pub(super) def: &'a ast::StmtClassDef,
    pub(super) depth: usize,
}

pub(super) fn target_binds(target: &Expr, name: &str) -> bool {
    target_leaves(target).into_iter().any(|leaf| is_name(leaf, name))
}

pub(super) fn attribute_root(expr: &Expr) -> &Expr {
    let mut head = expr;
    while let Expr::Attribute(ast::ExprAttribute { value, .. }) = head {
        head = value;
    }
    head
}

fn decorator_callee(decorator: &ast::Decorator) -> &Expr {
    match &decorator.expression {
        Expr::Call(ast::ExprCall { func, .. }) => func.as_ref(),
        expr => expr,
    }
}

// The name a method calls its instance, unless it is a static or class method.
pub(super) fn self_parameter(method: &ast::StmtFunctionDef) -> Option<&str> {
    receiver(method).filter(|_| !is_classmethod(method))
}

pub(super) fn is_builtin_call(checker: &Checker, expr: &Expr, names: &[&str]) -> bool {
    names.iter().any(|name| checker.semantic().is_builtin(expr, name))
}

// Whether a decorator hands back the function's own result: `staticmethod`, `classmethod`, a `functools` cache or
// `typing.override`.
pub(super) fn keeps_return(checker: &Checker, decorator: &ast::Decorator) -> bool {
    let expr = decorator_callee(decorator);
    is_builtin_call(checker, expr, &["staticmethod", "classmethod"])
        || checker.semantic().qualified_name(expr).is_some_and(|name| {
            matches!(
                name.segments(),
                ["functools", "cache" | "lru_cache"] | ["typing" | "typing_extensions", "override"]
            )
        })
}

pub(super) fn is_property(checker: &Checker, decorator: &ast::Decorator) -> bool {
    is_builtin_call(checker, &decorator.expression, &["property"])
        || checker
            .semantic()
            .qualified_name(&decorator.expression)
            .is_some_and(|name| name.segments() == ["functools", "cached_property"])
}

// The kind an annotation declares.
pub(super) fn annotation(checker: &Checker, expr: &Expr) -> Option<Kind> {
    let semantic = checker.semantic();
    let simple = |name: &str| match name {
        "str" => Some(Kind::Str),
        "int" => Some(Kind::Int),
        "bytes" => Some(Kind::Bytes),
        _ => None,
    };
    match expr {
        Expr::Name(name) if !semantic.is_bound(&name.id) => simple(&name.id),
        Expr::Name(name) => class_named(checker, &name.id)
            .map(|class| Kind::Instance(class.range))
            .or_else(|| path_class(checker, expr))
            .or_else(|| imported_class(checker, expr)),
        Expr::StringLiteral(string) => {
            let name = string.value.to_str();
            simple(name).or_else(|| class_named(checker, name).map(|class| Kind::Instance(class.range)))
        }
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::BitOr, right, .. })
            if right.is_none_literal_expr() && annotation(checker, left) == Some(Kind::Str) =>
        {
            Some(Kind::OptionalStr)
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            if semantic.match_typing_expr(value, "Final") {
                return annotation(checker, slice);
            }
            if semantic.match_typing_expr(value, "Optional") {
                return (annotation(checker, slice) == Some(Kind::Str)).then_some(Kind::OptionalStr);
            }
            let elts = match slice.as_ref() {
                Expr::Tuple(tuple) => tuple.elts.as_slice(),
                single => std::slice::from_ref(single),
            };
            if semantic.match_typing_expr(value, "Annotated") {
                return annotation(checker, elts.first()?);
            }
            let container = match value.as_ref() {
                Expr::Name(name) if !semantic.is_bound(&name.id) => name.id.to_string(),
                _ => {
                    let name = semantic.qualified_name(value)?;
                    match name.segments() {
                        ["typing" | "typing_extensions", container] | ["collections", "abc", container] => {
                            (*container).to_string()
                        }
                        _ => return None,
                    }
                }
            };
            if !CONTAINERS.contains(&container.as_str()) {
                return None;
            }
            let mut kinds = Vec::new();
            for elt in elts.iter().filter(|elt| !elt.is_ellipsis_literal_expr()) {
                kinds.push(annotation(checker, elt)?);
            }
            shared(kinds).and_then(items)
        }
        _ => path_class(checker, expr).or_else(|| imported_class(checker, expr)),
    }
}

// An instance of an imported class that an annotation names: the class of the run, or else one spelled as a class,
// which is not a typing alias.
pub(super) fn imported_class(checker: &Checker, expr: &Expr) -> Option<Kind> {
    let semantic = checker.semantic();
    let name = semantic.qualified_name(expr)?;
    let root = attribute_root(expr).as_name_expr()?;
    if !semantic.bindings(root.id.as_str()).iter().all(|binding| binding.kind == BindingKind::Import) {
        return None;
    }
    let run = expr.as_name_expr().and_then(|bound| checker.project().run_import_class(&bound.id));
    match (run, name.segments()) {
        (Some(true), _) => Some(Kind::ImportedInstance),
        (Some(false), _) | (None, ["typing" | "typing_extensions", ..]) => None,
        (None, segments) => segments.last().is_some_and(|last| is_class_like(last)).then_some(Kind::ImportedInstance),
    }
}

pub(super) fn path_class(checker: &Checker, expr: &Expr) -> Option<Kind> {
    checker.semantic().qualified_name(expr).and_then(|name| match name.segments() {
        ["pathlib", class] if PATH_CLASSES.contains(class) => Some(Kind::Path),
        _ => None,
    })
}

// The class the module defines once under `name`, at its top level.
pub(in crate::rules::logic) fn class_named<'a>(checker: &Checker<'a>, name: &str) -> Option<&'a ast::StmtClassDef> {
    let [binding] = checker.semantic().bindings(name) else {
        return None;
    };
    if binding.kind != BindingKind::Class {
        return None;
    }
    checker.module_classes().get(name).copied()
}

// Statements of a module body come in source order, so the class starting at `range` is found by a search.
pub(super) fn class_at<'a>(checker: &Checker<'a>, range: TextRange) -> Option<&'a ast::StmtClassDef> {
    let body = checker.body();
    let index = body.binary_search_by_key(&range.start(), Ranged::start).ok()?;
    body[index].as_class_def_stmt().filter(|class| class.range == range)
}

// What calling the class gives: an instance, unless the class has a `__new__`, a metaclass or a decorator that may
// return something else.
pub(super) fn instance_of(checker: &Checker, class: &ast::StmtClassDef) -> Option<Kind> {
    let plain = class.keywords().is_empty()
        && class.decorator_list.iter().all(|decorator| {
            checker.semantic().qualified_name(decorator_callee(decorator)).is_some_and(|name| {
                matches!(
                    name.segments(),
                    ["dataclasses", "dataclass"] | ["functools", "total_ordering"] | ["typing", "final"]
                )
            })
        })
        && !class
            .body
            .iter()
            .any(|stmt| stmt.as_function_def_stmt().is_some_and(|function| function.name.as_str() == "__new__"));
    (plain && class_named(checker, &class.name).is_some()).then_some(Kind::Instance(class.range))
}
