Flags attributes of a library's public classes that the reStructuredText class docstring leaves without an `:ivar:`,
`:cvar:` or `:var:` field.

## Why is this bad?

A reader needs to know what an attribute holds and when it is available. Attributes can be documented in place or in the
class docstring. When asked to list class members,
[Sphinx autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-option-autoclass-undoc-members)
normally includes only documented public members.

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
`docstring-style = "rst"`, for undocumented public attributes. A `:vartype name:` field also counts, although
[rst-type-field](SMP912.html) reports types written in docstrings. Write the description by hand.

It also checks classes documented by the project's autodoc directives.

### Active on

- **Names the class body assigns**: simplint treats a `ClassVar`, a `Final` with an assigned value and an ordinary
  unannotated assignment as class variables. Other annotated names and declarations such as `attrs.field()` count as
  instance attributes. The rule also recognizes names in literal `__slots__` declarations.

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

- **Loop variables of the class body**: the target of a `for` statement in the class body is a class variable, as a name
  the body assigns without an annotation is, so the class documents it.

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

      for step in (1, 2):
          pass
  ```

- **Attributes a regular method sets on its first parameter**: these are instance attributes. A method sets them at any
  depth of its body, directly or through
  [`object.__setattr__(self, "name", value)`](https://docs.python.org/3/reference/datamodel.html#object.__setattr__) as
  a frozen dataclass does. A class defined inside a method has its own `self`, so its attributes belong to that class
  instead.

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

- **Attributes documented in place**: a `#:` comment immediately above the assignment or at the end of its line counts,
  as does a string as the next statement.

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

## Other tools

- [pydoclint DOC601](https://jsh9.github.io/pydoclint/violation_codes.html) checks whether a class docstring lists fewer
  attributes than the class defines. [Ruff issue #12434](https://github.com/astral-sh/ruff/issues/12434) tracks
  pydoclint checks.
