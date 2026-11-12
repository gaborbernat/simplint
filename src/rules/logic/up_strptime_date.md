Flags
[`datetime.strptime(...)`](https://docs.python.org/3/library/datetime.html#datetime.datetime.strptime).[`date()`](https://docs.python.org/3/library/datetime.html#datetime.datetime.date)
and `datetime.strptime(...)`.[`time()`](https://docs.python.org/3/library/datetime.html#datetime.datetime.time), which
Python 3.14 replaces with [`date.strptime`](https://docs.python.org/3/library/datetime.html#datetime.date.strptime) and
[`time.strptime`](https://docs.python.org/3/library/datetime.html#datetime.time.strptime).

## Why is this bad?

`datetime.strptime(text, format).date()` builds a full datetime to keep one part of it. From Python 3.14
[`date`](https://docs.python.org/3/library/datetime.html#datetime.date) and
[`time`](https://docs.python.org/3/library/datetime.html#datetime.time) parse their own part, which says what the code
reads and skips the intermediate value.

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

- **`.time()` with a format that has `%:z`**: the fix is unsafe, since `time.strptime` reads the zone that `.time()`
  drops. **Unsafe fix:** the call becomes `time.strptime`, which keeps the zone.

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

- **`.time()` with a format that has a date directive**: the fix is unsafe, since `time.strptime` does not check the
  date that `datetime.strptime` checks, so a day that does not exist is accepted. **Unsafe fix:** the call becomes
  `time.strptime`.

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

For `.date()` the fix is safe when the format is a string literal without a seconds directive, `%S`, or a composite that
includes seconds, `%X`, `%c`, `%T` or `%r`, and unsafe otherwise: `date.strptime` accepts the second 60 that
`datetime.strptime` rejects.

The `.time()` fix is safe when the format is a string literal whose directives are all time-of-day ones, `%H`, `%I`,
`%M`, `%f` and `%p`, with `%%`, and unsafe otherwise. `time.strptime` keeps the zone that `.time()` drops, does not
check the date that `datetime.strptime` checks, and accepts the second 60, so `%z`, `%:z`, a date directive and a
seconds directive change the result.

When the class is imported by name, the fix adds `date` or `time` to that `from datetime import` line. It reports
without a fix when the file already spells `date` or `time` outside an attribute, since the new name would take that
binding's place. No fix is offered when a comment lies in the class or in the `.date()` or `.time()` call it drops,
since the rewrite would drop it. A comment inside the `.date()` call keeps the report without a fix:

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Released rule
  [DTZ007](https://docs.astral.sh/ruff/rules/call-datetime-strptime-without-zone/) reports the same call for its missing
  timezone, which open issue [#13359](https://github.com/astral-sh/ruff/issues/13359) calls a false positive before
  `.date()`; this rule's fix clears both.
