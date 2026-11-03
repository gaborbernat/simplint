## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for `:param name:` and `:type name:` fields naming
a parameter the signature does not take. A class's docstring documents the parameters of its `__init__`, or of the
fields a dataclass or attrs class turns into them; a class that takes its constructor from a base class is not checked,
nor is a function with `**kwargs`, whose keywords such fields may document. The instance or class a method receives
first does not count as a parameter.

## Why is this bad?

A field for a parameter that is gone, often renamed or removed in a later change, documents a call that no longer works.

## Example

```python
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    :param mode: How to open it.
    """
```

Use instead:

```python
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    """
```

## Fix safety

The fix deletes the field with the lines that continue it. It is unsafe, since code may read the docstring through
`__doc__`. No fix is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on

## Conflicts with Ruff

Ruff's [docstring-extraneous-parameter (DOC102)](https://docs.astral.sh/ruff/rules/docstring-extraneous-parameter/)
reads Google and numpydoc sections only; see the Configuration page for the check simplint runs before linting.
