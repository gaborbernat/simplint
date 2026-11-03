## What it does

Checks the docstrings of a library's public API (see [SMP116](SMP116.html)), when `docstring-style = "rest"`, for
exceptions the function raises itself without a `:raises Error:` field. A field matches by the exception's last name, so
`:raises errors.ParseError:` documents `raise errors.ParseError(...)`. These are not counted:

- a bare `raise`, and a `raise` of an exception the function caught, as `raise error` in `except ... as error`
- an exception a surrounding `try` of the same function catches, by name or through `Exception` or `BaseException`
- `NotImplementedError`, which marks a method for subclasses to override
- a `raise` inside a nested function or class
- a name starting in lowercase, which is a variable rather than an exception class

Exceptions raised by the functions it calls are not counted, since the code does not show them.

## Why is this bad?

Callers handle the exceptions they know about. One the docstring leaves out reaches them as a surprise, often in
production, where the error path is first exercised.

## Example

```python
def parse(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :returns: The number.
    """
    if not text:
        raise ValueError("empty")
    return int(text)
```

Use instead:

```python
def parse(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :returns: The number.
    :raises ValueError: When the text is empty.
    """
    if not text:
        raise ValueError("empty")
    return int(text)
```

## Fix safety

No fix is offered, since only the author knows when the exception is raised.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on

## Conflicts with Ruff

Ruff's [docstring-missing-exception (DOC501)](https://docs.astral.sh/ruff/rules/docstring-missing-exception/) and
[docstring-extraneous-exception (DOC502)](https://docs.astral.sh/ruff/rules/docstring-extraneous-exception/) read
Google and numpydoc sections only; see the Configuration page for the check simplint runs before linting.
