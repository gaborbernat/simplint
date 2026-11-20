Flags `datetime.strptime(...).date()` and `datetime.strptime(...).time()` on Python 3.14 and later.

## Why is this bad?

When only a calendar date or time of day is needed,
[`datetime.date.strptime()`](https://docs.python.org/3/library/datetime.html#datetime.date.strptime) and
[`datetime.time.strptime()`](https://docs.python.org/3/library/datetime.html#datetime.time.strptime) can construct the
requested type directly. Both methods were added in Python 3.14.

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

Reports a call `.date()` or `.time()` with no arguments on a call that resolves to
[`datetime.datetime.strptime`](https://docs.python.org/3/library/datetime.html#datetime.datetime.strptime), whether the
class is imported by name or reached through the `datetime` module.

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

- **UTC offset in a time format.** The unsafe fix switches to
  [`datetime.time.strptime()`](https://docs.python.org/3/library/datetime.html#datetime.time.strptime), which retains
  the offset that `.time()` discards.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  import datetime


  def parse_time(text):
      return datetime.datetime.strptime(text, "%H:%M:%:z").time()
  ```

  ```python fixed
  import datetime


  def parse_time(text):
      return datetime.time.strptime(text, "%H:%M:%:z")
  ```

- **Date directive in a time format.** simplint marks this fix as unsafe. For example, on Python 3.14, `%m-%d` without a
  year can parse February 29 into a time. Constructing a datetime additionally checks that date against the default
  year, 1900. From 3.15, `%d` without a year raises
  [`ValueError`](https://docs.python.org/3/builtins/exceptions.html#ValueError) in both calls.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  import datetime


  def parse_time(text):
      return datetime.datetime.strptime(text, "%m-%d %H:%M").time()
  ```

  ```python fixed
  import datetime


  def parse_time(text):
      return datetime.time.strptime(text, "%m-%d %H:%M")
  ```

- **`.date()` with a seconds directive**: the fix is unsafe, since `date.strptime` accepts the second 60 that
  `datetime.strptime` rejects. **Unsafe fix:** the call becomes `date.strptime`.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  import datetime


  def parse_day(text):
      return datetime.datetime.strptime(text, "%Y-%m-%d %H:%M:%S").date()
  ```

  ```python fixed
  import datetime


  def parse_day(text):
      return datetime.date.strptime(text, "%Y-%m-%d %H:%M:%S")
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

For `.date()`, simplint marks the fix as unsafe when the format is computed, ends in a lone `%`, or contains `%S`, `%X`,
`%c`, `%T` or `%r`. A date can discard a leap second that a datetime would reject. The rule marks other literal formats
as safe, even though an invalid offset such as `+2400` with `%z` can be discarded by `date.strptime()` and rejected by
`datetime.strptime()`.

For `.time()`, simplint marks the fix as safe only for literal formats using `%H`, `%I`, `%M`, `%f`, `%p`, `%%` or
ordinary text. For example, `%H:%M:%S` is marked unsafe, although both `datetime.time.strptime()` and
`datetime.datetime.strptime()` reject a second of 60.

When the class comes from a module-level `from datetime import datetime`, the fix adds `date` or `time` to that import.
It skips the edit if that name already appears unqualified in the file or if the import is local to a function. A
comment in the class expression or the removed suffix also prevents a fix.

```toml config
[project]
requires-python = ">=3.14"
```

```python report
from datetime import datetime


def parse_day(text):
    return datetime.strptime(text, "%Y-%m-%d").date(
        # the calendar day
    )
```

```python fixed
from datetime import datetime


def parse_day(text):
    return datetime.strptime(text, "%Y-%m-%d").date(
        # the calendar day
    )
```

## Other tools

- Ruff's [DTZ007](https://docs.astral.sh/ruff/rules/call-datetime-strptime-without-zone/) reports `datetime.strptime()`
  without timezone information. [Ruff issue #13359](https://github.com/astral-sh/ruff/issues/13359) discusses calls
  followed by `.date()`.
