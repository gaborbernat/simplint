Flags `sys.exc_info()` reads of the exception being handled, which `sys.exception()` returns directly from Python 3.11.

## Why is this bad?

`sys.exc_info()` returns a tuple of three values, and code that reads its second value, or unpacks all three into
`traceback.print_exception`, only needs the exception. `sys.exception()` names that value and returns it alone.

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

Reports these forms, when the call resolves to `sys.exc_info`, so `import sys as system` counts:

- **`sys.exc_info()[1]`**: a read of the second value.
- **`traceback.print_exception(*sys.exc_info())`**: an unpacking of the whole tuple as the only argument.

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

The fix is safe. It replaces the subscript with `sys.exception()`, or the unpacked argument with it, which gives the
same value.
