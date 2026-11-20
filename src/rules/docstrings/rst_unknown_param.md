Flags `:param name:` and `:type name:` fields in reStructuredText docstrings that name a parameter the signature does
not take.

## Why is this bad?

A field left behind after a parameter is renamed tells readers they can still pass the old keyword. Python
[raises `TypeError`](https://docs.python.org/3/reference/expressions.html#calls) for an unexpected keyword unless the
function accepts extra keywords.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    :param mode: How to open it.
    """
```

Use instead:

```python ok
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    """
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def load(path: str) -> str:
      """Load a file.

      :param path: Where it is.
      :param mode: How to open it.
      """
      return path
  ```

## What it does

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for `:param name:` and `:type name:` fields naming a
parameter the signature does not take.

### Active on

- **Function and method docstrings**: fields are matched against the signature. The instance or class a method receives
  first does not count as a parameter.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def load(path: str) -> str:
      """Load a file.

      :param path: Where it is.
      :param mode: How to open it.
      """
      return path
  ```

  ```python fixed
  def load(path: str) -> str:
      """Load a file.

      :param path: Where it is.
      """
      return path
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files."""

      def read(self, size: int) -> str:
          """Read the file.

          :param self: The reader.
          :param size: Bytes to read.
          """
          return ""
  ```

- **Class docstrings.** The rule checks an explicit `__init__` signature or parameters inferred from dataclass, attrs or
  named tuple fields.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      :param mode: How to open it.
      """

      def __init__(self, source: str) -> None:
          self.source = source
  ```

- **Fields of an attrs class**: attrs names the parameter of a private field without its leading underscore, or by its
  `alias`, so `_x` takes `x`.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok file=point.py
  from attrs import define


  @define
  class Point:
      """A point on the horizontal axis.

      :param x: Where it sits.
      """

      _x: int
  ```

  ```python report file=alias.py
  from attrs import define, field


  @define
  class Point:
      """A point on the horizontal axis.

      :param x: Where it sits.
      """

      _x: int = field(alias="pos")
  ```

### Exceptions

- **A class without an explicit or recognized generated constructor**: the rule cannot determine its parameters. It also
  skips generated constructors whose bases may supply more fields.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Reader(Base):
      """Read files.

      :param mode: How to open it.
      """
  ```

- **An attrs class with a field whose `alias` is not a string literal**: the alias names the parameter, and simplint
  does not evaluate it, so the class's parameters are not known.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  from attrs import define, field

  POS = "pos"


  @define
  class Point:
      """A point on the horizontal axis.

      :param pos: Where it sits.
      """

      _x: int = field(alias=POS)
  ```

- **A function with `**kwargs`**: it takes any keyword, which such fields may document.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def load(path: str, **kwargs: str) -> str:
      """Load a file.

      :param path: Where it is.
      :param mode: How to open it.
      """
      return path
  ```

### Fix

The fix deletes the field with the lines that continue it.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    :param mode: How to open it.
    """
    return path
```

```python fixed
def load(path: str) -> str:
    """Load a file.

    :param path: Where it is.
    """
    return path
```

When the text left before the field ends in a quote or a backslash, the closing quotes go on a line of their own, since
on the same line they would end or escape the string.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
# fmt: off
def load(path: str) -> str:
    """Load "path"

    :param mode: How to open it.
    """
    return path
# fmt: on
```

```python fixed
# fmt: off
def load(path: str) -> str:
    """Load "path"
    """
    return path
# fmt: on
```

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def split(text: str) -> list[str]:
    r"""Split on \

    :param sep: Where to cut.
    """
    return text.split()
```

```python fixed
def split(text: str) -> list[str]:
    r"""Split on \
    """
    return text.split()
```

The field must start below the opening quotes in a single triple-quoted string whose text matches the source. If the
field ends the docstring, a comment after the closing quotes also prevents the fix.

## Other tools

- [Ruff DOC102](https://docs.astral.sh/ruff/rules/docstring-extraneous-parameter/) checks for extraneous parameters in
  Google and NumPy docstrings. [Ruff PR #13286](https://github.com/astral-sh/ruff/pull/13286) proposes support for
  Sphinx fields.
- [pydoclint DOC102](https://jsh9.github.io/pydoclint/violation_codes.html) checks for more parameters in the docstring
  than in the signature. [darglint DAR102](https://github.com/terrencepreilly/darglint) reports a documented parameter
  missing from the signature.
