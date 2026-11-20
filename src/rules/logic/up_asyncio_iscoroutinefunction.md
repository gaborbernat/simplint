Flags [`asyncio.iscoroutinefunction()`](https://docs.python.org/3/whatsnew/3.14.html#new-deprecations) calls, which
[`inspect.iscoroutinefunction()`](https://docs.python.org/3/library/inspect.html#inspect.iscoroutinefunction) replaces.

## Why is this bad?

Python 3.14 [deprecates `asyncio.iscoroutinefunction()`](https://docs.python.org/3/whatsnew/3.14.html#new-deprecations)
and schedules its removal for 3.16. It also recognizes the private `_is_coroutine` marker used by
[`asyncio.coroutine`](https://docs.python.org/3/whatsnew/3.11.html#whatsnew311-python-api-removed) before Python 3.11.
From 3.12, a synchronous function that returns a coroutine can use
[`inspect.markcoroutinefunction()`](https://docs.python.org/3/library/inspect.html#inspect.markcoroutinefunction) to be
recognized by `inspect.iscoroutinefunction()`.

## Example

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import asyncio


async def run(task):
    await asyncio.sleep(0)
    if asyncio.iscoroutinefunction(task):
        return await task()
    return task()
```

Use instead:

```python ok
import asyncio
import inspect


async def run(task):
    await asyncio.sleep(0)
    if inspect.iscoroutinefunction(task):
        return await task()
    return task()
```

## Configuration

The rule applies to every supported target version.

## What it does

### Active on

- **Calls of `asyncio.iscoroutinefunction`**, by its qualified name or by a name imported from `asyncio`. **Unsafe
  fix:** the call becomes `inspect.iscoroutinefunction(...)`, which changes the answer for a function marked with the
  legacy `_is_coroutine` marker. The fix adds `import inspect` when the module does not import it yet.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import asyncio


  async def run(task):
      await asyncio.sleep(0)
      if asyncio.iscoroutinefunction(task):
          return await task()
      return task()
  ```

  ```python fixed
  import asyncio
  import inspect


  async def run(task):
      await asyncio.sleep(0)
      if inspect.iscoroutinefunction(task):
          return await task()
      return task()
  ```

### Fix

The fix adds `import inspect` to the standard library imports at the start of the module. It keeps comments with the
import they precede.

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import asyncio

# os: the loop runs in the target directory
import os


def check(task):
    return asyncio.iscoroutinefunction(task), asyncio.run, os.sep
```

```python fixed
import asyncio
import inspect

# os: the loop runs in the target directory
import os


def check(task):
    return inspect.iscoroutinefunction(task), asyncio.run, os.sep
```

simplint reports the call without a fix in these cases.

- Another binding in the module or an enclosing scope would hide or be replaced by `inspect`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import asyncio
  from tools import inspect


  def check(task):
      return asyncio.iscoroutinefunction(task)
  ```

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import asyncio


  def check(inspect, task):
      return asyncio.iscoroutinefunction(task)
  ```

  ```python fixed
  import asyncio


  def check(inspect, task):
      return asyncio.iscoroutinefunction(task)
  ```

- The import would come before the module docstring and change `__doc__`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  # fmt: off
  """Task checks."""; import sys
  # fmt: on
  import asyncio


  def check(task):
      return asyncio.iscoroutinefunction(task)
  ```

  ```python fixed
  # fmt: off
  """Task checks."""; import sys
  # fmt: on
  import asyncio


  def check(task):
      return asyncio.iscoroutinefunction(task)
  ```

- The import would come before a `from __future__` import and cause a syntax error.

- The insertion point follows a standard library import that shares its line with another statement.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  # fmt: off
  import asyncio; print(asyncio.iscoroutinefunction(print))
  # fmt: on
  ```

  ```python fixed
  # fmt: off
  import asyncio; print(asyncio.iscoroutinefunction(print))
  # fmt: on
  ```
