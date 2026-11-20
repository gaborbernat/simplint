Flags exception classes in an `except` clause that an earlier handler, or another entry of the same clause, already
catches through a base class.

## Why is this bad?

Python tries the handlers of a `try` in order and runs the first whose class matches. For example, a handler for
[`FileNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#FileNotFoundError) after one for
[`OSError`](https://docs.python.org/3/builtins/exceptions.html#OSError) never runs. Within a single clause,
`(OSError, FileNotFoundError)` catches the same exceptions as `OSError` alone, in either order.

## Example

```python report
def read(path: str) -> str | None:
    try:
        with open(path) as stream:
            return stream.read()
    except OSError:
        return ""
    except FileNotFoundError:
        return None
```

Use instead:

```python ok
def read(path: str) -> str | None:
    try:
        with open(path) as stream:
            return stream.read()
    except FileNotFoundError:
        return None
    except OSError:
        return ""
```

## What it does

### Active on

- **A class whose base an earlier handler catches**: a lone class means the whole handler never runs; an entry of a
  tuple never matches.

  ```python report
  def load(path: str) -> str | None:
      try:
          with open(path) as stream:
              return stream.read()
      except LookupError:
          return None
      except (KeyError, UnicodeDecodeError):
          return ""
  ```

- **A handler whose every entry an earlier handler catches** gets one report on the clause: the handler never runs.

  ```python report
  def load(path: str) -> str | None:
      try:
          with open(path) as stream:
              return stream.read()
      except LookupError:
          return None
      except OSError:
          return None
      except (KeyError, FileNotFoundError):
          return ""
  ```

- **A tuple entry whose base the same tuple names** adds nothing.

  ```python report
  def parse(text: bytes) -> int:
      try:
          return int(text.decode())
      except (ValueError, UnicodeDecodeError):
          return 0
  ```

The classes are the builtin exceptions, the standard library exceptions whose bases are the same on every supported
Python version, and the classes defined in the same file.

### Exceptions

- **Classes simplint cannot place**, such as a parameter or a class imported from another module of the project, are
  left alone.

  ```python ok
  def load(path: str, error: type[Exception]) -> str | None:
      try:
          with open(path) as stream:
              return stream.read()
      except Exception:
          return None
      except error:
          return ""
  ```

- **The same class named twice** is left to Ruff's
  [duplicate-handler-exception](https://docs.astral.sh/ruff/rules/duplicate-handler-exception/) and
  [duplicate-try-block-exception](https://docs.astral.sh/ruff/rules/duplicate-try-block-exception/).

  ```python ok
  def load(path: str) -> str | None:
      try:
          with open(path) as stream:
              return stream.read()
      except (OSError, OSError):
          return None
  ```

### Fix

The fix safely removes a redundant tuple entry. Its base class remains in the same tuple or an earlier handler. Simplint
leaves a fully shadowed handler for you to review because deleting it would also delete its body. It also preserves
tuples that contain comments.

```python report
def parse(text: bytes) -> int:
    try:
        return int(text.decode())
    except (ValueError, UnicodeDecodeError):
        return 0
```

```python fixed
def parse(text: bytes) -> int:
    try:
        return int(text.decode())
    except ValueError:
        return 0
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** its duplicate-handler rules report a repeated exception class.
- [pylint](https://github.com/pylint-dev/pylint) reports both forms as
  [bad-except-order](https://pylint.readthedocs.io/en/latest/user_guide/messages/error/bad-except-order.html) and
  [overlapping-except](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/overlapping-except.html).
