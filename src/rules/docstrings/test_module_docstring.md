Flags a module docstring in a test module.

## Why is this bad?

A summary such as “Tests for the config loader” repeats the module name `test_loader.py`. Put any setup requirement or
other detail near the tests that depend on it.
[Google's style guide](https://google.github.io/styleguide/pyguide.html#3821-test-modules) makes module docstrings
optional for tests.

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
single string. When simplint can delete the docstring, it offers a fix. If it cannot, for example because of a
file-level `# ruff: noqa`, it leaves the report to D419 even when D419 is disabled.

## What it does

Checks for a module docstring in a test module.

### Active on

- **Test filenames.** simplint recognizes any checked file whose name starts with `test_` or ends in `_test.py`. This
  includes pytest's default
  [test discovery patterns](https://docs.pytest.org/en/stable/explanation/goodpractices.html#conventions-for-python-test-discovery).

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

- **Test directories.** simplint checks files under `tests` or `test` in the project, including nested projects rooted
  in a directory with one of these names. It also recognizes additional pytest `python_files` patterns within configured
  `testpaths`.

  ```python report file=tests/helpers.py
  """Helpers for the tests."""
  ```

  ```python ok file=app/config.py
  """Load the config file."""
  ```

### Exceptions

- **Copyright and license notices.** The rule preserves docstrings containing `copyright`, `©`,
  `SPDX-License-Identifier`, `licensed under` or `permission is hereby granted`, regardless of case.

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

- [Ruff D100](https://docs.astral.sh/ruff/rules/undocumented-public-module/) asks for a docstring on public modules.
  [Ruff issue #10645](https://github.com/astral-sh/ruff/issues/10645) proposes an exception for tests.
