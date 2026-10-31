## What it does

Checks for module-level constants that only one function uses. A constant here is a name assigned once, with `=` or an
[annotated assignment](https://docs.python.org/3/reference/simple_stmts.html#annotated-assignment-statements). It is
reported when every reference to it sits inside the body of a single top-level function or method, including functions
nested in it. A constant used only inside a module-level lambda is not reported.

These are skipped:

- public names: those listed in [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package),
  or, when the module has no `__all__`, builds it dynamically or binds it inside a nested block, names without a leading
  `_`
- names another file in the run imports, or patches with
  [`unittest.mock.patch`](https://docs.python.org/3/library/unittest.mock.html#patch) or
  [`patch.object`](https://docs.python.org/3/library/unittest.mock.html#patch-object)
- [dunder](https://docs.python.org/3/reference/lexical_analysis.html#reserved-classes-of-identifiers) names and
  class-like names such as `UserId`
- constants with a comment on the line directly above or at the end of their line
- names another module in the run assigns as an attribute, such as `config._TIMEOUT = 5`, since a local copy would
  ignore the override
- values the function changes, which are state that must outlive each call: it calls a mutating method on them such as
  `add`, `append` or `update`, assigns or deletes an item or attribute, uses them in a `with` statement, or passes them
  to `next`
- constants referenced at import time, such as in a default value or decorator, or in any annotation

## Why is this bad?

A module-level constant suggests the value is shared, so a reader has to search the module to learn that one function
uses it. Defined in that function, its scope matches its use. A comment above the constant, such as one saying it is
built once for speed, keeps it at module level.

## Example

```python
_TIMEOUT = 30


def connect(host: str) -> socket.socket:
    return socket.create_connection((host, 443), timeout=_TIMEOUT)
```

Use instead:

```python
def connect(host: str) -> socket.socket:
    return socket.create_connection((host, 443), timeout=30)
```

## Fix safety

The fix is always unsafe: the value is rebuilt on every call, which costs time for expensive values and gives each call
a new object for mutable ones. The fix moves the value in one of two ways:

- When the constant is used once, outside any loop, nested function or class, lambda or comprehension, and its value
  fits on one line, the fix inlines the value at the use, adding parentheses when needed. Inside an f-string it inlines
  only a name, attribute or number: a value with quotes there is a syntax error before Python 3.12, and a longer one
  buries the text the f-string builds. A string that calls `format` keeps a name: as a literal it becomes Ruff's
  [f-string (UP032)](https://docs.astral.sh/ruff/rules/f-string/) candidate, which Ruff reports only while no argument
  spans lines, so the result would depend on how `ruff format` lays out the call.
- Otherwise it declares a local before the statement that first uses it and renames the uses. The local takes the
  constant's name in lowercase, without leading underscores, and keeps its annotation, with `Final` taken off.

Of the blank lines above and below the constant, the fix removes the shorter run, so the code around it keeps the wider
gap. At the top of the file, the blank lines below go with it.

No fix is offered when:

- the constant shares a line with other code;
- the local name already appears in the file;
- the value reads a module name the module binds again or deletes later, since the function would see the later value;
- the value reads a module name the function also uses for a parameter or local, such as `operator.eq` in a function
  taking an `operator` argument.

## Conflicts with Ruff

Ruff's [magic-value-comparison (PLR2004)](https://docs.astral.sh/ruff/rules/magic-value-comparison/) asks for a named
constant where the inlined value lands in a comparison, and
[except-with-non-exception-classes (B030)](https://docs.astral.sh/ruff/rules/except-with-non-exception-classes/) reports
a conditional value inlined into an `except` clause. When a fix would give either rule a new report, simplint withdraws
it and keeps the report.
