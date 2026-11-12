Flags `Callable[..., T]` annotations.

## Why is this bad?

`...` accepts any arguments, so the type checker cannot catch a call with the wrong number, type or names of arguments.
A [`Protocol`](https://docs.python.org/3/library/typing.html#typing.Protocol)
([PEP 544](https://peps.python.org/pep-0544/)) with a `__call__` method, called a
[callback protocol](https://typing.python.org/en/latest/spec/callables.html#callback-protocols), states the parameters.
It can also state keyword names and defaults, which a parameter list such as `Callable[[str, int], T]` cannot express.

## Example

```python report
from collections.abc import Callable


def run(callback: Callable[..., None]) -> None:
    callback("build", retries=3)
```

Use instead:

```python ok
from typing import Protocol


class Callback(Protocol):
    def __call__(self, name: str, *, retries: int) -> None: ...


def run(callback: Callback) -> None:
    callback("build", retries=3)
```

## What it does

No fix is offered: the real signature has to be written by hand.

### Active on

- **`Callable[..., T]`**: a subscript whose first argument is `...` and whose name, or the last part of a dotted name,
  is `Callable`. The check goes by name only, so it also matches type aliases and other expressions outside annotations.
  The matched names are:

  - a bare `Callable` imported from `typing` or `collections.abc`.

    ```python report
    from collections.abc import Callable


    def run(callback: Callable[..., None]) -> None:
        callback()
    ```

  - [`typing.Callable`](https://docs.python.org/3/library/typing.html#typing.Callable), a dotted name, here in a type
    alias.

    ```python report
    import typing

    Handler = typing.Callable[..., int]
    ```

  - [`collections.abc.Callable`](https://docs.python.org/3/library/collections.abc.html#collections.abc.Callable), a
    dotted name in a parameter annotation.

    ```python report
    import collections.abc


    def run(callback: collections.abc.Callable[..., None]) -> None:
        callback()
    ```

### Exceptions

- **Test parameters that are not called in the test body**: a test may inspect a callable or pass it to another API
  without invoking it itself. Its annotation need not describe one call signature. A direct call in the body still
  reports, including one inside a nested block.

  ```python ok file=tests/test_run.py
  from collections.abc import Callable

  from app.run import run_twice


  def test_run_twice(callback: Callable[..., None]) -> None:
      run_twice(callback)
  ```

- **String annotations**: these are not checked.

  ```python ok
  def run(callback: "Callable[..., None]") -> None:
      callback()
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
