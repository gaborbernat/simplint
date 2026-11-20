Flags uses of [`sys.exc_info()`](https://docs.python.org/3/library/sys.html#sys.exc_info) that Python 3.11 can replace
with [`sys.exception()`](https://docs.python.org/3/library/sys.html#sys.exception).

## Why is this bad?

Inside an exception handler, `sys.exc_info()` returns `(type(e), e, e.__traceback__)`. `sys.exception()` returns `e`
directly. Outside a handler, `sys.exception()` and `sys.exc_info()[1]` both return `None`.
[`traceback.print_exception()`](https://docs.python.org/3/library/traceback.html#traceback.print_exception) accepts the
exception alone.

## Example

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import sys

error = sys.exc_info()[1]
```

Use instead:

```python ok
import sys

error = sys.exception()
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python ok
  import sys

  error = sys.exc_info()[1]
  ```

## What it does

### Active on

The rule recognizes module aliases such as `import sys as system` in both forms.

- **`sys.exc_info()[1]`**: a read of the second value.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python report
  import sys

  error = sys.exc_info()[1]
  ```

  ```python fixed
  import sys

  error = sys.exception()
  ```

- **`traceback.print_exception(*sys.exc_info())`.** The unpacked tuple must be the only positional argument. Keywords
  such as `limit=` may follow.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python report
  import sys
  import traceback


  def report() -> None:
      try:
          run()
      except Exception:
          traceback.print_exception(*sys.exc_info())
  ```

  ```python fixed
  import sys
  import traceback


  def report() -> None:
      try:
          run()
      except Exception:
          traceback.print_exception(sys.exception())
  ```

### Exceptions

- **The first or third value**: `sys.exc_info()[0]` and `sys.exc_info()[2]` have no single call with the same result.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python ok
  import sys

  kind = sys.exc_info()[0]
  trace = sys.exc_info()[2]
  ```

- **A call imported by name**: `from sys import exc_info` gives a call the rule does not rewrite.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python ok
  from sys import exc_info

  error = exc_info()[1]
  ```

- **Target versions before 3.11**: `sys.exception()` does not exist there.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python ok
  import sys

  error = sys.exc_info()[1]
  ```

### Fix

The safe fix replaces the subscript or the unpacked argument with `sys.exception()`. simplint reports without a fix if
the replacement would remove a comment.

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import sys

error = sys.exc_info()[
    1  # the exception being handled
]
```

```python fixed
import sys

error = sys.exc_info()[
    1  # the exception being handled
]
```
