Flags a [`unittest.TestCase`](https://docs.python.org/3/library/unittest.html#unittest.TestCase) method that calls
`patcher.start()` and registers `patcher.stop` with `self.addCleanup`. The rule applies to projects targeting Python
3.11 or later.

## Why is this bad?

Forgetting to stop a patch can change what later tests see.
[`self.enterContext(patcher)`](https://docs.python.org/3/library/unittest.html#unittest.TestCase.enterContext) enters
the patch, registers its exit as a cleanup and returns the patched value in one call.

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
  `self.addCleanup(patcher.stop)`. The patch can come from
  [`mock.patch`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch),
  [`mock.patch.object`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.object) or
  [`mock.patch.dict`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.dict), under
  [`unittest.mock`](https://docs.python.org/3/library/unittest.mock.html#module-unittest.mock) or the `mock` name, as
  the module resolves them. The report sits on the first statement.

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

The fix replaces the three statements with `value = self.enterContext(...)`. It is unsafe because `patch.stopall()` only
stops patches started with `start()`. A patcher name that the method uses elsewhere, a sequence with a comment in it, or
statements that share a line with other code are reported without a fix.
