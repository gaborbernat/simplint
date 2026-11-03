## What it does

Checks the docstrings of a library's public API (see [SMP302](SMP302.html)), when `docstring-style = "rest"`, for
parameters without a `:param name:` field. A class's constructor parameters may be documented in the class docstring or
in its `__init__` docstring. `*args` and `**kwargs` count as `args` and `kwargs`, written with or without their stars,
as `:param \*args:`. The instance or class a method receives first and names with a leading underscore, which are
internal, are skipped; [SMP323](SMP323.html) reports documentation of the latter in the public API.

Docstrings outside the public API are not checked, since [SMP300](SMP300.html) asks for none there.

## Why is this bad?

Users read a parameter's meaning in the rendered reference or their editor's hover help. A parameter the docstring skips
shows with its name and type only, leaving its units, defaults and constraints unsaid.

## Example

```python
def copy(source: str, target: str) -> None:
    """Copy a file.

    :param source: Where from.
    """
```

Use instead:

```python
def copy(source: str, target: str) -> None:
    """Copy a file.

    :param source: Where from.
    :param target: Where to.
    """
```

## Fix safety

The fix adds a `:param name:` field for each parameter, with an empty description for you to write, which
[SMP310](SMP310.html) reports until you do. Each goes before the first field of a later parameter, or after the last
parameter field, and a docstring without fields gets a field list after a blank line; for a class, the fields go to the
`__init__` docstring when it documents parameters already. The fix is unsafe, since code may read the docstring through
`__doc__`. No fix is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
