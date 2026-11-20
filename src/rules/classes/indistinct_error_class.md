Flags stateless exception classes when no handler in the checked production code distinguishes them or their subclasses
by type.

## Why is this bad?

Use a built-in exception when callers already handle the failure that way. In the example, `ConfigError` adds a type to
import and document, but no handler distinguishes it from other invalid values. A custom exception is useful when a
caller needs to recover from this failure without catching unrelated
[`ValueError`](https://docs.python.org/3/builtins/exceptions.html#ValueError)s. The
[Google Python Style Guide](https://google.github.io/styleguide/pyguide.html#244-decision) recommends built-in
exceptions when they fit.

## Example

```python report
class ConfigError(Exception):
    pass


def load(path: str) -> None:
    raise ConfigError(path)
```

Use instead:

```python ok
def load(path: str) -> None:
    raise ValueError(path)
```

## What it does

### Active on

- **Module-level exception classes** are candidates when no handler in the checked production files names the class or a
  subclass. Simplint follows inheritance from
  [`BaseException`](https://docs.python.org/3/builtins/exceptions.html#BaseException), including through other project
  classes.

  ```python report
  class ConfigError(Exception):
      pass
  ```

- **Subclasses of built-in exceptions**: a built-in base makes the class an exception, whatever the class's own name.

  ```python report
  class ConfigError(ValueError):
      pass
  ```

- **Standard-library exception bases**: a class of a standard-library module whose name ends in `Error`,
  [`Exception`](https://docs.python.org/3/builtins/exceptions.html#Exception) or
  [`Warning`](https://docs.python.org/3/builtins/exceptions.html#Warning) counts as an exception base.

  ```python report
  from json import JSONDecodeError


  class ConfigError(JSONDecodeError):
      pass
  ```

- **Test modules** can define or catch an exception without affecting this rule. A `pytest.raises(...)` assertion also
  leaves the production diagnostic in place.

  ```python report file=app/errors.py
  class ConfigError(Exception):
      pass
  ```

  ```python context file=tests/test_errors.py
  from app.errors import ConfigError


  def test_config() -> None:
      try:
          raise ConfigError()
      except ConfigError:
          pass
  ```

- **Handlers that distinguish an exception by type** include these cases.

  - **A class named in a tuple of an `except` clause**: each class of the tuple counts.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    from app.errors import ConfigError


    def load() -> None:
        try:
            raise ConfigError()
        except (KeyError, ConfigError):
            pass
    ```

  - **A tuple of classes named by an `except` clause**: a module-level tuple of classes, such as
    `RETRYABLE = (ConfigError,)`, counts as each class it holds, in whichever module the tuple is defined. An annotated
    tuple counts too, and so does a tuple that unpacks another, as `BOTH = (*RETRYABLE, OtherError)` does.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass


    RETRYABLE = (ConfigError,)
    ```

    ```python context file=app/main.py
    from app.errors import RETRYABLE


    def load() -> None:
        try:
            raise ValueError()
        except RETRYABLE:
            pass
    ```

    ```toml config
    [project]
    name = "demo"
    requires-python = ">=3.11"
    ```

    ```python ok file=app/limits.py
    from typing import Final


    class ConfigError(Exception):
        pass


    class LimitError(Exception):
        pass


    RETRYABLE: Final = (ConfigError,)
    BOTH = (*RETRYABLE, LimitError)
    ```

    ```python context file=app/main.py
    from app.limits import BOTH


    def load() -> None:
        try:
            raise ValueError()
        except BOTH:
            pass
    ```

  - **A class given to [`contextlib.suppress`](https://docs.python.org/3/library/contextlib.html#contextlib.suppress)**:
    each argument of `suppress` counts, starred or not, since the context manager catches it.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    import contextlib

    from app.errors import ConfigError


    def load() -> None:
        with contextlib.suppress(ConfigError):
            raise ConfigError()
    ```

  - **An `except*` clause**: an exception group handler names a class the same way.

    ```toml config
    [project]
    requires-python = ">=3.11"
    ```

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    from app.errors import ConfigError


    def load() -> None:
        try:
            raise ExceptionGroup("errors", [ConfigError()])
        except* ConfigError:
            pass
    ```

  - **A split or subgroup of an exception group inside a handler**: `split` and `subgroup` take the classes to select as
    their first argument.

    ```toml config
    [project]
    requires-python = ">=3.11"
    ```

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    from app.errors import ConfigError


    def load() -> None:
        try:
            raise ExceptionGroup("errors", [ConfigError()])
        except ExceptionGroup as group:
            matched, rest = group.split(ConfigError)
    ```

  - **An `isinstance` check inside a handler**: a check on the caught exception names the class.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    from app.errors import ConfigError


    def load() -> None:
        try:
            raise ConfigError()
        except Exception as error:
            if isinstance(error, ConfigError):
                pass
    ```

  - **A class pattern inside a handler**: a `case` of a `match` on the caught exception names the class.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/main.py
    from app.errors import ConfigError


    def load() -> None:
        try:
            raise ConfigError()
        except Exception as error:
            match error:
                case ConfigError():
                    pass
    ```

  - **A handler through a module that re-exports the class**: the catch reaches the class where it is defined, through
    the `from ... import` statements that re-export it.

    ```python ok file=app/errors.py
    class ConfigError(Exception):
        pass
    ```

    ```python context file=app/api.py
    from app.errors import ConfigError

    __all__ = ["ConfigError"]
    ```

    ```python context file=app/main.py
    from app.api import ConfigError


    def load() -> None:
        try:
            raise ConfigError()
        except ConfigError:
            pass
    ```

  - **A handler in the module itself**: a class the module catches is in use.

    ```python ok
    class ConfigError(Exception):
        pass


    def load() -> None:
        try:
            raise ConfigError()
        except ConfigError:
            pass
    ```

### Exceptions

- **A library's public API**: a class a public module offers, which users of the library may catch, is kept. This rule
  reuses the public API decision of the docstring rules.

  ```toml config
  [build-system]
  requires = ["setuptools"]

  [project]
  name = "mylib"
  ```

  ```python ok file=mylib/errors.py
  class ConfigError(Exception):
      pass
  ```

- **Classes with state** may supply information that callers read from the exception. Simplint skips a class that
  defines `__init__`, assigns or annotates a class attribute, or assigns an attribute on `self`.

  ```python ok
  class ConfigError(Exception):
      def __init__(self, key: str) -> None:
          super().__init__(key)
          self.key = key
  ```

- **A base class with a caught subclass** stays in the hierarchy. For example, `except ConfigError` distinguishes
  `ConfigError`, so simplint keeps both it and its base `AppError`. The handler itself catches only `ConfigError` and
  its subclasses.

  ```python ok file=app/errors.py
  class AppError(Exception):
      pass


  class ConfigError(AppError):
      pass
  ```

  ```python context file=app/main.py
  from app.errors import ConfigError


  def load() -> None:
      try:
          raise ConfigError()
      except ConfigError:
          pass
  ```

- **Warning subclasses** can be selected by warning filters.

  ```python ok
  class ConfigWarning(UserWarning):
      pass
  ```

The rule offers no fix because changing the exception type can change which handlers run.

## Other tools

- Ruff's [rule list](https://docs.astral.sh/ruff/rules/) has no equivalent check for unused exception types.
