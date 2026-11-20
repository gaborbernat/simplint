Flags `:param:`, `:ivar:`, `:cvar:` and `:var:` fields in reStructuredText docstrings whose description only repeats the
name, as `:param user_id: The user ID.`.

## Why is this bad?

“The user ID” tells a caller nothing beyond the parameter name `user_id`. State which system issued the ID, for example,
or what an absent value means.
[Google's style guide](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) recommends enough
detail to call the function without reading its code.

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

- **`:param:` fields.** The rule splits the parameter name at underscores and compares words without regard to case. It
  ignores common words such as “the”, “parameter” and “value”. It reports a description if at least one word remains and
  every remaining word appears in the name.

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
