Flags a patch that a `unittest.TestCase` method starts by hand and stops with `addCleanup`. `self.enterContext` starts
the patch and registers its stop in one call. It exists from Python 3.11.

## Why is this bad?

Starting a patch, then registering its stop, takes three statements that must stay in step. Forgetting the cleanup leaks
the patch into other tests. `enterContext` enters the context manager and cleans it up with the test, in the order the
test case expects, so the statement that starts the patch is the one that stops it.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.11"
```

```python report
from unittest import TestCase, mock


class ConfigTest(TestCase):
    def test_home(self):
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)
        home.return_value = "/home"
```

```python fixed
from unittest import TestCase, mock


class ConfigTest(TestCase):
    def test_home(self):
        home = self.enterContext(mock.patch("os.getcwd"))
        home.return_value = "/home"
```

## Configuration

- The Python version the code targets, taken from
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`, falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py310"
  ```

  ```python ok
  from unittest import TestCase, mock


  class ConfigTest(TestCase):
      def test_home(self):
          patcher = mock.patch("os.getcwd")
          home = patcher.start()
          self.addCleanup(patcher.stop)
          home.return_value = "/home"
  ```

## What it does

### Active on

- **Three consecutive statements in a `TestCase` method**: `patcher = patch(...)`, `value = patcher.start()` and
  `self.addCleanup(patcher.stop)`. The patch can come from `mock.patch`, `mock.patch.object` or `mock.patch.dict`, under
  `unittest.mock` or the `mock` name, as the module resolves them. The report sits on the first statement.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  from unittest import TestCase, mock


  class ConfigTest(TestCase):
      def test_home(self):
          patcher = mock.patch.dict("os.environ", {"HOME": "/home"})
          home = patcher.start()
          self.addCleanup(patcher.stop)
  ```

### Exceptions

- **Classes that do not visibly derive from `unittest.TestCase`**: a base under another name may define `enterContext`
  differently or not at all.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python ok
  from third_party import Base
  from unittest import mock


  class ConfigTest(Base):
      def test_home(self):
          patcher = mock.patch("os.getcwd")
          home = patcher.start()
          self.addCleanup(patcher.stop)
  ```

### Fix

The fix is safe when the patcher name appears only in the three statements. It rewrites them as one assignment of
`self.enterContext(...)` to the value's name, and keeps the indentation. A patcher name that other code in the file
uses, a sequence with a comment in it, or statements that share a line with other code are reported without a fix.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
