## What it does

Checks the docstrings of classes, when `docstring-style = "rest"`, for an `:ivar name:` field documenting a class
variable, or a `:cvar name:` field documenting an instance attribute. A class variable is a `ClassVar`, or a name the
class body assigns without an annotation that no method sets on `self`; an instance attribute is an annotated field, or
a name a method sets on `self`. A class default that methods also set on `self` takes either field, and `:var:` suits
both.

## Why is this bad?

Sphinx renders the two as "Instance Variables" and "Class Variables", so the wrong one tells readers to look for the
value in the wrong place.

## Example

```python
class Engine:
    """Run jobs.

    :ivar limit: The most jobs it runs at once.
    """

    limit = 3
```

Use instead:

```python
class Engine:
    """Run jobs.

    :cvar limit: The most jobs it runs at once.
    """

    limit = 3
```

## Fix safety

The fix renames the field, keeping its description. It is unsafe, since code may read the docstring through `__doc__`.
No fix is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
