Flags an `add_note` call on an exception that is then dropped instead of raised.

## Why is this bad?

[`BaseException.add_note()`](https://docs.python.org/3/builtins/exceptions.html#BaseException.add_note) attaches text to
an exception for its traceback. If the handler swallows the exception, or a function constructs it and then discards it,
the caller never sees the note.

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

- **An exception constructed in a function.** The rule looks for a single assignment such as `error = ValueError(path)`
  and reports the first `error.add_note(...)` if nothing else reads `error`.

  ```python report
  def check(path: str) -> None:
      error = ValueError(path)
      error.add_note("the path is not absolute")
  ```

### Exceptions

- **Another use of the exception.** Raising, chaining, returning or passing the exception can make the note available
  elsewhere.

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

- **Exceptions constructed at module scope** can be read from another module. The restriction to local names applies to
  constructed exceptions. The rule checks `except ... as error` handlers at module scope too.
