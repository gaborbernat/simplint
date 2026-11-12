Flags an `add_note` call on an exception that is then dropped instead of raised.

## Why is this bad?

[`BaseException.add_note`](https://docs.python.org/3/library/exceptions.html#BaseException.add_note) attaches text that
the traceback prints below the message. A handler that adds a note and then swallows the exception, or a function that
builds an exception, notes it and never raises it, writes the note to an object no one reads. The code reads as if the
error carried extra context, while it is silently ignored.

## Example

```python report
def read(path: str) -> str | None:
    try:
        with open(path) as stream:
            return stream.read()
    except OSError as error:
        error.add_note(f"while reading {path}")
    return None
```

Use instead:

```python ok
def read(path: str) -> str:
    try:
        with open(path) as stream:
            return stream.read()
    except OSError as error:
        error.add_note(f"while reading {path}")
        raise
```

## What it does

### Active on

- **A caught exception** given a note and then not re-raised, raised, returned or passed anywhere in the handler.
- **A local exception** built into a name of a function, given a note, and not used in any other way.

  ```python report
  def check(path: str) -> None:
      error = ValueError(path)
      error.add_note("the path is not absolute")
  ```

### Exceptions

- **Any other use** of the exception keeps the note reachable: raising it, chaining it with `from`, storing it, or
  passing it to a function.

  ```python ok
  def read(path: str, errors: list[Exception]) -> str | None:
      try:
          with open(path) as stream:
              return stream.read()
      except OSError as error:
          error.add_note(path)
          errors.append(error)
      return None
  ```

- **A module-level name** may be read from another module, so only names local to a function are followed.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
