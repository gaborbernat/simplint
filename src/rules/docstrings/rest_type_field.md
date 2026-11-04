## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for type information, which belongs in annotations:
`:type name:`, `:rtype:`, `:vartype name:` and `:ytype:` fields, and the type in `:param type name:` or
`:ivar type name:`.

## Why is this bad?

An annotation is the type a type checker verifies; a type in the docstring is checked by nothing and drifts from the
code. Sphinx shows annotations itself, in the signature or, with
[`autodoc_typehints = "description"`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autodoc_typehints),
in the parameter list.

## Example

```python
def scale(value, factor):
    """Scale a value.

    :param float value: The value.
    :param factor: The factor.
    :type factor: float
    :rtype: float
    """
```

Use instead:

```python
def scale(value: float, factor: float) -> float:
    """Scale a value.

    :param value: The value.
    :param factor: The factor.
    """
```

## Fix safety

Where the signature already annotates what the field types, the fix deletes the `:type:`, `:rtype:` or `:ytype:` field
with the lines that continue it, or drops the type from a `:param type name:` field. It is unsafe, since code may read
the docstring through `__doc__`. Without the annotation no fix is offered, since the docstring's type, such as
`list of str`, may not be a valid annotation; nor is one offered for attribute types, nor when escapes in the docstring
make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
