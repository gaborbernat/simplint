Flags `async def` on special methods that Python calls without awaiting the result.

## Why is this bad?

Python calls `__len__`, `__iter__`, `__enter__`, `__aiter__` and the other special methods of the data model
synchronously and uses what they return. An `async def` method returns a coroutine instead, so `len(obj)` raises
`TypeError`, `__init__` raises `TypeError` for returning a value other than `None`, and `async for` gets a coroutine
where it needs an async iterator. The coroutine is never awaited, and Python warns about it at best.

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
