## What it does

Checks for a module docstring in a test module. A test module is a file named `test_*.py`, `*_test.py` or
[`conftest.py`](https://docs.pytest.org/en/stable/reference/fixtures.html#conftest-py-sharing-fixtures-across-multiple-files),
or any file in a directory named `tests` or `test` below the project root, the directory holding `pyproject.toml`. The
first two patterns are pytest's default
[test discovery](https://docs.pytest.org/en/stable/explanation/goodpractices.html#conventions-for-python-test-discovery)
names.

Other modules are checked by [SMP003](SMP003.html). This rule conflicts with Ruff's
[undocumented-public-module (D100)](https://docs.astral.sh/ruff/rules/undocumented-public-module/).

## Why is this bad?

The module path and the test function names already say what is under test. The docstring repeats them and goes stale as
tests move between modules.

## Example

```python
"""Tests for the config loader."""

from app.config import load_config


def test_load_config_empty_file(tmp_path: Path) -> None: ...
```

Use instead:

```python
from app.config import load_config


def test_load_config_empty_file(tmp_path: Path) -> None: ...
```

## Fix safety

The fix deletes the docstring and the blank lines after it. It is unsafe, since code may read the docstring through
`__doc__`: a package under `tests` built as a fixture can use its docstring as its description. A docstring that shares
its line with the next statement goes up to that statement; no fix is offered when a comment sits between them, or when
nothing follows it but a semicolon.

## Conflicts with Ruff

Ruff's [undocumented-public-module (D100)](https://docs.astral.sh/ruff/rules/undocumented-public-module/) asks for the
docstring this rule removes, since a test module is public unless its name starts with `_`. While D100 applies to test
files, simplint withdraws the fix and keeps the report; turn D100 off for them with
[`per-file-ignores`](https://docs.astral.sh/ruff/settings/#lint_per-file-ignores).
