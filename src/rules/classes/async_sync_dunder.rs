use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::yields;

// The special methods Python calls without awaiting the result. `__getitem__`, `__getattr__`, `__get__` and `__call__`
// are left out, as `await obj[key]`, `await obj.name` and `await obj()` are ways to design an awaitable API.
const SYNCHRONOUS: [&str; 52] = [
    "__init__", "__new__", "__del__", "__init_subclass__", "__set_name__", "__class_getitem__", "__repr__",
    "__str__", "__bytes__", "__format__", "__hash__", "__bool__", "__len__", "__length_hint__", "__index__", "__int__",
    "__float__", "__complex__", "__iter__", "__next__", "__reversed__", "__contains__", "__setitem__", "__delitem__",
    "__missing__", "__setattr__", "__delattr__", "__dir__", "__set__", "__delete__", "__enter__", "__exit__",
    "__eq__", "__ne__", "__lt__", "__le__", "__gt__", "__ge__", "__copy__", "__deepcopy__", "__reduce__",
    "__reduce_ex__", "__getstate__", "__setstate__", "__getnewargs__", "__getnewargs_ex__", "__instancecheck__",
    "__subclasscheck__", "__aiter__", "__await__", "__fspath__", "__sizeof__",
];

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(ast::StmtFunctionDef { is_async: true, name, body, .. }) = stmt else {
        return;
    };
    if !matches!(walker.scopes().last(), Some(Scope::Class(_))) || !SYNCHRONOUS.contains(&name.as_str()) {
        return;
    }
    // An `async def __aiter__` that yields is an async generator function, and the async generator it returns is the
    // async iterator `async for` asks for.
    let generator = yields(body);
    if generator && name.as_str() == "__aiter__" {
        return;
    }
    let result = if generator { "an async generator" } else { "an unawaited coroutine" };
    walker.checker.report(
        Rule::AsyncSyncDunder,
        name.range(),
        format!(
            "Python calls `{name}` without awaiting it, so `async def` makes it return {result}; define it with `def`"
        ),
    );
}
