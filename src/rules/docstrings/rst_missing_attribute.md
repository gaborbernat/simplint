Flags attributes of a library's public classes that the reStructuredText class docstring leaves without an `:ivar:`,
`:cvar:` or `:var:` field. autodoc lists only documented attributes, so an undocumented one is invisible or unexplained
in the reference.

## Why is this bad?

Users read a class's attributes in the rendered reference, where autodoc lists only those with documentation. An
undocumented attribute is either invisible there or shown without saying what it holds.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
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

```python ok
class Engine:
    """Run jobs.

    :param speed: How fast it runs.
    :cvar limit: The most jobs it runs at once.
    :ivar state: What it is doing.
    """
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :param speed: How fast it runs.
      """

      def __init__(self, speed: int) -> None:
          self.speed = speed
          self.state = "idle"
  ```

## What it does

Checks the docstrings of a library's public classes (see [undocumented-public-api](SMP903.html)), when
`docstring-style = "rst"`, for attributes without an `:ivar name:`, `:cvar name:` or `:var name:` field. The attributes
are those [autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html) documents.

### Active on

- **Names the class body assigns**: a `ClassVar` or a name assigned without an annotation is a class variable; an
  annotated name, such as a dataclass field, is an instance attribute; a class variable a method also sets on `self` is
  an instance attribute with a default.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs."""

      limit = 3
      state: str
  ```

- **Attributes a regular method sets on its first parameter**: these are instance attributes. A method sets them at any
  depth of its body, directly or through `object.__setattr__(self, "name", value)` as a frozen dataclass does.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :param speed: How fast it runs.
      """

      def __init__(self, speed: int) -> None:
          self.speed = speed
          self.state = "idle"
  ```

- **Attributes a class method sets on its first parameter**: these are class variables, found the same way.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs."""

      @classmethod
      def configure(cls) -> None:
          cls.limit = 3
  ```

### Exceptions

- **Names starting with `_`**: a leading underscore marks the name as internal, and [rst-private-field](SMP915.html)
  reports documentation of internal names in the public API.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs."""

      _cache = {}
  ```

- **Attributes documented in place**: a `#:` comment above the attribute, or a string right after it, documents it in
  the forms autodoc reads.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs."""

      #: The most jobs it runs at once.
      limit = 3

      rate = 2
      """Jobs started per second."""
  ```

- **Attributes the constructor takes as a parameter of the same name**: the `:param:` field documents it, which
  [rst-missing-param](SMP916.html) asks for.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :param speed: How fast it runs.
      """

      def __init__(self, speed: int) -> None:
          self.speed = speed
  ```

### Fix

The fix adds a `:cvar name:` or `:ivar name:` field for each attribute, with an empty description for you to write,
which [rst-empty-field](SMP914.html) reports until you do. The fields go after the parameters and before the return and
exception fields, in the order Sphinx renders them.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[project]
name = "demo"

[build-system]
requires = []

[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :param speed: How fast it runs.
    """

    def __init__(self, speed: int) -> None:
        self.speed = speed
        self.state = "idle"
```

```python fixed
class Engine:
    """Run jobs.

    :param speed: How fast it runs.
    :ivar state:
    """

    def __init__(self, speed: int) -> None:
        self.speed = speed
        self.state = "idle"
```

No fix is offered when escapes in the docstring make its text differ from the source.
