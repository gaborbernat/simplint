Flags an `:ivar:` field documenting a class variable, or a `:cvar:` field documenting an instance attribute, in
reStructuredText class docstrings. Sphinx renders the two under different headings, so the wrong field sends readers to
the wrong place.

## Why is this bad?

Sphinx renders the two as "Instance Variables" and "Class Variables", so the wrong one tells readers to look for the
value in the wrong place.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar limit: The most jobs it runs at once.
    """

    limit = 3
```

Use instead:

```python ok
class Engine:
    """Run jobs.

    :cvar limit: The most jobs it runs at once.
    """

    limit = 3
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

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3
  ```

## What it does

Checks the docstrings of classes, when `docstring-style = "rst"`, for attribute fields of the wrong kind.

### Active on

- **`:ivar name:` fields**: reported when `name` is a class variable.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3
  ```

- **`:cvar name:` fields**: reported when `name` is an instance attribute.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :cvar state: What it is doing.
      """

      def __init__(self) -> None:
          self.state = "idle"
  ```

### Exceptions

- **`:var name:` fields**: `:var:` suits both kinds.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :var limit: The most jobs it runs at once.
      """

      limit = 3
  ```

- **A class default that methods also set on `self`**: the attribute is both kinds, so `:ivar:` and `:cvar:` both fit.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3

      def reset(self) -> None:
          self.limit = 3
  ```

### Attribute kinds

| Attribute                                                                          | Kind               | Field                |
| ---------------------------------------------------------------------------------- | ------------------ | -------------------- |
| a `ClassVar`                                                                       | class variable     | `:cvar:`             |
| a `Final` with a value in the class body, as `SHOW: Final = 1`                     | class variable     | `:cvar:`             |
| a name the class body assigns without an annotation, that no method sets on `self` | class variable     | `:cvar:`             |
| an annotated field                                                                 | instance attribute | `:ivar:`             |
| a name a method sets on `self`                                                     | instance attribute | `:ivar:`             |
| a class default that methods also set on `self`                                    | both               | `:ivar:` or `:cvar:` |

### Fix

The fix renames the field, keeping its description.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar limit: The most jobs it runs at once.
    """

    limit = 3
```

```python fixed
class Engine:
    """Run jobs.

    :cvar limit: The most jobs it runs at once.
    """

    limit = 3
```

No fix is offered when escapes in the docstring make its text differ from the source.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
