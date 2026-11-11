Flags exception classes that no `except` clause in the run names by name.

## Why is this bad?

Callers handle an error by the type they catch. A class that nothing catches by name adds a name to import and a type to
document, yet callers cannot tell it from a built-in exception, which says the same thing. Raising a built-in such as
`ValueError` keeps the program to the types Python already has.

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

- **Exception classes no handler names**: a class whose base is `Exception`, `BaseException`, a class of the run that is
  one, or a standard-library exception, with no `except` clause in non-test code that names it or a subclass of it.

  ```python report
  class ConfigError(Exception):
      pass
  ```

- **Subclasses of built-in exceptions**: a built-in base makes the class an exception, whatever the class's own name.

  ```python report
  class ConfigError(ValueError):
      pass
  ```

- **Standard-library exception bases**: a class of a standard-library module whose name ends in `Error`, `Exception` or
  `Warning` counts as an exception base.

  ```python report
  from json import JSONDecodeError


  class ConfigError(JSONDecodeError):
      pass
  ```

- **Catches in test modules**: an `except` clause in a test module, or a `pytest.raises(...)` block, does not count, so
  the class stays reported.

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

- **What counts as naming a class in a handler**: a class named in any of these ways is caught by name, so it is not
  reported:

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

  - **A class given to `contextlib.suppress`**: each argument of `suppress` counts, starred or not, since the context
    manager catches it.

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

- **Classes with state**: a class that assigns attributes, or defines `__init__`, is kept, since a handler may read
  attributes off it that a built-in cannot carry. The rule does not follow each attribute read, so any such class is
  skipped.

  ```python ok
  class ConfigError(Exception):
      def __init__(self, key: str) -> None:
          super().__init__(key)
          self.key = key
  ```

- **A base class with a caught subclass**: a handler that names a subclass also catches its bases, so only the subclass
  is needed. A base is reported only when no subclass of it is caught.

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

- **Warnings**: a subclass of `Warning` is raised to be seen, not caught, so it is not reported.

  ```python ok
  class ConfigWarning(UserWarning):
      pass
  ```

The rule has no fix, because the right replacement type depends on what the callers expect, and only the code's author
knows that.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
