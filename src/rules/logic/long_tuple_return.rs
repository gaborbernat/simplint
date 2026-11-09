use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::last_segment;

// The methods the pickle and copy protocols call, which return a tuple of the shape they read. `__getstate__` returns
// what the class's own `__setstate__` reads back.
const PROTOCOL_METHODS: [&str; 5] =
    ["__reduce__", "__reduce_ex__", "__getnewargs__", "__getnewargs_ex__", "__getstate__"];

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Return(ast::StmtReturn { value: Some(value), .. }) = stmt else {
        return;
    };
    let Expr::Tuple(tuple) = value.as_ref() else {
        return;
    };
    // The innermost scope is the function the return leaves, which a nested function's own return belongs to.
    let Some(Scope::Function(function)) = walker.scopes().last() else { return };
    let name = function.name.as_str();
    let length = tuple.elts.len();
    let limit = walker.checker.settings.project.knobs.max_tuple_elements;
    // A starred element stands for any number of values, so the tuple's length is unknown. A key function's tuple is
    // compared or hashed as a whole, and one declared `tuple[T, ...]` is a sequence: neither is unpacked by position.
    if length <= limit
        || PROTOCOL_METHODS.contains(&name)
        || name == "key"
        || name.ends_with("_key")
        || tuple.elts.iter().any(Expr::is_starred_expr)
        || function.returns.as_deref().is_some_and(|returns| is_sequence(walker, returns))
        || function.decorator_list.iter().any(|decorator| last_segment(&decorator.expression) == Some("override"))
    {
        return;
    }
    walker.checker.report(
        Rule::LongTupleReturn,
        value.range(),
        format!("`{name}` returns a tuple of {length} elements; a dataclass or a named tuple names its fields"),
    );
}

// An annotation of a tuple of any length, `tuple[T, ...]`.
fn is_sequence(walker: &Walker, annotation: &Expr) -> bool {
    let Expr::Subscript(ast::ExprSubscript { value, slice, .. }) = annotation else {
        return false;
    };
    let Expr::Tuple(ast::ExprTuple { elts, .. }) = slice.as_ref() else {
        return false;
    };
    let semantic = walker.checker.semantic();
    (semantic.is_builtin(value, "tuple") || semantic.match_typing_expr(value, "Tuple"))
        && matches!(elts.as_slice(), [_, Expr::EllipsisLiteral(_)])
}
