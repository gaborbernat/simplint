## What it does

Checks for `object` in annotations of parameters, return values and variables, including nested uses such as
`dict[str, object]`. Both `object` and `builtins.object` match. String annotations are not checked.

Parameters of these functions are exempt, since their signatures often have to accept any value:

- dunder functions and methods, such as `__eq__(self, other: object)`
- functions that return [`TypeIs[...]`](https://docs.python.org/3/library/typing.html#typing.TypeIs)
  ([PEP 742](https://peps.python.org/pep-0742/)) or
  [`TypeGuard[...]`](https://docs.python.org/3/library/typing.html#typing.TypeGuard)
  ([PEP 647](https://peps.python.org/pep-0647/))

Their return annotations are still checked.

Ruff's [any-type](https://docs.astral.sh/ruff/rules/any-type/) (ANN401) rejects `Any` in function annotations. This rule
covers `object`, the other annotation that accepts anything.

## Why is this bad?

`object` accepts anything and permits almost no operations, so the code behind it narrows with `isinstance` or casts at
runtime. A precise type moves that check to the type checker and documents what callers pass.

## Example

```python
def describe(value: object) -> str:
    return str(value)
```

Use instead:

```python
def describe(value: int | str) -> str:
    return str(value)
```

## Fix safety

No fix is offered: the precise type depends on what callers pass.
