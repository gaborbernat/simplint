Flags parameters that a public API docstring leaves without a `:param name:` field, when the project uses
reStructuredText docstrings.

## Why is this bad?

A signature can show that `timeout` takes a float and defaults to `5.0`. The docstring can tell callers whether that
means seconds per attempt or for the whole request. [PEP 257](https://peps.python.org/pep-0257/#multi-line-docstrings)
recommends documenting arguments, and
[Google's style guide](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) asks for enough
information to call the function without reading its implementation.

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

Write the description by hand. [rst-empty-field](SMP914.html) also checks that each field has text.

The rule also checks parameters on definitions the project's autodoc directives document.

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

- **Parameters such as `__obj`.** The rule treats these as public, following the
  [historical typing convention](https://typing.python.org/en/latest/spec/historical.html#positional-only-parameters)
  for positional-only parameters.

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

- **Parameters named with one leading underscore**: simplint treats these as internal. [rst-private-field](SMP915.html)
  reports documentation of them in the public API.

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

- **Generated constructors.** For dataclasses, attrs classes and named tuples, the rule can infer parameters from field
  declarations. It skips `ClassVar`, the `KW_ONLY` marker and fields declared with `init=False`. Quoted annotations such
  as `"ClassVar[int]"` count too.

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

- [Ruff D417](https://docs.astral.sh/ruff/rules/undocumented-param/) checks documented argument sections.
  [Ruff PR #21076](https://github.com/astral-sh/ruff/pull/21076) proposes a missing-parameter check.
  [Issue #6606](https://github.com/astral-sh/ruff/issues/6606) tracks Sphinx-style docstrings.
- [pydoclint DOC101](https://jsh9.github.io/pydoclint/violation_codes.html) checks for fewer parameters in the docstring
  than in the signature. [darglint DAR101](https://github.com/terrencepreilly/darglint) reports missing parameter
  descriptions.
