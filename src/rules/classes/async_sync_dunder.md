Flags `async def` on special methods that Python calls without awaiting the result.

## Why is this bad?

[`len(obj)`](https://docs.python.org/3/builtins/functions.html#len) expects an integer from `__len__`, and `async for`
expects an async iterator from `__aiter__`. An `async def` that contains no `yield` returns a coroutine when called.
Python raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) when either operation receives
that coroutine. Other methods can fail silently. For example, a synchronous context manager never awaits an async
`__exit__`, so its cleanup code does not run.

## Example

```python report
class Stream:
    async def __aiter__(self) -> "Stream":
        return self

    async def __anext__(self) -> int:
        return 1
```

Use instead:

```python ok
class Stream:
    def __aiter__(self) -> "Stream":
        return self

    async def __anext__(self) -> int:
        return 1
```

## What it does

### Active on

- **Synchronous special methods** defined with `async def` in a class body: object creation and representation,
  conversions, containers and iteration, attribute and descriptor access, `__enter__` and `__exit__`, comparisons,
  copying and pickling, `__aiter__` and `__await__`.

  ```python report
  class Stream:
      async def __len__(self) -> int:
          return 0
  ```

### Exceptions

- **`__aiter__` written as an async generator**: an `async def __aiter__` that yields returns an async generator, which
  is the async iterator `async for` needs.

  ```python ok
  class Stream:
      async def __aiter__(self):
          yield 1
  ```

- **The async protocol methods** `__anext__`, `__aenter__` and `__aexit__`, and methods such as `__call__` and
  `__getitem__` that may return an awaitable by design.

  ```python ok
  class Client:
      async def __call__(self) -> int:
          return 1
  ```

## Other tools

- Ruff's [yield-in-init](https://docs.astral.sh/ruff/rules/yield-in-init/) checks for `yield` in `__init__`.
