## What it does

Checks classes, when `docstring-style = "rest"`, for a constructor parameter documented by a `:param name:` field in
both the class docstring and the `__init__` docstring.

## Why is this bad?

[autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autoclass_content) shows the class
docstring by default, so the copy in `__init__` is read by those who open the source only, and the two drift apart as
each is edited on its own.

## Example

```python
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
```

Use instead:

```python
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up."""
```

## Fix safety

The fix deletes the field from the `__init__` docstring, with the lines that continue it. It is unsafe, since code may
read the docstring through `__doc__`, and the `__init__` text may hold details the class docstring lacks. No fix is
offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
