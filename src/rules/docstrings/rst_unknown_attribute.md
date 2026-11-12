Flags `:ivar:`, `:cvar:`, `:var:` and `:vartype:` fields in a reStructuredText class docstring that name an attribute
the class does not have.

## Why is this bad?

A field for an attribute that is gone, often renamed or removed in a later change, documents state no instance has.
Neither Sphinx nor a type checker compares the field with the class, so it stays until a reader relies on it.
[PEP 8](https://peps.python.org/pep-0008/#comments) says comments that contradict the code are worse than no comments.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    :ivar queue: The jobs waiting.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

Use instead:

```python ok
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :cvar limit: The most jobs it runs at once.
      """

      state = "idle"
  ```

## What it does

Checks the docstrings of classes, when `docstring-style = "rst"`, for `:ivar name:`, `:cvar name:`, `:var name:` and
`:vartype name:` fields naming an attribute the class does not have.

### Active on

- **Attribute fields in a class docstring**: reported when the name is none of the attributes the class has: those its
  body assigns, including names unpacked from a tuple and the strings of a `__slots__`, those its methods set on `self`,
  and its methods and properties, which autodoc can render as attributes.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar state: What it is doing.
      :ivar queue: The jobs waiting.
      """

      def __init__(self) -> None:
          self.state = "idle"
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :ivar run: The entry point.
      """

      def run(self) -> None:
          print()
  ```

- **Type aliases in the class body**: a `type` statement there is a class attribute, so a `:cvar` field may document it.

  ```toml config
  [project]
  requires-python = ">=3.12"

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :cvar Point: The pair of coordinates.
      """

      type Point = tuple[int, int]
  ```

### Exceptions

- **A class with a base class other than `object`, `ABC`, `Protocol`, `Generic`, `NamedTuple` or `TypedDict`**: the base
  may define the attribute.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine(Base):
      """Run jobs.

      :ivar queue: The jobs waiting.
      """
  ```

- **Names a `__slots__` declares**: the strings of a string, tuple, list or set, and the keys of a dict, whether the
  assignment is annotated or not.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Named:
      """A name.

      :ivar text: The text.
      """

      __slots__ = "text"


  class Pair:
      """Two names.

      :ivar first: The first.
      :ivar second: The second.
      """

      __slots__ = ("first", "second")


  class Listed:
      """A list of names.

      :ivar third: The third.
      """

      __slots__ = ["third"]


  class Stored:
      """A set of names.

      :ivar fourth: The fourth.
      """

      __slots__ = {"fourth"}


  class Keyed:
      """Keys.

      :ivar key: The key.
      """

      __slots__ = {"key": "The key."}


  class Annotated:
      """Annotated slots.

      :ivar size: The size.
      """

      __slots__: tuple[str, ...] = ("size",)
  ```

- **Names unpacked into the class body**: each name a tuple or starred target binds, such as `low` and `high` in
  `low, high = 0, 10`, or `tail` in `head, *tail = values`, and the target of a `for` statement in the class body.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Bounds:
      """Limits of a range.

      :cvar low: The lower limit.
      :cvar high: The upper limit.
      :cvar tail: The rest.
      :cvar step: The distance between them.
      """

      low, high = 0, 10
      head, *tail = 1, 2, 3
      for step in (1, 2):
          pass
  ```

- **Members under `if` or `try` in the class body**: a method or assignment there is a member of the class, as one that
  a version check picks.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  import sys


  class Engine:
      """Run jobs.

      :ivar run: The entry point.
      """

      if sys.version_info >= (3, 12):

          def run(self) -> None:
              print()

      else:

          def run(self) -> None:
              print()
  ```

### Fix

The fix deletes the field with the lines that continue it.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    :ivar queue: The jobs waiting.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

```python fixed
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

No fix is offered when escapes in the docstring make its text differ from the source.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#12434](https://github.com/astral-sh/ruff/issues/12434).
- [pydoclint](https://github.com/jsh9/pydoclint) reports a class docstring that lists more attributes than the class has
  as [DOC602](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
