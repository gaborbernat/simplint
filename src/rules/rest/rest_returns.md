## What it does

Checks the `:returns:` and `:yields:` fields of reStructuredText docstrings, when `docstring-style = "rest"`:

- a docstring of a library's public API (see [SMP116](SMP116.html)) without `:returns:` for a function that returns a
  value: its return annotation is not `None`, `NoReturn` or `Never`, or, without one, it returns something other than
  `None`
- a docstring of a public generator without `:yields:`
- any docstring with `:returns:` for a function that returns nothing: its annotation says so, or, without one, it has
  no `return` with a value and its body is more than a placeholder such as `...`, as in an abstract method
- any docstring of a generator with `:returns:` and no `:yields:`

Properties, whose docstring describes the value they hold, and `__init__` are skipped.

## Why is this bad?

What a function hands back is half of its contract. A missing field leaves it to the annotation alone; a `:returns:` on
a function that returns nothing, or on a generator, describes a value no caller receives.

## Example

```python
def total(values: list[int]) -> int:
    """Add the values.

    :param values: The values.
    """
    return sum(values)
```

Use instead:

```python
def total(values: list[int]) -> int:
    """Add the values.

    :param values: The values.
    :returns: Their sum.
    """
    return sum(values)
```

## Fix safety

For a function that returns nothing, the fix deletes the `:returns:` and `:rtype:` fields with the lines that continue
them. It is unsafe, since code may read the docstring through `__doc__`. No fix is offered when escapes in the docstring
make its text differ from the source, nor for the other reports.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on

## Conflicts with Ruff

Ruff's [docstring-missing-returns (DOC201)](https://docs.astral.sh/ruff/rules/docstring-missing-returns/),
[docstring-extraneous-returns (DOC202)](https://docs.astral.sh/ruff/rules/docstring-extraneous-returns/),
[docstring-missing-yields (DOC402)](https://docs.astral.sh/ruff/rules/docstring-missing-yields/) and
[docstring-extraneous-yields (DOC403)](https://docs.astral.sh/ruff/rules/docstring-extraneous-yields/) read Google and
numpydoc sections only; see the Configuration page for the check simplint runs before linting.
