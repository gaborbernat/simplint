Flags a constructor parameter documented by a `:param name:` field in both the class docstring and the `__init__`
docstring. Sphinx shows the class docstring by default, so the two copies drift apart as each is edited on its own.

## Why is this bad?

[autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autoclass_content) shows the class
docstring by default, so the copy in `__init__` is read only by those who open the source.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
```

Use instead:

```python ok
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up."""
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

## What it does

Checks classes, when `docstring-style = "rst"`, for constructor parameters documented twice.

### Active on

- **`:param name:` fields in the `__init__` docstring**: reported when the class docstring also has a `:param name:`
  field for the same constructor parameter.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str, mode: str) -> None:
          """Set up.

          :param mode: How to open it.
          """
  ```

### Fix

The fix deletes the field from the `__init__` docstring, with the lines that continue it.

**Unsafe fix:** code may read the docstring through `__doc__`, and the `__init__` text may hold details the class
docstring lacks.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
```

```python fixed
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up."""
```

No fix is offered when escapes in the docstring make its text differ from the source.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Open pull request
  [#23081](https://github.com/astral-sh/ruff/pull/23081) (DOC304) reports any parameters section in a Google or NumPy
  class docstring; this rule reports reST `:param:` fields documented in both the class and `__init__`.
- [pydoclint](https://github.com/jsh9/pydoclint) reports any `__init__` docstring as
  [DOC301](https://jsh9.github.io/pydoclint/violation_codes.html), not only the parameters documented in both places.
