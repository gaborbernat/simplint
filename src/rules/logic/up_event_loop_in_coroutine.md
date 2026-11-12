Flags [`asyncio.get_event_loop()`](https://docs.python.org/3/library/asyncio-eventloop.html#asyncio.get_event_loop)
calls inside coroutines, where
[`asyncio.get_running_loop()`](https://docs.python.org/3/library/asyncio-eventloop.html#asyncio.get_running_loop) (added
in Python 3.7) returns the loop the coroutine runs on.

## Why is this bad?

Inside a coroutine, a loop is running. The
[asyncio docs](https://docs.python.org/3/library/asyncio-eventloop.html#asyncio.get_event_loop) prefer
`get_running_loop()` there and in callbacks, because `get_event_loop()` behaves in more complex ways, especially with
custom event loop policies. `get_running_loop()` returns the running loop or raises
[`RuntimeError`](https://docs.python.org/3/builtins/exceptions.html#RuntimeError), and it consults no policy.

`get_event_loop()` serves code outside a loop, and that part keeps changing: Python
[3.10](https://docs.python.org/3.10/library/asyncio-eventloop.html#asyncio.get_event_loop) deprecated the call without a
running loop, Python 3.14 made it raise `RuntimeError`, and the policy system it consults is deprecated for removal in
Python 3.16.

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
  counts as part of it, while the body of a nested `def` or `lambda` does not. Their decorators and defaults run in the
  coroutine, so a call there is reported too, and so is a call in an annotation of a nested `def` unless the annotations
  are lazy. **Safe fix:** the attribute is renamed to `get_running_loop`. A call through a name imported from `asyncio`,
  such as `get_event_loop()`, is reported without a fix.

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

  ```python report
  import asyncio


  async def run():
      def later(loop=asyncio.get_event_loop()):
          return loop

      return later
  ```

  ```python fixed
  import asyncio


  async def run():
      def later(loop=asyncio.get_running_loop()):
          return loop

      return later
  ```

### Exceptions

- **Decorators, defaults and annotations of an `async def`**: they run where the `def` statement runs, before any loop
  is running, so a call there stays.

  ```python ok
  import asyncio


  @register(asyncio.get_event_loop())
  async def run(loop=asyncio.get_event_loop()):
      return loop
  ```

- **Annotations of a nested `def` when annotations are lazy**: with `from __future__ import annotations`, or a target of
  Python 3.14 or later, an annotation is not evaluated when the `def` runs.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  import asyncio


  async def run():
      def later(loop: asyncio.get_event_loop()):
          return loop

      return later
  ```

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
