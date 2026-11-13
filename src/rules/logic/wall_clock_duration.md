Flags durations measured as `time.time() - start`, where `start` also came from
[`time.time()`](https://docs.python.org/3/library/time.html#time.time).
[`time.perf_counter()`](https://docs.python.org/3/library/time.html#time.perf_counter) is the clock for durations.

## Why is this bad?

`time.time()` follows the system clock, which NTP, a user or a virtual machine resume can step backward or forward. A
duration taken across such a step comes out negative or far too long, which breaks timeouts, retries and benchmarks.
[PEP 418](https://peps.python.org/pep-0418/) added monotonic clocks for this: `time.perf_counter()` never goes backward
and has the highest resolution, and only the difference between two of its readings has a meaning.

## Example

```python report
import time


def timed(work):
    start = time.time()
    work()
    return time.time() - start
```

Use instead:

```python ok
import time


def timed(work):
    start = time.perf_counter()
    work()
    return time.perf_counter() - start
```

## What it does

### Active on

- **Subtractions between `time.time()` readings**, where at least one side is a local bound once to `time.time()` and
  the other is a call to it or another such local. **Fix:** every `time.time()` call of the measurement becomes
  `time.perf_counter()`. It is offered when the locals are read only in such subtractions, since a `perf_counter()`
  reading is not a date.

  ```python report
  import time


  def timed(work):
      start = time.time()
      work()
      end = time.time()
      return end - start
  ```

  ```python fixed
  import time


  def timed(work):
      start = time.perf_counter()
      work()
      end = time.perf_counter()
      return end - start
  ```

- **Readings that are also used as timestamps**, reported without a fix: the timestamp needs `time.time()`, and the
  duration a second reading of `time.perf_counter()`.

  ```python report
  import time


  def timed(work):
      start = time.time()
      work()
      print("started at", start)
      return time.time() - start
  ```

### Exceptions

- **Offsets from the current time**, such as `time.time() - 3600`: the result is a timestamp, not a duration.

  ```python ok
  import time


  def an_hour_ago():
      return time.time() - 3600
  ```

- **Locals bound more than once**, which may hold something other than a reading.

  ```python ok
  import time


  def elapsed(start=None):
      if start is None:
          start = time.time()
      return time.time() - start
  ```
