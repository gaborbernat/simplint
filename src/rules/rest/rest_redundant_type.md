## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for types that repeat the signature's annotations:
a `:type name:` field or a `:param type name:` field for an annotated parameter, and an `:rtype:` field for a function
with a return annotation.

## Why is this bad?

The annotation is the type a type checker verifies; a copy in the docstring is checked by nothing and drifts when the
annotation changes. Sphinx shows the annotations itself, in the signature or, with
[`autodoc_typehints = "description"`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autodoc_typehints),
in the parameter list.

## Example

```python
def scale(value: float) -> float:
    """Scale a value.

    :param float value: The value.
    :rtype: float
    """
```

Use instead:

```python
def scale(value: float) -> float:
    """Scale a value.

    :param value: The value.
    """
```

## Fix safety

The fix deletes a `:type:` or `:rtype:` field with the lines that continue it, and drops the type from a
`:param type name:` field. It is unsafe, since code may read the docstring through `__doc__`. No fix is offered when
escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
