Flags type information in reStructuredText docstrings, which belongs in annotations.

## Why is this bad?

An annotation is the type a type checker verifies. A type written in the docstring repeats it and can drift from the
code. Sphinx shows annotations itself, in the signature or, with
[`autodoc_typehints = "description"`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autodoc_typehints),
in the parameter list. The
[Google style guide](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) asks for a type in the
description only where the code has no annotation. [numpydoc](https://numpydoc.readthedocs.io/en/latest/format.html)
requires types in Returns and Yields, and [pydoclint](https://jsh9.github.io/pydoclint/config_options.html) requires
argument types in docstrings by default.

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

- **`:vartype name:` fields and the type in `:ivar type name:`**: attribute types are reported but get no fix.

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

- when escapes in the docstring make its text differ from the source

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#12434](https://github.com/astral-sh/ruff/issues/12434)
  and [#9070](https://github.com/astral-sh/ruff/issues/9070).
