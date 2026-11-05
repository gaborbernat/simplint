Flags `self.assertTrue` and `self.assertFalse` around a `startswith`, `endswith`, `hasattr` or `issubclass` check in a
`unittest.TestCase` subclass, where the test case has a helper that names the check. The helpers exist from Python 3.14.

## Why is this bad?

`assertTrue(value.startswith(prefix))` only reports that the check failed. The dedicated helper, `assertStartsWith`,
reports the string and the prefix too, so the failure reads without rerunning the test. The helper also takes the same
`msg` argument.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.14"
```

```python report
from unittest import TestCase


class ConfigTest(TestCase):
    def test_name(self):
        self.assertTrue(name.startswith("demo"))
```

```python fixed
from unittest import TestCase


class ConfigTest(TestCase):
    def test_name(self):
        self.assertStartsWith(name, "demo")
```

## Configuration

- The Python version the code targets, taken from
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`, falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py313"
  ```

  ```python ok
  from unittest import TestCase


  class ConfigTest(TestCase):
      def test_name(self):
          self.assertTrue(name.startswith("demo"))
  ```

## What it does

### Active on

- **`self.assertTrue` and `self.assertFalse` on one of these checks**, in a class that visibly derives from
  `unittest.TestCase`:

  | Check                 | `assertTrue`       | `assertFalse`         |
  | --------------------- | ------------------ | --------------------- |
  | `value.startswith(p)` | `assertStartsWith` | `assertNotStartsWith` |
  | `value.endswith(s)`   | `assertEndsWith`   | `assertNotEndsWith`   |
  | `hasattr(obj, name)`  | `assertHasAttr`    | `assertNotHasAttr`    |
  | `issubclass(c, base)` | `assertIsSubclass` | `assertNotIsSubclass` |

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from unittest import TestCase


  class ConfigTest(TestCase):
      def test_optional(self):
          self.assertFalse(hasattr(config, "debug"))
  ```

  ```python fixed
  from unittest import TestCase


  class ConfigTest(TestCase):
      def test_optional(self):
          self.assertNotHasAttr(config, "debug")
  ```

### Exceptions

- **Classes that do not visibly derive from `unittest.TestCase`**: a base under another name may define these helpers
  with other meanings.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python ok
  from third_party import Base


  class ConfigTest(Base):
      def test_name(self):
          self.assertTrue(name.startswith("demo"))
  ```

- **`startswith` and `endswith` with a start or end argument**: the helper takes the two operands only.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python ok
  from unittest import TestCase


  class ConfigTest(TestCase):
      def test_name(self):
          self.assertTrue(name.startswith("demo", 1))
  ```

### Fix

The fix is safe: it rewrites the call with the same operands and the `msg` argument, given as `msg=`. A call with a
comment in it is reported without a fix, because the rewrite would drop the comment.
