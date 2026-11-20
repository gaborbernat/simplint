Flags `:param:`, `:raises:`, `:returns:`, `:yields:` and `:var:` fields in reStructuredText docstrings that have nothing
after their name, such as `:param path:` with no description.

## Why is this bad?

An empty entry such as `:param timeout:` leaves readers guessing the units and what happens when the timeout expires.
[PEP 257](https://peps.python.org/pep-0257/#multi-line-docstrings) calls for documenting arguments, return values and
exceptions where applicable.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def read(path: str) -> str:
    """Read a file.

    :param path:
    """
    return path
```

Use instead:

```python ok
def read(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    """
    return path
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :param path:
      """
      return path
  ```

## What it does

Checks the field lists of reStructuredText docstrings, when `docstring-style = "rst"`, for fields with nothing after
their name. No fix is offered, since only the author knows what the field should say.

### Active on

- **Fields with no description**: a `:param:`, `:raises:`, `:returns:`, `:yields:` or `:var:` field with nothing after
  the name or on any indented continuation line.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :param path:
      """
      return path
  ```

### Exceptions

- **Malformed fields**: a field that [rst-field-syntax](SMP910.html) reports, such as an unnamed `:param:`, is left to
  it, since the field must be fixed before its description can be judged.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      :param:
      """
      return path
  ```

## Other tools

- [Ruff D417](https://docs.astral.sh/ruff/rules/undocumented-param/) checks for undocumented parameters in argument
  sections. [Ruff issue #14494](https://github.com/astral-sh/ruff/issues/14494) asks for a check on empty exception
  descriptions.
