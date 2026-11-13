Flags a test, a fixture or a method of a test class that changes process-wide state by hand: an environment variable,
the working directory, `sys.path`, `sys.argv`, `sys.modules`, or an attribute of an imported module.

## Why is this bad?

All tests of a run share one process, so a change a test makes by hand outlives it and reaches every test that runs
after it. Those tests then pass or fail depending on the order they run in, and on how
[pytest-xdist](https://pytest-xdist.readthedocs.io/) spreads them across workers. The
[`monkeypatch`](https://docs.pytest.org/en/stable/how-to/monkeypatch.html) fixture makes the same changes and undoes
them when the test or fixture that requested it finishes, and
[`mock.patch.dict`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.dict) does the same for a
`with` block. Adam Johnson's
[How to mock environment variables with pytest](https://adamj.eu/tech/2020/10/13/how-to-mock-environment-variables-with-pytest/)
walks through both.

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

- **Environment variables**: an item assigned to or deleted from `os.environ`, a call of its `update`, `pop`, `popitem`,
  `setdefault` or `clear`, and `os.putenv` or `os.unsetenv`.

- **The working directory**: `os.chdir`.

- **`sys.path`, `sys.argv` and `sys.modules`**: assigning them or an item of them, deleting an item, and the methods
  that change a list or a dictionary in place, such as `sys.path.insert` or `sys.modules.pop`.

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
since `setup_method` and its kin run as part of each test.

### Exceptions

- **Changes something puts back**: a function with a `finally`, a fixture that yields, a function that registers a
  cleanup with `addCleanup`, `addfinalizer`, `enterContext` or `ExitStack.callback`, and a change inside a `with` block
  of any context manager other than `open`, such as `mock.patch.dict` or a project's own `fresh_env()`.

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

- **State the function saved first**: `os.getcwd()` saves the working directory, and a copy such as `sys.path[:]`,
  `list(sys.argv)`, `dict(os.environ)` or `sys.modules.copy()`, or reading `module.attribute` into a name, saves the
  rest, so the function can put it back.

- **State a patcher or `monkeypatch` puts back**: `patch.dict(os.environ)` or `mocker.patch.dict(sys.modules)`, as a
  decorator or a call in the function, restores the whole mapping once the test ends, and `monkeypatch.setenv`,
  `delenv`, `syspath_prepend` or `chdir` restores what it replaced.

  ```python ok file=tests/test_patched_env.py
  import os
  from unittest import mock


  @mock.patch.dict(os.environ)
  def test_reads_override() -> None:
      os.environ["APP_MODE"] = "debug"
      assert os.environ["APP_MODE"] == "debug"
  ```

- **Modules, classes and tests with their own teardown**: a module with `teardown_module`, `teardown_function` or an
  autouse fixture that restores what it changes, a class with `tearDown`, `teardown_method` or `teardown_class`, a
  `patch` decorator, or a base other than `unittest.TestCase`, whose hooks it inherits, and a test, class or module with
  a `usefixtures` mark, whose fixture may restore the state.

- **Tests that request `pytester` or `testdir`**: pytester snapshots the working directory, `sys.path` and
  `sys.modules`, and restores them after the test.

- **Rebinding `os.environ` itself**: Ruff's
  [assignment-to-os-environ](https://docs.astral.sh/ruff/rules/assignment-to-os-environ/) (B003) reports it.

### Fix

No fix is offered, as moving to `monkeypatch` adds a parameter to the test, and to a fixture that every caller of a
helper has to pass along.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [B003](https://docs.astral.sh/ruff/rules/assignment-to-os-environ/), which reports assigning `os.environ` itself and
  which this rule leaves to it; nothing reports the other changes.
