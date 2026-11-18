Flags a lazy import directly inside a module-level
[`with contextlib.suppress(ImportError):`](https://docs.python.org/3/library/contextlib.html#contextlib.suppress) block.

## Why is this bad?

`suppress(ImportError)` around an import makes a missing optional module harmless: the block ends and the module goes
on. A lazy import does not load the module when the statement runs, so the block catches nothing, and a missing module
raises `ImportError` at the first use of its name, far from the guard
([PEP 810](https://peps.python.org/pep-0810/#what-happens-when-lazy-imports-encounter-errors)). The same holds for
`ModuleNotFoundError`, `Exception` and `BaseException`, which cover it.

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
  fix removes the entry when this import is the only one the entry defers.

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

The fix is unsafe: the module then loads when the block runs, so its import cost and anything its import does move to
the module's load.

## Other tools

- **[flake8-lazy](https://github.com/henryiii/flake8-lazy):** `LZY301`, for the `lazy` keyword only and without a fix.
- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. `lazy` is a syntax error in a `try` body, so a
  `try`/`except ImportError` guard needs no rule.
