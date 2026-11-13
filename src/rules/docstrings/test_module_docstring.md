Flags a module docstring in a test module.

## Why is this bad?

The module path and the test function names already say what is under test. The docstring repeats them and goes stale as
tests move between modules. The [Google style guide](https://google.github.io/styleguide/pyguide.html#3821-test-modules)
does not require docstrings on test modules, and a [Ruff issue](https://github.com/astral-sh/ruff/issues/10645) asks for
D100 to skip test modules.

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

## Ruff overlap

Ruff's [empty-docstring](https://docs.astral.sh/ruff/rules/empty-docstring/) (D419) reports an empty docstring of a
single string. This rule removes such a docstring with a fix, unless a file-level `# ruff: noqa` withdraws the fix; then
it leaves the docstring to D419, whether or not the project selects that rule.

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

### Exceptions

- **Docstrings with a copyright or license notice**: code copied from another project must keep the notice its license
  asks for, such as a `Copyright` line, an `SPDX-License-Identifier` or the MIT permission text.

  ```python ok file=tests/test_set.py
  """Adapted from the pyrsistent test suite.

  Copyright (c) 2022 Tobias Gustafsson. Permission is hereby granted, free of charge, to any person obtaining a copy.
  """
  ```

### Fix

The fix deletes the docstring and the blank lines after it. A docstring that shares its line with the next statement
goes up to that statement. A file with a file-level `ruff: noqa` keeps its docstring: the codes it names may be silenced
by the docstring's own text, and removing the docstring would leave them unused.

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

A comment after the docstring stays on a line of its own, unless it is a pragma or tool directive, such as `# fmt: off`
or `# noqa: E501`, which goes with the docstring.

```python report file=test_net.py
"""Network tests."""  # these need the staging VPN

import os
```

```python fixed file=test_net.py
# these need the staging VPN
import os
```

No fix is offered when a comment sits between the docstring and a statement sharing its line, or when nothing follows it
but a semicolon.

```python report file=test_empty.py
# fmt: off
"""Tests for the empty file.""";
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** conflicts with released rule
  [D100](https://docs.astral.sh/ruff/rules/undocumented-public-module/), which asks for the docstring this rule removes;
  open issue [#10645](https://github.com/astral-sh/ruff/issues/10645) asks
  [D100](https://docs.astral.sh/ruff/rules/undocumented-public-module/) to skip test modules.
