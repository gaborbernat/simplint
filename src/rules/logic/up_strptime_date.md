Flags `datetime.strptime(...).date()` and `datetime.strptime(...).time()`, which Python 3.14 replaces with
`date.strptime` and `time.strptime`.

## Why is this bad?

`datetime.strptime(text, format).date()` builds a full datetime to keep one part of it. From Python 3.14 `date` and
`time` parse their own part, which says what the code reads and skips the intermediate value.

## Example

```toml config
[project]
requires-python = ">=3.14"
```

```python report
from datetime import datetime


def parse_day(text):
    return datetime.strptime(text, "%Y-%m-%d").date()
```

Use instead:

```python ok
from datetime import date


def parse_day(text):
    return date.strptime(text, "%Y-%m-%d")
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  from datetime import datetime


  def parse_day(text):
      return datetime.strptime(text, "%Y-%m-%d").date()
  ```

## What it does

Reports a call `.date()` or `.time()` with no arguments on a call that resolves to `datetime.datetime.strptime`, whether
the class is imported by name or reached through the `datetime` module.

### Active on

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import datetime


def parse_day(text):
    return datetime.datetime.strptime(text, "%Y-%m-%d").date()
```

```python fixed
import datetime


def parse_day(text):
    return datetime.date.strptime(text, "%Y-%m-%d")
```

### Exceptions

- **Target versions before 3.14**: `date.strptime` does not exist there.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import datetime


  def parse_day(text):
      return datetime.datetime.strptime(text, "%Y-%m-%d").date()
  ```

### Fix

The fix is safe for `.date()`. For `.time()` it is safe when the format is a string literal without `%z`, and unsafe
otherwise: `time.strptime` keeps the zone that `.time()` drops, so the value changes with `%z`. When the class is
imported by name, the fix adds `date` or `time` to that `from datetime import` line. It reports without a fix when the
file already spells `date` or `time` outside an attribute, since the new name would take that binding's place.
