Flags a lazy import directly inside a module-level
[`with contextlib.suppress(ImportError):`](https://docs.python.org/3/library/contextlib.html#contextlib.suppress) block.

## Why is this bad?

An eager import raises inside `suppress(ImportError)`, where the guard can catch it. With a lazy import, the error can
surface after the guard has exited, when code first reads the imported name. A guard that catches
[`ModuleNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#ModuleNotFoundError),
[`Exception`](https://docs.python.org/3/builtins/exceptions.html#Exception) or
[`BaseException`](https://docs.python.org/3/builtins/exceptions.html#BaseException) can have the same problem.
[PEP 810](https://peps.python.org/pep-0810/#what-happens-when-lazy-imports-encounter-errors) describes when lazy imports
raise.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
from contextlib import suppress

with suppress(ImportError):
    lazy import ujson as json
```

```python fixed
from contextlib import suppress

with suppress(ImportError):
    import ujson as json
```

## What it does

### Active on

- **A `lazy` import directly in the block.** The fix removes `lazy`.

- **An import directly in the block whose module
  [`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules) lists**:
  on Python 3.15 the list defers it the same way, while older versions ignore the list and run the guard as written. The
  fix removes a unique entry when there is no other eligible import of that module. The check matches the declaration by
  name even if it comes after the block and could not affect that import.

  ```python report
  from contextlib import suppress

  __lazy_modules__ = ["ujson"]

  with suppress(ImportError):
      import ujson
  ```

  ```python fixed
  from contextlib import suppress

  __lazy_modules__ = []

  with suppress(ImportError):
      import ujson
  ```

### Exceptions

- **`suppress` of other exceptions**, such as `suppress(KeyError)`, which never covered a failed import.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  from contextlib import suppress

  with suppress(KeyError):
      lazy import json
  ```

- **A listed module the module also imports outside the block**: the entry defers that import too, so the fix keeps it,
  and the report stays.

### Fix

The unsafe fix makes the import eager. Its cost and any side effects then occur inside the guard when the containing
module loads.

## Other tools

- **[flake8-lazy](https://github.com/henryiii/flake8-lazy):** `LZY301`, for the `lazy` keyword only and without a fix.

Python already rejects `lazy` inside a `try` body.
