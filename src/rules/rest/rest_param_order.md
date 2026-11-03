## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for `:param:` fields in a different order from the
signature's parameters. Fields for names the signature does not take are left to [SMP301](SMP301.html).

## Why is this bad?

A reader matches the documented parameters against the signature from left to right; a different order makes the
reference harder to scan and hides a missing field.

## Example

```python
def move(source: str, target: str) -> None:
    """Move a file.

    :param target: Where to.
    :param source: Where from.
    """
```

Use instead:

```python
def move(source: str, target: str) -> None:
    """Move a file.

    :param source: Where from.
    :param target: Where to.
    """
```

## Fix safety

No fix is offered, since moving fields can split the lines that continue them from their text.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
