Flags reads of deprecated or removed standard library attributes with known replacements.

## Why is this bad?

A removed attribute raises [`AttributeError`](https://docs.python.org/3/builtins/exceptions.html#AttributeError). Some
older functions emit [`DeprecationWarning`](https://docs.python.org/3/builtins/exceptions.html#DeprecationWarning) when
called. Python normally [hides those warnings](https://docs.python.org/3/library/warnings.html#default-warning-filter)
outside `__main__`, but a test suite can treat them as errors.

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

The table gives the first target version where simplint reports each attribute. The rule checks reads such as
`threading.currentThread` and skips direct imports such as `from threading import currentThread`.

- **Renames.** simplint can replace the attribute name. It marks the calendar month fixes as unsafe.

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

Calling `threading.currentThread()` or `threading.activeCount()` emits a warning and calls its snake_case counterpart.
Python 3.12 removed `configparser.SafeConfigParser`. simplint marks these renames as safe.

The calendar fix changes the type. For example, `calendar.January` is the integer `1`, and `calendar.JANUARY` is a
member of [`calendar.Month`](https://docs.python.org/3/library/calendar.html#calendar.Month). They compare equal, but
have different representations and give different results for `type(value) is int`.

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

- **Earlier targets.** The rule checks each name from the version listed in the table. For example, `SafeConfigParser`
  already warned before Python 3.12, when it was removed.

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

- [Ruff issue #970](https://github.com/astral-sh/ruff/issues/970) tracks Pylint compatibility.
  [Issue #1507](https://github.com/astral-sh/ruff/issues/1507) discusses using banned APIs for deprecation checks.
  Ruff's [UP005](https://docs.astral.sh/ruff/rules/deprecated-unittest-alias/) handles deprecated unittest assertion
  names.
- Pylint's
  [deprecated-method](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/deprecated-method.html)
  reports calls to deprecated methods.
