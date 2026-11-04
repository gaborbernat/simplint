Flags `:ivar:`, `:cvar:`, `:var:` and `:vartype:` fields in a reStructuredText class docstring that name an attribute
the class does not have. Such a field, often left behind by a rename or removal, documents state no instance has.

## Why is this bad?

A field for an attribute that is gone, often renamed or removed in a later change, documents state no instance has.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    :ivar queue: The jobs waiting.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

Use instead:

```python ok
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    """

    def __init__(self) -> None:
        self.state = "idle"
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

      :cvar limit: The most jobs it runs at once.
      """

      state = "idle"
  ```

## What it does

Checks the docstrings of classes, when `docstring-style = "rst"`, for `:ivar name:`, `:cvar name:`, `:var name:` and
`:vartype name:` fields naming an attribute the class does not have.

### Active on

- **Attribute fields in a class docstring**: reported when the name is none of the attributes the class has: those its
  body assigns, those its methods set on `self`, and its methods and properties, which autodoc can render as attributes.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar state: What it is doing.
      :ivar queue: The jobs waiting.
      """

      def __init__(self) -> None:
          self.state = "idle"
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :ivar run: The entry point.
      """

      def run(self) -> None:
          print()
  ```

### Exceptions

- **A class with a base class other than `object`, `ABC`, `Protocol`, `Generic`, `NamedTuple` or `TypedDict`**: the base
  may define the attribute.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine(Base):
      """Run jobs.

      :ivar queue: The jobs waiting.
      """
  ```

### Fix

The fix deletes the field with the lines that continue it.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    :ivar queue: The jobs waiting.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

```python fixed
class Engine:
    """Run jobs.

    :ivar state: What it is doing.
    """

    def __init__(self) -> None:
        self.state = "idle"
```

No fix is offered when escapes in the docstring make its text differ from the source.
