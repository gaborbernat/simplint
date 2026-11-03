## What it does

Checks the `:returns:` and `:yields:` fields of reStructuredText docstrings, when `docstring-style = "rest"`:

- a docstring of a library's public API (see [SMP302](SMP302.html)) without `:returns:` for a function that returns a
  value: its return annotation is not `None`, `NoReturn` or `Never`, or, without one, it returns something other than
  `None`

- a docstring of a public generator without `:yields:`

- any docstring with `:returns:` for a function that returns nothing: its annotation says so, or, without one, it has no
  `return` with a value and its body is more than a placeholder such as `...`, as in an abstract method

- any docstring of a generator with `:returns:` and no `:yields:`

- any docstring of a function that is not a generator with `:yields:`

- a `:returns:`, `:rtype:`, `:yields:` or `:ytype:` field in the docstring of a class, which describes the class, or of
  a property, whose text describes the value it holds

`__init__` is skipped.

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

- for a function that returns nothing, the fix deletes the `:returns:` and `:rtype:` fields with the lines that continue
  them
- for a generator documented with `:returns:`, it renames the field to `:yields:`, keeping its description
- for a function that is not a generator, it deletes the `:yields:` and `:ytype:` fields
- for a missing field, it adds `:returns:` or `:yields:` after the parameter fields, with an empty description for you
  to write, which [SMP310](SMP310.html) reports until you do

Each is unsafe, since code may read the docstring through `__doc__`. No fix is offered when escapes in the docstring
make its text differ from the source, nor for a field of a class or property, whose text belongs in the description.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on

## Conflicts with Ruff

Ruff's [docstring-missing-returns (DOC201)](https://docs.astral.sh/ruff/rules/docstring-missing-returns/) and
[docstring-missing-yields (DOC402)](https://docs.astral.sh/ruff/rules/docstring-missing-yields/) read Google and
numpydoc sections only, so they report the `:returns:` and `:yields:` fields this rule asks for as missing sections.
simplint stops with an error before linting while either applies to a file this rule checks; turn them off in Ruff, or
turn this rule off.
