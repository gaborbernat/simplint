Flags `:param:` fields in a reStructuredText docstring that are in a different order from the signature's parameters.

## Why is this bad?

When parameter descriptions follow the signature, a reader can match each positional argument to its description while
reading a call. [pydoclint](https://jsh9.github.io/pydoclint/config_options.html) checks this order by default.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def move(source: str, target: str) -> None:
    """Move a file.

    :param target: Where to.
    :param source: Where from.
    """
```

Use instead:

```python ok
def move(source: str, target: str) -> None:
    """Move a file.

    :param source: Where from.
    :param target: Where to.
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
  def move(source: str, target: str) -> None:
      """Move a file.

      :param target: Where to.
      :param source: Where from.
      """
  ```

## What it does

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for the order of their `:param:` fields. It reports
the first pair of documented parameters that appear in reverse order. Move the field and any indented continuation lines
together.

### Active on

- **`:param:` fields for parameters the signature takes**: reported when their order differs from the signature's.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def move(source: str, target: str) -> None:
      """Move a file.

      :param target: Where to.
      :param source: Where from.
      """
  ```

### Exceptions

- **Fields for names the signature does not take**: these are left to [rst-unknown-param](SMP917.html), which reports
  them.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def move(source: str, target: str) -> None:
      """Move a file.

      :param gone: Removed.
      :param source: Where from.
      :param target: Where to.
      """
  ```

## Other tools

- [pydoclint DOC104](https://jsh9.github.io/pydoclint/violation_codes.html) checks argument order.
  [Ruff PR #23047](https://github.com/astral-sh/ruff/pull/23047) proposes DOC104 for Google and NumPy docstrings.
