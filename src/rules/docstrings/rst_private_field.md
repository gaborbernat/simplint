Flags fields in public API docstrings that document a parameter or attribute named with a leading underscore.

## Why is this bad?

Publishing a field for `_retries` can lead callers to depend on an internal setting.
[PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) uses a leading underscore to mark an internal
interface. [rst-missing-param](SMP916.html) and [rst-missing-attribute](SMP920.html) skip those names.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
def fetch(url: str, _retries: int = 3) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
    :param _retries: How often to retry, for the test suite.
    """
```

Use instead:

```python ok
def fetch(url: str, _retries: int = 3) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
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
  def fetch(_retries: int) -> None:
      """Download a resource.

      :param _retries: How often to retry.
      """
  ```

## What it does

Checks the docstrings of a library's public API (see [undocumented-public-api](SMP903.html)), when
`docstring-style = "rst"`, for fields naming a parameter or attribute with a leading underscore.

It also checks definitions documented by the project's autodoc directives.

### Active on

- **`:param:`, `:type:`, `:ivar:`, `:cvar:`, `:var:` and `:vartype:` fields in public API docstrings**: reported when
  the name they document starts with `_`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def fetch(url: str, _retries: int) -> bytes:
      """Download a resource.

      :param url: Address of the resource.
      :param _retries: How often to retry.
      """
      return b""
  ```

- **The `__init__` docstring of a public class**: it documents the constructor as the class docstring may, so its fields
  are checked too.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Client:
      """Talk to the service.

      :param url: Address of the service.
      """

      def __init__(self, url: str, _timeout: float) -> None:
          """Set up the client.

          :param _timeout: Seconds to wait.
          """
  ```

### Exceptions

- **Dunder names, such as `__wrapped__`**: a name with double leading and trailing underscores is one of Python's
  special names ([PEP 8](https://peps.python.org/pep-0008/#descriptive-naming-styles)) rather than an internal one, so
  it counts as public.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def wrap(__wrapped__: int) -> int:
      """Wrap a value.

      :param __wrapped__: The value.
      """
      return __wrapped__
  ```

- **Parameters named with two leading underscores, such as `__obj`**: by the
  [typeshed convention](https://typing.python.org/en/latest/spec/historical.html#positional-only-parameters) type
  checkers can interpret these names as positional-only. simplint allows them in parameter fields.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def wrap(__obj: int) -> int:
      """Wrap a value.

      :param __obj: The value.
      """
      return __obj
  ```

### Fix

The fix deletes the field with the lines that continue it.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[project]
name = "demo"

[build-system]
requires = []

[tool.simplint]
docstring-style = "rst"
```

```python report
def fetch(url: str, _retries: int) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
    :param _retries: How often to retry.
    """
    return b""
```

```python fixed
def fetch(url: str, _retries: int) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
    """
    return b""
```

The field must start below the opening quotes in a single triple-quoted string whose text matches the source. If the
field ends the docstring, a comment after the closing quotes also prevents the fix.
