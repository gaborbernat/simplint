## What it does

Checks the docstrings of a library's public API (see [SMP116](SMP116.html)), when `docstring-style = "rest"`, for
parameters without a `:param name:` field. A class's constructor parameters may be documented in the class docstring
or in its `__init__` docstring. `*args` and `**kwargs` count as `args` and `kwargs`, written with or without their stars,
as `:param \*args:`. The instance or class a method receives first and names made only of underscores are skipped.

Docstrings outside the public API are not checked, since [SMP002](SMP002.html) asks for none there.

## Why is this bad?

Users read a parameter's meaning in the rendered reference or their editor's hover help. A parameter the docstring
skips shows with its name and type only, leaving its units, defaults and constraints unsaid.

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

No fix is offered, since only the author knows what a parameter means.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on

## Conflicts with Ruff

Ruff's [undocumented-param (D417)](https://docs.astral.sh/ruff/rules/undocumented-param/) reads Google-style sections
only; see the Configuration page for the check simplint runs before linting.
