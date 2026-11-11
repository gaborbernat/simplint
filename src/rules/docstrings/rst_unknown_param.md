Flags `:param name:` and `:type name:` fields in reStructuredText docstrings that name a parameter the signature does
not take.

## Why is this bad?

A field for a parameter that is gone, often renamed or removed in a later change, documents a call that no longer works.

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

- **Class docstrings**: a class's docstring documents the parameters of its `__init__`, or of the fields a dataclass or
  attrs class turns into them.

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

- **A class that takes its constructor from a base class**: the base's signature is not in the class, so its parameters
  are not known.

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

No fix is offered when escapes in the docstring make its text differ from the source.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of preview rule
  [DOC102](https://docs.astral.sh/ruff/rules/docstring-extraneous-parameter/), which checks Google and NumPy sections
  but not reST fields; open pull request [#13286](https://github.com/astral-sh/ruff/pull/13286) would add them.
- [pydoclint](https://github.com/jsh9/pydoclint) reports a docstring with more parameters than the signature as
  [DOC102](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
- [darglint](https://github.com/terrencepreilly/darglint) reports a docstring parameter the signature lacks as
  [DAR102](https://github.com/terrencepreilly/darglint), with no fix.
