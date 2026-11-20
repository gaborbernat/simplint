Flags an `:ivar:` field documenting a class variable, or a `:cvar:` field documenting an instance attribute, in
reStructuredText class docstrings.

## Why is this bad?

An `:ivar:` field tells a reader that the value belongs to an instance. A `:cvar:` field identifies a variable shared by
the class. [Sphinx](https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists) displays both under
Variables. [pydoctor](https://pydoctor.readthedocs.io/en/latest/codedoc.html) distinguishes them, so using the wrong
field misclassifies the attribute.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar limit: The most jobs it runs at once.
    """

    limit = 3
```

Use instead:

```python ok
class Engine:
    """Run jobs.

    :cvar limit: The most jobs it runs at once.
    """

    limit = 3
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

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3
  ```

## What it does

Checks the docstrings of classes, when `docstring-style = "rst"`, for attribute fields of the wrong kind.

### Active on

- **`:ivar name:` fields**: reported when `name` is a class variable.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3
  ```

  A `ClassVar` annotation counts the same when quoted, as `"ClassVar[int]"`.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  from typing import ClassVar


  class Engine:
      """Run jobs.

      :ivar limit: The most jobs it runs at once.
      """

      limit: "ClassVar[int]" = 3
  ```

- **A loop variable of the class body**: the target of a `for` statement there is a class variable, so `:ivar:` for it
  is reported.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :ivar step: The step.
      """

      for step in (1, 2):
          pass
  ```

- **`:cvar name:` fields**: reported when `name` is an instance attribute.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Engine:
      """Run jobs.

      :cvar state: What it is doing.
      """

      def __init__(self) -> None:
          self.state = "idle"
  ```

### Exceptions

- **`:var name:` fields**: `:var:` suits both kinds.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :var limit: The most jobs it runs at once.
      """

      limit = 3
  ```

- **A class default that methods also set on `self`**: the attribute is both kinds, so `:ivar:` and `:cvar:` both fit.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs.

      :ivar limit: The most jobs it runs at once.
      """

      limit = 3

      def reset(self) -> None:
          self.limit = 3
  ```

### Attribute kinds

| Attribute                                                                           | Kind               | Field                |
| ----------------------------------------------------------------------------------- | ------------------ | -------------------- |
| a `ClassVar`                                                                        | class variable     | `:cvar:`             |
| a `Final` with a value in the class body, as `SHOW: Final = 1`                      | class variable     | `:cvar:`             |
| a name the class body assigns without an annotation, that no method sets on `self`  | class variable     | `:cvar:`             |
| the target of a `for` statement in the class body                                   | class variable     | `:cvar:`             |
| any other annotated field, or a field declared with `field()`, `ib()` or `attrib()` | instance attribute | `:ivar:`             |
| a name a method sets on `self`                                                      | instance attribute | `:ivar:`             |
| a class default that methods also set on `self`                                     | both               | `:ivar:` or `:cvar:` |

### Fix

The fix renames the field, keeping its description.

**Unsafe fix:** code may read the docstring through `__doc__`.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Engine:
    """Run jobs.

    :ivar limit: The most jobs it runs at once.
    """

    limit = 3
```

```python fixed
class Engine:
    """Run jobs.

    :cvar limit: The most jobs it runs at once.
    """

    limit = 3
```

The field must start below the opening quotes in a single triple-quoted string whose text matches the source.
