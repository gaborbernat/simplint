Flags attribute references to deprecated standard library names that name a replacement.

## Why is this bad?

Each name warns when it is used, or is gone in a later Python release, and a replacement does the same work. A project
that targets the version where the name is deprecated or removed should use the replacement, since the old name fails
there or keeps warning on every use.

## Example

```toml config
[project]
requires-python = ">=3.12"
```

```python report
import calendar
import threading


def describe(month):
    worker = threading.currentThread()
    return worker.name, calendar.January == month
```

Use instead:

```python ok
import calendar
import threading


def describe(month):
    worker = threading.current_thread()
    return worker.name, calendar.JANUARY == month
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py312"
  ```

  ```python report
  import threading


  def worker():
      return threading.currentThread()
  ```

## What it does

### Active on

Each name is reported from the Python version in its table row, where it is deprecated or removed, when the code reads
it. A name imported with `from module import name` is not reported, as renaming it would also rename its uses.

- **Renames**: the attribute is replaced with the new name, which does the same work. **Safe fix**, except for the
  `calendar` months, where it is unsafe.

  | Name                                    | From | Replacement                                                                                                                                                                      |
  | --------------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | `threading.currentThread`               | 3.10 | [`threading.current_thread`](https://docs.python.org/3/library/threading.html#threading.current_thread)                                                                          |
  | `threading.activeCount`                 | 3.10 | [`threading.active_count`](https://docs.python.org/3/library/threading.html#threading.active_count)                                                                              |
  | `calendar.January`, `calendar.February` | 3.12 | [`calendar.JANUARY`](https://docs.python.org/3/library/calendar.html#calendar.JANUARY), [`calendar.FEBRUARY`](https://docs.python.org/3/library/calendar.html#calendar.FEBRUARY) |
  | `configparser.SafeConfigParser`         | 3.12 | [`configparser.ConfigParser`](https://docs.python.org/3/library/configparser.html#configparser.ConfigParser)                                                                     |

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  import calendar


  def is_february(month):
      return month == calendar.February
  ```

  ```python fixed
  import calendar


  def is_february(month):
      return month == calendar.FEBRUARY
  ```

- **Replacements that need a review**: the replacement works differently, so the rule names it and offers no fix.

  | Name                                                                        | From | Replacement                                                                                             |
  | --------------------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------- |
  | `pkgutil.find_loader`, `pkgutil.get_loader`                                 | 3.12 | [`importlib.util.find_spec`](https://docs.python.org/3/library/importlib.html#importlib.util.find_spec) |
  | `unittest.findTestCases`, `unittest.makeSuite`, `unittest.getTestCaseNames` | 3.11 | [`unittest.TestLoader`](https://docs.python.org/3/library/unittest.html#unittest.TestLoader) methods    |
  | `glob.glob0`, `glob.glob1`                                                  | 3.13 | [`glob.glob`](https://docs.python.org/3/library/glob.html#glob.glob)                                    |

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  import pkgutil


  def has_module(name):
      return pkgutil.find_loader(name) is not None
  ```

### Fix

`threading.currentThread` warns and returns `threading.current_thread()`, so the rename only drops the warning, and
`configparser.SafeConfigParser` is gone from the versions the rule reports, so no code that runs changes. Both fixes are
safe.

The `calendar` fix is unsafe: `calendar.January` is the plain integer `1`, while `calendar.JANUARY` is the
[`calendar.Month`](https://docs.python.org/3/library/calendar.html#calendar.Month)`.JANUARY` member. The two compare
equal, but [`repr()`](https://docs.python.org/3/builtins/functions.html#repr) and
[`type()`](https://docs.python.org/3/builtins/functions.html#type) differ, which shows in output and in
`type(value) is `[`int`](https://docs.python.org/3/builtins/functions.html#int) checks.

No fix is offered for replacements that need a review.

### Exceptions

- **Methods such as `Thread.setDaemon()`, `Event.isSet()`, `Condition.notifyAll()` and `RawConfigParser.readfp()`**: the
  receiver's type is not known, and names such as `isSet` and `getName` belong to many classes, so they are not
  reported.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  def ready(task):
      return task.isSet()
  ```

- **Assignments and `del`**: `threading.activeCount = ...` patches the old name for code that still reads it.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  import threading


  def patch(counter):
      threading.activeCount = counter
  ```

- **[`sys.last_value`](https://docs.python.org/3/library/sys.html#sys.last_value),
  [`sys.last_type`](https://docs.python.org/3/library/sys.html#sys.last_type) and
  [`sys.last_traceback`](https://docs.python.org/3/library/sys.html#sys.last_traceback)**: the documentation deprecates
  them for [`sys.last_exc`](https://docs.python.org/3/library/sys.html#sys.last_exc), but they never warn, and code that
  sets them for tools that read them, or reads them when `sys.last_exc` is missing, keeps them on purpose.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  import sys


  def last_error():
      return sys.last_exc if hasattr(sys, "last_exc") else sys.last_value
  ```

- **Targets before the version in the table**: the name still works without a warning there.

  ```toml config
  [tool.ruff]
  target-version = "py311"
  ```

  ```python ok
  import calendar


  def describe(month):
      return calendar.January == month
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#970](https://github.com/astral-sh/ruff/issues/970)
  (pylint `deprecated-method`) and [#1507](https://github.com/astral-sh/ruff/issues/1507). Ruff's pyupgrade rules such
  as [UP005](https://docs.astral.sh/ruff/rules/deprecated-unittest-alias/) cover other deprecated names, not these.
- [pylint](https://github.com/pylint-dev/pylint) reports calls to the same deprecated standard library names as
  [deprecated-method](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/deprecated-method.html), with
  no fix.
