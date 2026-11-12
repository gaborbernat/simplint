Flags attributes of a library's public classes that the reStructuredText class docstring leaves without an `:ivar:`,
`:cvar:` or `:var:` field.

## Why is this bad?

Users read a class's attributes in the rendered reference, where autodoc by default
[lists only members with a docstring or `#:` comment](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-option-autoclass-undoc-members).
An undocumented attribute is either missing there or, with `:undoc-members:`, shown without saying what it holds. The
[Google style guide](https://google.github.io/styleguide/pyguide.html#384-classes) and
[numpydoc](https://numpydoc.readthedocs.io/en/latest/format.html) both list a class's public attributes in its
docstring, and pydoclint [checks that list against the class body](https://jsh9.github.io/pydoclint/config_options.html)
by default.

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

No fix is offered: the description is for the author to write, and [rst-empty-field](SMP914.html) reports the empty
field until they do.

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

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#12434](https://github.com/astral-sh/ruff/issues/12434).
- [pydoclint](https://github.com/jsh9/pydoclint) reports a class docstring that lists fewer attributes than the class
  has as [DOC601](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
