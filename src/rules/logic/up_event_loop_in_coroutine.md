Flags `asyncio.get_event_loop()` calls inside coroutines, where `asyncio.get_running_loop()` returns the loop the
coroutine runs on.

## Why is this bad?

A coroutine runs on a loop that is always running, so `get_event_loop()` only has to return that loop.
`get_running_loop()` returns it directly and raises `RuntimeError` when no loop runs, which surfaces a mistake that
`get_event_loop()` would hide. `get_event_loop()` also serves code outside a loop, where Python 3.10 deprecated the
implicit loop creation and Python 3.14 made the call raise `RuntimeError`.

## Example

```python report
import asyncio


async def run() -> asyncio.AbstractEventLoop:
    return asyncio.get_event_loop()
```

Use instead:

```python ok
import asyncio


async def run() -> asyncio.AbstractEventLoop:
    return asyncio.get_running_loop()
```

## What it does

### Active on

- **`asyncio.get_event_loop()` with no arguments, called inside an `async def`**: a class body inside the coroutine
  counts as part of it, while a nested `def` or `lambda` does not. **Safe fix:** the attribute is renamed to
  `get_running_loop`. A call through a name imported from `asyncio`, such as `get_event_loop()`, is reported without a
  fix.

  ```python report
  import asyncio


  async def run() -> asyncio.AbstractEventLoop:
      return asyncio.get_event_loop()
  ```

  ```python fixed
  import asyncio


  async def run() -> asyncio.AbstractEventLoop:
      return asyncio.get_running_loop()
  ```

### Exceptions

- **Calls in plain functions**: such a function may run before any loop starts, where `get_running_loop()` raises.

  ```python ok
  import asyncio


  def start() -> asyncio.AbstractEventLoop:
      return asyncio.get_event_loop()
  ```

- **Calls in a nested `def` or `lambda` of a coroutine**: the nested function may run after the loop stops.

  ```python ok
  import asyncio


  async def run():
      def later() -> asyncio.AbstractEventLoop:
          return asyncio.get_event_loop()

      return later
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
