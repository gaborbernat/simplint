Flags [`time.sleep`](https://docs.python.org/3/library/time.html#time.sleep) and `asyncio.sleep` calls in a test module
whose argument is not the literal `0`.

## Why is this bad?

A fixed sleep delays a test when the code is ready early and can expire too soon on a busy machine. For a threaded test,
[`Event.wait(timeout)`](https://docs.python.org/3/library/threading.html#threading.Event.wait) returns as soon as the
worker sets the event. Check its return value so the test fails if the timeout expires. Martin Fowler describes the same
[timing problem in asynchronous tests](https://martinfowler.com/articles/nonDeterminism.html#AsynchronousBehavior).

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
  too. A call counts in the body of a test function, a fixture, or a setup or teardown method named `setUp`, `tearDown`,
  `asyncSetUp`, `asyncTearDown`, `setUpClass` or `tearDownClass`. The rule recognizes `asyncio.sleep` even without an
  `await`.

  ```python report file=tests/test_sleep.py
  import asyncio
  from time import sleep


  def test_sleep():
      sleep(0.5)


  async def test_async_sleep():
      await asyncio.sleep(2)
  ```

### Exceptions

- **Sleeps in helpers**: a function or class that is not a test or a fixture, including one a test defines inside its
  body, such as a test server, a test double or a backoff helper, may wait on purpose to stand in for a slow peer.

  ```python ok file=tests/test_server.py
  import time


  class SlowServer:
      def respond(self) -> bytes:
          time.sleep(0.1)
          return b"ok"
  ```

- **A sleep for the literal `0`**: the rule accepts the exact spelling `0` for either sleep function. For example,
  `await asyncio.sleep(0)` gives other tasks a chance to run.

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
