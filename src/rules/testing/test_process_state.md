Flags a test, a fixture or a method of a test class that changes process-wide state by hand: an environment variable,
the working directory, [`sys.path`](https://docs.python.org/3/library/sys.html#sys.path),
[`sys.argv`](https://docs.python.org/3/library/sys.html#sys.argv),
[`sys.modules`](https://docs.python.org/3/library/sys.html#sys.modules), or an attribute of an imported module.

## Why is this bad?

Tests in the same process share the environment, working directory and imported modules. A test that leaves any of these
changed can affect the next test in that process. With [pytest-xdist](https://pytest-xdist.readthedocs.io/), the failure
can depend on which worker runs each test. The
[`monkeypatch` fixture](https://docs.pytest.org/en/stable/how-to/monkeypatch.html) restores the values it changed during
cleanup. For a dictionary, [`patch.dict`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.dict)
restores the mapping when its context exits.

## Example

```python report file=tests/test_settings.py
import os


def test_utc_timezone():
    os.environ["TZ"] = "UTC"
    assert load_settings().timezone == "UTC"
```

Use instead:

```python ok file=tests/test_settings_patched.py
def test_utc_timezone(monkeypatch):
    monkeypatch.setenv("TZ", "UTC")
    assert load_settings().timezone == "UTC"
```

## What it does

### Active on

- **Environment variables**: an item assigned to or deleted from
  [`os.environ`](https://docs.python.org/3/library/os.html#os.environ), a call of its `update`, `pop`, `popitem`,
  `setdefault` or `clear`, and [`os.putenv`](https://docs.python.org/3/library/os.html#os.putenv) or
  [`os.unsetenv`](https://docs.python.org/3/library/os.html#os.unsetenv).

- **The working directory**: [`os.chdir`](https://docs.python.org/3/library/os.html#os.chdir).

- **`sys.path`, `sys.argv` and `sys.modules`**: assigning them or an item of them, deleting an item, and calls to
  recognized mutating methods such as `sys.path.insert` or `sys.modules.pop`.

- **Module attributes**: an assignment to an attribute of a name that only ever binds an imported module, as
  `json.dumps = repr` does.

  ```python report file=tests/test_cli.py
  import json
  import os
  import sys


  def test_cli(tmp_path):
      os.chdir(tmp_path)
      sys.argv = ["tool", "--help"]
      json.dumps = repr
  ```

The functions checked are the `test*` functions of a test module, its fixtures, and every method of its test classes,
including setup and teardown methods.

### Exceptions

- **Possible cleanup.** The rule skips a function with a `finally`, a fixture that yields, a function that registers a
  cleanup with `addCleanup`, `addfinalizer`, `enterContext` or `ExitStack.callback`, and changes inside a `with` block
  that calls a context manager other than the builtin `open`, such as `mock.patch.dict` or a project's own
  `fresh_env()`.

  ```python ok file=tests/test_restored.py
  import os

  import pytest


  @pytest.fixture
  def utc():
      os.environ["TZ"] = "UTC"
      yield
      del os.environ["TZ"]


  def test_restored(tmp_path):
      previous = os.getcwd()
      try:
          os.chdir(tmp_path)
      finally:
          os.chdir(previous)
  ```

- **Possible saved state.** A call to [`os.getcwd()`](https://docs.python.org/3/library/os.html#os.getcwd) suppresses
  working-directory reports. Assigning a copy such as `sys.path[:]`, `list(sys.argv)`, `dict(os.environ)` or
  `sys.modules.copy()` suppresses reports for that object. Assigning an imported module's attribute to a variable
  suppresses module-attribute reports. The rule looks anywhere in the function.

- **Patches and `monkeypatch`.** A `patch.dict` call on `os.environ` or `sys.modules` suppresses reports for that
  mapping. Likewise, `monkeypatch.setenv` or `delenv` suppresses environment reports, `syspath_prepend` suppresses
  `sys.path` reports, and `chdir` suppresses working-directory reports. The rule groups changes by kind and does not
  verify that cleanup restores each individual change.

  ```python ok file=tests/test_patched_env.py
  import os
  from unittest import mock


  @mock.patch.dict(os.environ)
  def test_reads_override() -> None:
      os.environ["APP_MODE"] = "debug"
      assert os.environ["APP_MODE"] == "debug"
  ```

- **Teardown hooks and fixtures.** The rule skips a module with a teardown hook or an autouse fixture that may restore
  state. It skips a class with a teardown hook, a patch decorator or a base other than `object` or
  [`unittest.TestCase`](https://docs.python.org/3/library/unittest.html#unittest.TestCase). A `usefixtures` mark on a
  module, class or test also excludes its tests.

- **Tests that request `pytester` or `testdir`**: pytester snapshots the working directory, `sys.path` and
  `sys.modules`, and restores them after the test.

- **Rebinding `os.environ` itself**: Ruff's
  [assignment-to-os-environ](https://docs.astral.sh/ruff/rules/assignment-to-os-environ/) (B003) reports it.

### Fix

No fix is offered, as moving to `monkeypatch` adds a parameter to the test, and to a fixture that every caller of a
helper has to pass along.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [B003](https://docs.astral.sh/ruff/rules/assignment-to-os-environ/), which reports assignments to `os.environ` itself.
  This rule checks changes to individual entries.
