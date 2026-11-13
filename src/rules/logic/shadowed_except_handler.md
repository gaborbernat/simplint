Flags exception classes in an `except` clause that an earlier handler, or another entry of the same clause, already
catches through a base class.

## Why is this bad?

Python tries the handlers of a `try` in order and runs the first whose class matches. A handler for `FileNotFoundError`
after one for `OSError` never runs, so the code in it is dead and the reader is misled about which handler deals with
the error. Inside one clause, `(OSError, FileNotFoundError)` catches the same as `OSError` alone, and the extra entry
suggests a distinction the code does not make.

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

The fix removes a shadowed entry from a tuple and is safe: the base that shadows it stays in front of it, so the clause
catches the same exceptions. A lone class, a handler whose every entry is shadowed and a tuple with a comment in it get
no fix, since removing the handler drops its body.

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Its duplicate handler rules cover a class named
  twice, not a subclass after its base.
- [pylint](https://github.com/pylint-dev/pylint) reports both forms as
  [bad-except-order](https://pylint.readthedocs.io/en/latest/user_guide/messages/error/bad-except-order.html) and
  [overlapping-except](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/overlapping-except.html).
