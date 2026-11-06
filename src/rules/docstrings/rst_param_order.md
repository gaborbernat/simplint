Flags `:param:` fields in a reStructuredText docstring that are in a different order from the signature's parameters.
Readers match the fields against the signature from left to right, so a different order slows them down and hides a
missing field.

## Why is this bad?

A reader matches the documented parameters against the signature from left to right; a different order makes the
reference harder to scan and hides a missing field.

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

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for the order of their `:param:` fields. No fix is
offered, since moving fields can split the lines that continue them from their text.

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

- [pydoclint](https://github.com/jsh9/pydoclint) reports arguments documented in a different order than the signature as
  [DOC104](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
