## What it does

Checks the docstrings of classes, when `docstring-style = "rest"`, for `:ivar name:`, `:cvar name:`, `:var name:` and
`:vartype name:` fields naming an attribute the class does not have. The class has the attributes its body assigns, those
its methods set on `self`, and its methods and properties, which autodoc can render as attributes. A class with a base
class other than `object`, `ABC`, `Protocol`, `Generic`, `NamedTuple` or `TypedDict` is not checked, since the base may
define the attribute.

## Why is this bad?

A field for an attribute that is gone, often renamed or removed in a later change, documents state no instance has.

## Example

```python
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    :ivar queue: The jobs waiting.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

Use instead:

```python
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

## Fix safety

The fix deletes the field with the lines that continue it. It is unsafe, since code may read the docstring through
`__doc__`. No fix is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
