Flags parameters that a public API docstring leaves without a `:param name:` field, when the project uses
reStructuredText docstrings. Without the field, the reference shows only the parameter's name and type.

## Why is this bad?

Users read a parameter's meaning in the rendered reference or their editor's hover help. A parameter the docstring skips
shows with its name and type only, leaving its units, defaults and constraints unsaid.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
def copy(source: str, target: str) -> None:
    """Copy a file.

    :param source: Where from.
    """
```

Use instead:

```python ok
def copy(source: str, target: str) -> None:
    """Copy a file.

    :param source: Where from.
    :param target: Where to.
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
  def copy(source: str, target: str) -> None:
      """Copy a file.

      :param source: Where from.
      """
  ```

## What it does

Checks the docstrings of a library's public API (see [undocumented-public-api](SMP903.html)), when
`docstring-style = "rst"`, for parameters without a `:param name:` field.

No fix is offered: the description is for the author to write, and [rst-empty-field](SMP914.html) reports the empty
field until they do.

### Active on

- **Parameters of public functions and methods**: each needs a `:param name:` field.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def copy(source: str, target: str) -> None:
      """Copy a file.

      :param source: Where from.
      """
  ```

- **Constructor parameters of a public class**: they may be documented in the class docstring or in its `__init__`
  docstring.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str, encoding: str) -> None:
          """Set up."""
  ```

- **Fields of an attrs class**: attrs names the constructor parameter of a private field without its leading underscore,
  or by its `alias`, so the docstring documents that name.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  from attrs import define


  @define
  class Point:
      """A point on the horizontal axis."""

      _x: int
  ```

- **`*args` and `**kwargs`**: they count as `args` and `kwargs`, written with or without their stars, as
  `:param \*args:`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def copy(source: str, *args: str, **kwargs: str) -> None:
      """Copy a file.

      :param source: Where from.
      """
  ```

- **Parameters named with two leading underscores, such as `__obj`**: the double underscore marks the parameter
  positional-only rather than internal, so it is checked.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def wrap(__obj: int) -> int:
      """Wrap a value."""
      return __obj
  ```

### Exceptions

- **The instance or class a method receives first**: callers do not pass it, so there is nothing to document.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Reader:
      """Read files."""

      def read(self, size: int) -> str:
          """Read the file.

          :param size: Bytes to read.
          """
          return ""
  ```

- **Parameters named with one leading underscore**: they are internal, a knob the documentation leaves out;
  [rst-private-field](SMP915.html) reports documentation of them in the public API.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def fetch(url: str, _retries: int = 3) -> bytes:
      """Download a resource.

      :param url: Address of the resource.
      """
      return b""
  ```

- **Docstrings outside the public API**: [function-docstring](SMP901.html) asks for no docstring there.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok file=demo/_core.py
  def copy(source: str, target: str) -> None:
      """Copy a file.

      :param source: Where from.
      """
  ```

- **Class variables of a generated class**: a `ClassVar` field is no constructor parameter, whether its annotation is
  written as `ClassVar[int]` or as the quoted string `"ClassVar[int]"`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  from dataclasses import dataclass
  from typing import ClassVar


  @dataclass
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      source: str
      count: "ClassVar[int]" = 0
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#12434](https://github.com/astral-sh/ruff/issues/12434)
  (DOC101) and [#6606](https://github.com/astral-sh/ruff/issues/6606) (Sphinx style). Released rule
  [D417](https://docs.astral.sh/ruff/rules/undocumented-param/) and open pull request
  [#21076](https://github.com/astral-sh/ruff/pull/21076) check only Google and NumPy sections, not `:param` fields.
- [pydoclint](https://github.com/jsh9/pydoclint) reports a docstring with fewer parameters than the signature as
  [DOC101](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
- [darglint](https://github.com/terrencepreilly/darglint) reports a missing parameter as
  [DAR101](https://github.com/terrencepreilly/darglint), with no fix.
