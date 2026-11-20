Flags `Callable[..., T]` annotations.

## Why is this bad?

The `...` in `Callable[..., T]` leaves the arguments unchecked. Use `Callable[[str, int], T]` for positional parameters,
or a [callback protocol](https://typing.python.org/en/latest/spec/callables.html#callback-protocols) with a `__call__`
method for keyword names, defaults or overloads. A decorator that forwards arguments can preserve the wrapped signature
with a [`ParamSpec`](https://docs.python.org/3/library/typing.html#typing.ParamSpec).

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

Specify the callback's signature by hand.

### Active on

- simplint resolves `Callable` to `typing`, `typing_extensions` or `collections.abc`, including imports under an alias.
  It checks expressions outside annotations too, including type alias values.

  - a bare `Callable` imported from `typing` or
    [`collections.abc`](https://docs.python.org/3/library/collections.abc.html#module-collections.abc).

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

- **Function parameters in test modules**: a test may inspect a callable or pass it to another API without invoking it
  itself. Its annotation need not describe one call signature. The rule checks the annotation when the body contains
  `callback(...)` or `callback.__call__(...)`, including inside a nested block.

  ```python ok file=tests/test_run.py
  from collections.abc import Callable

  from app.run import run_twice


  def test_run_twice(callback: Callable[..., None]) -> None:
      run_twice(callback)
  ```

- **The bound of a type variable**, as `TypeVar("F", bound=Callable[..., Any])` or `def wrap[F: Callable[..., Any]]`:
  the [typing docs](https://typing.python.org/en/latest/reference/generics.html) use it for a decorator that keeps the
  signature of what it wraps, which a `Protocol` cannot spell. A constraint of a type variable stays reported.

  ```python ok
  from collections.abc import Callable
  from typing import Any, TypeVar

  F = TypeVar("F", bound=Callable[..., Any])
  ```

- **String annotations**: these are not checked.

  ```python ok
  def run(callback: "Callable[..., None]") -> None:
      callback()
  ```
