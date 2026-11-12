Flags [`time.sleep`](https://docs.python.org/3/library/time.html#time.sleep) and `await asyncio.sleep` calls in a test
module whose argument is not the literal `0`.

## Why is this bad?

A test that sleeps waits a fixed time whether the code is ready or not. The wait is long enough to slow the suite, and
too short on a loaded machine, where the test then fails for no real reason. Martin Fowler's
[article on non-deterministic tests](https://martinfowler.com/articles/nonDeterminism.html#AsynchronousBehavior) names
bare sleeps as a cause of both and asks for a callback or polling instead, and warns that flaky tests teach people to
ignore failures. Waiting on an event the code sets, with
[`Event.wait`](https://docs.python.org/3/library/threading.html#threading.Event.wait), returns as soon as the code is
ready, and its timeout bounds the wait if it never is.

## Example

```python report file=tests/test_worker.py
import time


def test_worker_finishes():
    start_worker()
    time.sleep(1)
    assert worker_done()
```

Use instead:

```python ok
import threading


def test_worker_finishes():
    done = threading.Event()
    start_worker(on_done=done.set)
    assert done.wait(timeout=5)
```

## What it does

### Active on

- **`time.sleep` and [`asyncio.sleep`](https://docs.python.org/3/library/asyncio-task.html#asyncio.sleep) calls**: the
  call resolves through the module's imports, so `from time import sleep` and `await asyncio.sleep(...)` are reported
  too. Every call counts, in a test function or in a helper of the module, as the helper runs for the tests too.

  ```python report file=tests/test_sleep.py
  import asyncio
  from time import sleep


  def test_sleep():
      sleep(0.5)


  async def test_async_sleep():
      await asyncio.sleep(2)
  ```

### Exceptions

- **A sleep for the literal `0`**: `sleep(0)` yields to other tasks without waiting, so it does not make a test slow.

  ```python ok file=tests/test_zero.py
  import time


  def test_zero():
      time.sleep(0)
  ```

- **`await asyncio.sleep` as the whole body of a `while`**: Ruff's
  [async-busy-wait](https://docs.astral.sh/ruff/rules/async-busy-wait/) (ASYNC110) reports a loop that waits on a sleep
  this way, so this rule leaves that sleep to Ruff.

  ```python ok file=test_busy.py
  import asyncio


  async def test_busy(done):
      while not done():
          await asyncio.sleep(1)
  ```

- **Modules that patch `time.sleep`**: a test that replaces the sleep with `mocker.patch`, `monkeypatch.setattr` or
  `patch` already controls the wait, so the module is not reported. The patch is recognized by its target written as the
  dotted string `"time.sleep"`, or by `monkeypatch.setattr(time, "sleep", ...)`.

  ```python ok file=tests/test_patched.py
  import time


  def test_patched(mocker):
      mocker.patch("time.sleep")
      time.sleep(1)
  ```

  ```python ok file=tests/test_patched_module.py
  import time


  def test_patched(monkeypatch):
      monkeypatch.setattr(time, "sleep", lambda seconds: None)
      time.sleep(1)
  ```

### Fix

No fix is offered, as the wait has to become an event the code sets, which depends on the code under test.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [ASYNC110](https://docs.astral.sh/ruff/rules/async-busy-wait/), which reports an `asyncio.sleep` polling loop and
  which this rule leaves to it. [ASYNC251](https://docs.astral.sh/ruff/rules/blocking-sleep-in-async-function/) reports
  `time.sleep` in async functions for blocking, a different problem.
