Flags type information in reStructuredText docstrings, which belongs in annotations.

## Why is this bad?

Writing a parameter type in both its annotation and its docstring means a signature change can leave the reference out
of date. [Sphinx autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autodoc_typehints)
can display annotations in the signature or in the description. Other conventions differ.
[numpydoc](https://numpydoc.readthedocs.io/en/latest/format.html) requires types in Returns and Yields, and
[pydoclint](https://jsh9.github.io/pydoclint/config_options.html) expects argument types in docstrings by default.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def scale(value, factor):
    """Scale a value.

    :param float value: The value.
    :param factor: The factor.
    :type factor: float
    :rtype: float
    """
```

Use instead:

```python ok
def scale(value: float, factor: float) -> float:
    """Scale a value.

    :param value: The value.
    :param factor: The factor.
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
  def scale(value: float) -> float:
      """Scale a value.

      :rtype: float
      """
      return value
  ```

## What it does

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for type information.

### Active on

- **`:type name:`, `:rtype:` and `:ytype:` fields**: where the signature already annotates what the field types, the fix
  deletes the field with the lines that continue it.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def scale(value: float, factor: float) -> float:
      """Scale a value.

      :param value: The value.
      :type value: float
      :rtype: float
      """
      return value * factor
  ```

  ```python fixed
  def scale(value: float, factor: float) -> float:
      """Scale a value.

      :param value: The value.
      """
      return value * factor
  ```

- **The type in `:param type name:`**: where the signature already annotates the parameter, the fix drops the type from
  the field.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def scale(value: float, factor: float) -> float:
      """Scale a value.

      :param float value: The value.
      :param factor: The factor.
      """
      return value * factor
  ```

  ```python fixed
  def scale(value: float, factor: float) -> float:
      """Scale a value.

      :param value: The value.
      :param factor: The factor.
      """
      return value * factor
  ```

  The stars of `*args` and `**kwargs` stay in the name the field gives, and only the type goes.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def call(*args: int) -> None:
      r"""Call it.

      :param int \*args: The numbers.
      """
  ```

  ```python fixed
  def call(*args: int) -> None:
      r"""Call it.

      :param \*args: The numbers.
      """
  ```

- **Attribute types.** The rule reports `:vartype name:` and inline types in `:var:`, `:ivar:` or `:cvar:` fields. Add
  the annotation where the attribute is defined.

  - A `:vartype name:` field, here for a class attribute

    ```toml config
    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    class Engine:
        """Run jobs.

        :vartype limit: int
        """

        limit = 3
    ```

  - The type in `:ivar type name:`, here for an attribute set in `__init__`

    ```toml config
    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    class Engine:
        """Run jobs.

        :ivar int state: What it is doing.
        """

        def __init__(self) -> None:
            self.state = "idle"
    ```

### Fix

**Unsafe fix:** code may read the docstring through `__doc__`.

No fix is offered:

- without the annotation, since the docstring's type, such as `list of str`, may not be a valid annotation

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def join(values):
      """Join the strings.

      :param list of str values: The values.
      """
      return "".join(values)
  ```

- for attribute types

- for a field beside the opening quotes, or a docstring other than a single triple-quoted string whose text matches the
  source

- for a deletion at the end of a docstring with a comment after its closing quotes

## Other tools

- [Ruff issue #9070](https://github.com/astral-sh/ruff/issues/9070) proposes a fix to remove types from docstrings.
