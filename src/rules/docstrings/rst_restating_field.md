Flags `:param:`, `:ivar:`, `:cvar:` and `:var:` fields in reStructuredText docstrings whose description only repeats the
name, as `:param user_id: The user ID.`. Such a description tells readers nothing the signature does not.

## Why is this bad?

Such a description takes the space of a useful one while telling readers nothing the signature does not: not the unit,
the accepted range, what `None` means, or how the value is used. It is often what fills a field added only to satisfy a
check.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def fetch(url: str, timeout: float) -> bytes:
    """Download a resource.

    :param url: The URL.
    :param timeout: The timeout.
    """
```

Use instead:

```python ok
def fetch(url: str, timeout: float) -> bytes:
    """Download a resource.

    :param url: Address of the resource, with its scheme.
    :param timeout: Seconds to wait for the first byte.
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
  def fetch(url: str) -> bytes:
      """Download a resource.

      :param url: The URL.
      """
      return b""
  ```

## What it does

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for field descriptions that only repeat the name. No
fix is offered, since only the author knows what the value means.

### Active on

- **`:param:` fields**: reported when every word of the description, beyond articles and words such as "parameter" or
  "value", is a word of the parameter's name.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def fetch(url: str, timeout: float) -> bytes:
      """Download a resource.

      :param url: The URL.
      :param timeout: The timeout.
      """
      return b""
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def fetch(url: str, timeout: float) -> bytes:
      """Download a resource.

      :param url: Address of the resource, with its scheme.
      :param timeout: Seconds to wait for the first byte.
      """
      return b""
  ```

- **`:ivar:`, `:cvar:` and `:var:` fields**: reported the same way for the attribute's name.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar limit: The limit.
      """

      def __init__(self, limit: int) -> None:
          self.limit = limit
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
