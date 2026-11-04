## What it does

Checks the docstrings of a library's public classes (see [SMP302](SMP302.html)), when `docstring-style = "rest"`, for
attributes without an `:ivar name:`, `:cvar name:` or `:var name:` field. The attributes are those
[autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html) documents:

- names the class body assigns: a `ClassVar` or a name assigned without an annotation is a class variable, an annotated
  name, such as a dataclass field, an instance attribute, and a class variable a method also sets on `self` is an
  instance attribute with a default
- attributes a method sets on its first parameter, at any depth of its body, directly or through
  `object.__setattr__(self, "name", value)` as a frozen dataclass does; a class method's are class variables, and a
  static method sets none

These are skipped:

- names starting with `_`
- an attribute a `#:` comment above it, or a string right after it, documents, the forms autodoc reads
- an attribute the constructor takes as a parameter of the same name, whose `:param:` field documents it and which
  [SMP312](SMP312.html) asks for

## Why is this bad?

Users read a class's attributes in the rendered reference, where autodoc lists only those with documentation. An
undocumented attribute is either invisible there or shown without saying what it holds.

## Example

```python
class Engine:
    """Run jobs.

    :param speed: How fast it runs.
    """

    limit = 3

    def __init__(self, speed: int) -> None:
        self.speed = speed
        self.state = "idle"
```

Use instead:

```python
class Engine:
    """Run jobs.

    :param speed: How fast it runs.
    :cvar limit: The most jobs it runs at once.
    :ivar state: What it is doing.
    """
```

## Fix safety

The fix adds a `:cvar name:` or `:ivar name:` field for each attribute, with an empty description for you to write,
which [SMP310](SMP310.html) reports until you do. The fields go after the parameters and before the return and exception
fields, in the order Sphinx renders them. The fix is unsafe, since code may read the docstring through `__doc__`. No fix
is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
