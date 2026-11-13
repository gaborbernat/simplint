Flags a call of an `async def` function as a statement of its own, whose coroutine nothing awaits.

## Why is this bad?

Calling an `async def` function does not run its body. It returns a coroutine, and the body runs only when something
awaits it or schedules it as a task. A call whose result is dropped never runs, and Python only warns
`coroutine ... was never awaited` when the object is garbage collected, if warnings are shown at all.

## Example

```python report
import asyncio


async def save(record: str) -> None:
    await asyncio.sleep(0)


async def handle(record: str) -> None:
    save(record)
```

Use instead:

```python ok
import asyncio


async def save(record: str) -> None:
    await asyncio.sleep(0)


async def handle(record: str) -> None:
    await save(record)
```

## What it does

### Active on

- **Calls by name** of an `async def` of the same file, or of one another module of the run defines and the file
  imports, and **calls of `self.method()`** inside a method of the class that defines the `async def`.

  **Unsafe fix:** inside an `async def`, the fix adds `await`, which runs the coroutine at that point and waits for it.
  Code that meant to start it in the background needs `asyncio.create_task` instead. Outside an `async def`, a class
  body or a lambda included, nothing can await, so the call is reported without a fix.

  ```python report
  import asyncio


  async def save(record: str) -> None:
      await asyncio.sleep(0)


  async def handle(record: str) -> None:
      save(record)
  ```

  ```python fixed
  import asyncio


  async def save(record: str) -> None:
      await asyncio.sleep(0)


  async def handle(record: str) -> None:
      await save(record)
  ```

### Exceptions

- **Async generators**, which yield, return an async iterator rather than a coroutine.

- **Decorated functions**, since a decorator such as `contextlib.asynccontextmanager` replaces what the call returns.

- **A call whose result is used**, passed on, as `asyncio.run(main())`, or stored.

  ```python ok
  import asyncio


  async def main() -> None:
      await asyncio.sleep(0)


  asyncio.run(main())
  ```

## Ruff overlap

Ruff has no rule for an unawaited coroutine. Its
[asyncio-dangling-task (RUF006)](https://docs.astral.sh/ruff/rules/asyncio-dangling-task/) reports a task created and
dropped, which does run.

## Other tools

- **[mypy](https://mypy.readthedocs.io/en/stable/error_code_list.html#check-that-coroutine-return-value-is-used-unused-coroutine):**
  `unused-coroutine` reports the same call from the types, across any import.
