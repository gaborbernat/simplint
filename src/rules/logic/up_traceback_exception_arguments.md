Flags `traceback` calls that pass an exception's type and traceback next to the exception, which Python 3.10 reads from
the exception itself.

## Why is this bad?

Since Python 3.10 the `traceback` functions take the exception alone, and read its type and traceback from it. The
three-argument form repeats what the exception already carries, and makes the call longer to read.

## Example

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import traceback


def report(run):
    try:
        run()
    except Exception as error:
        traceback.print_exception(type(error), error, error.__traceback__)
```

Use instead:

```python ok
import traceback


def report(run):
    try:
        run()
    except Exception as error:
        traceback.print_exception(error)
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.9"
  ```

  ```python ok
  import traceback


  def report(run):
      try:
          run()
      except Exception as error:
          traceback.print_exception(type(error), error, error.__traceback__)
  ```

## What it does

### Active on

Reports a call that resolves to
[`traceback.print_exception`](https://docs.python.org/3/library/traceback.html#traceback.print_exception),
[`traceback.format_exception`](https://docs.python.org/3/library/traceback.html#traceback.format_exception) or
[`traceback.format_exception_only`](https://docs.python.org/3/library/traceback.html#traceback.format_exception_only)
and passes:

- **Three positional arguments** to the first two functions: `type(error)` or `error.__class__`, then `error`, then
  `error.__traceback__`, all for the same name.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import traceback


  def report(error):
      return traceback.format_exception(error.__class__, error, error.__traceback__, limit=2)
  ```

  ```python fixed
  import traceback


  def report(error):
      return traceback.format_exception(error, limit=2)
  ```

- **Two positional arguments** to `format_exception_only`: `type(error)`, then `error`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import traceback


  def report(error):
      return traceback.format_exception_only(type(error), error)
  ```

  ```python fixed
  import traceback


  def report(error):
      return traceback.format_exception_only(error)
  ```

### Exceptions

- **A traceback that is not the exception's own**: the call passes a different traceback, which the exception does not
  carry.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python ok
  import traceback


  def report(error, saved):
      return traceback.format_exception(type(error), error, saved)
  ```

- **Target versions before 3.10**: the single-argument form raises
  [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) there.

  ```toml config
  [project]
  requires-python = ">=3.9"
  ```

  ```python ok
  import traceback


  def report(error):
      return traceback.format_exception(type(error), error, error.__traceback__)
  ```

### Fix

The fix is safe. It replaces the first arguments with the exception and keeps the keyword arguments, such as `limit=`
and `chain=`. No fix is offered when a comment lies in the arguments it replaces, since the rewrite would drop it. A
comment between those arguments keeps the report without a fix:

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import traceback


def report(error):
    return traceback.format_exception(
        type(error),  # the type, for readers of older code
        error,
        error.__traceback__,
    )
```

```python fixed
import traceback


def report(error):
    return traceback.format_exception(
        type(error),  # the type, for readers of older code
        error,
        error.__traceback__,
    )
```

Parentheses around the replaced arguments go with them, so the rewrite leaves valid code:

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import traceback


def report(error):
    return traceback.format_exception_only((type(error)), (error))
```

```python fixed
import traceback


def report(error):
    return traceback.format_exception_only(error)
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
