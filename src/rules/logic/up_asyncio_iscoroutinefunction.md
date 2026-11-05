Flags `asyncio.iscoroutinefunction()` calls, which `inspect.iscoroutinefunction()` replaces.

## Why is this bad?

Python 3.14 deprecates `asyncio.iscoroutinefunction()`. It also accepts functions marked with the `_is_coroutine` marker
that the `asyncio.coroutine` decorator set, a decorator removed in Python 3.11. `inspect.iscoroutinefunction()` checks
only the function itself, so the two answer differently for such functions.

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

No setting changes the report. Every target version has `inspect.iscoroutinefunction()`, so the rule reports on every
target.

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

The import goes before the first top-level import that sorts after `inspect`, or after the last top-level import. No fix
is offered when:

- `inspect` is already bound to something else in the module, which the import would shadow;

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
