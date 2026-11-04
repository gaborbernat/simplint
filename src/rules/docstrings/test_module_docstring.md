Flags a module docstring in a test module. The module path and the test names already say what is under test, so the
docstring repeats them and goes stale.

## Why is this bad?

The module path and the test function names already say what is under test. The docstring repeats them and goes stale as
tests move between modules.

## Example

```python report file=tests/test_loader.py
"""Tests for the config loader."""

from app.config import load_config


def test_load_config_empty_file(tmp_path: Path) -> None: ...
```

Use instead:

```python ok file=tests/test_loader.py
from app.config import load_config


def test_load_config_empty_file(tmp_path: Path) -> None: ...
```

## Conflicts with Ruff

Ruff's [undocumented-public-module](https://docs.astral.sh/ruff/rules/undocumented-public-module/) asks for the
docstring this rule removes, since a test module is public unless its name starts with `_`, and
[undocumented-public-package](https://docs.astral.sh/ruff/rules/undocumented-public-package/) does the same for the
`__init__.py` of a test package.

simplint stops with an error before linting while either applies to a test module. Turn them off for test files with
[`per-file-ignores`](https://docs.astral.sh/ruff/settings/#lint_per-file-ignores), or turn this rule off.

## What it does

Checks for a module docstring in a test module.

### Active on

- **Files named `test_*.py` or `*_test.py`**: pytest's default
  [test discovery](https://docs.pytest.org/en/stable/explanation/goodpractices.html#conventions-for-python-test-discovery)
  names.

  ```python report file=test_loader.py
  """Tests for the config loader."""

  from app.config import load_config


  def test_load_config_empty_file(tmp_path: Path) -> None: ...
  ```

- **Files named
  [`conftest.py`](https://docs.pytest.org/en/stable/reference/fixtures.html#conftest-py-sharing-fixtures-across-multiple-files)**:
  where pytest shares fixtures across the test files of a directory.

  ```python report file=conftest.py
  """Fixtures shared by the tests."""
  ```

- **Any file in a directory named `tests` or `test` below the project root**: the project root is the directory holding
  `pyproject.toml`.

  ```python report file=tests/helpers.py
  """Helpers for the tests."""
  ```

  ```python ok file=app/config.py
  """Load the config file."""
  ```

### Fix

The fix deletes the docstring and the blank lines after it. A docstring that shares its line with the next statement
goes up to that statement.

**Unsafe fix:** code may read the docstring through `__doc__`: a package under `tests` built as a fixture can use its
docstring as its description.

```python report file=test_loader.py
"""Tests for the config loader."""

from app.config import load_config
```

```python fixed file=test_loader.py
from app.config import load_config
```

```python report file=test_probe.py
"""Tests for the probe."""

import os
```

```python fixed file=test_probe.py
import os
```

No fix is offered when a comment sits between the docstring and a statement sharing its line, or when nothing follows it
but a semicolon.

```python report file=test_empty.py
# fmt: off
"""Tests for the empty file.""";
```
