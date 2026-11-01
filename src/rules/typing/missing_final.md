## What it does

Checks for module-level variables that are assigned once and not annotated
[`Final`](https://docs.python.org/3/library/typing.html#typing.Final). Only statements at the top level of the module
are checked: a plain assignment to one name, or an annotated assignment with a value. Assignments inside an `if`, `try`
or other block are not reported.

A name counts as reassigned, and is skipped, when the module binds it more than once. Any second binding counts: another
assignment, `+=` and other augmented assignments, `del`, an import, a `def`, a loop target, or an assignment in a
function that declares the name [`global`](https://docs.python.org/3/reference/simple_stmts.html#the-global-statement).

The rule does not run when the target Python version is older than 3.8, which introduced `typing.Final`. It also skips:

- dunder names such as `__version__`
- implicit type aliases: a generic such as `dict[str, Any]` or `Literal["a"]`, or a union of builtin types such as
  `str | bytes`
- names another module in the run assigns as an attribute, such as `config.TIMEOUT = 5`, an override `Final` would
  forbid
- `TYPE_CHECKING = False`, an idiom that avoids importing `typing` at runtime, which adding `Final` would defeat
- class-like names, which start with a capital letter and contain a lowercase one, such as `Alias`
- tuple unpacking such as `FIRST, SECOND = 1, 2`, and chained assignments such as `FIRST = SECOND = 1`
- annotations that already use `Final`, [`ClassVar`](https://docs.python.org/3/library/typing.html#typing.ClassVar) or
  [`TypeAlias`](https://docs.python.org/3/library/typing.html#typing.TypeAlias), also inside `Annotated[...]`
- values built by [`TypeVar`](https://docs.python.org/3/library/typing.html#typing.TypeVar),
  [`ParamSpec`](https://docs.python.org/3/library/typing.html#typing.ParamSpec),
  [`TypeVarTuple`](https://docs.python.org/3/library/typing.html#typing.TypeVarTuple),
  [`NewType`](https://docs.python.org/3/library/typing.html#typing.NewType),
  [`TypeAliasType`](https://docs.python.org/3/library/typing.html#typing.TypeAliasType),
  [`NamedTuple`](https://docs.python.org/3/library/typing.html#typing.NamedTuple) or
  [`TypedDict`](https://docs.python.org/3/library/typing.html#typing.TypedDict), which type checkers reject as `Final`

[SMP102](SMP102.html) checks the names of the same variables.

## Why is this bad?

Without `Final`, a reader has to search the codebase to learn that a module variable never changes. With it, the type
checker rejects any reassignment, here or in another module ([PEP 591](https://peps.python.org/pep-0591/)).

## Example

```python
TIMEOUT = 30
LIMIT: int = 10
```

Use instead:

```python
from typing import Final

TIMEOUT: Final[int] = 30
LIMIT: Final[int] = 10
```

## Fix safety

The fix is unsafe: the annotation lands in the module's `__annotations__`, which code reads at runtime, such as Sphinx
autodoc rendering the type or a dataclass-style library, and adding `Final` may also add a `typing` import. When the
variable has an annotation, the fix wraps it in `Final[...]`. Without one, the fix adds `Final[int]`, `Final[float]`,
`Final[str]`, `Final[bytes]` or `Final[bool]` when the value is a literal of that type; f-strings count as `str`. Any
other value, such as a call, a negative number or a container, gets no fix, because its type cannot be read from the
text.

To make `Final` available, the fix picks the first of these that applies:

1. Write `Final` when the module already binds it.
1. Write `typing.Final` when the module has `import typing`, or `t.Final` for `import typing as t`.
1. Add `Final` to the first `from typing import` statement, in the order Ruff's isort keeps: constants such as
   `TYPE_CHECKING` first, then classes, then the rest, each alphabetically.
1. Insert `from typing import Final` after the module docstring and
   [`__future__`](https://docs.python.org/3/library/__future__.html) imports, or before the first statement when there
   are none.

A `Final` imported only under [`if TYPE_CHECKING:`](https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING)
does not count as bound when annotations are evaluated eagerly, which is before Python 3.14 without
`from __future__ import annotations`. In that case the fix adds a runtime import.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
