Flags imports from `unittest.mock` in a test module of a project that depends on pytest-mock.

## Why is this bad?

pytest-mock provides the `mocker` fixture, which undoes each patch when its test ends. A test that imports
`unittest.mock` must manage the patch itself, with a `with` block or a decorator, and the project then has two ways to
patch. Using `mocker` everywhere keeps one style and needs no cleanup code in the test.

## Example

```toml config
[project]
name = "demo"
dependencies = ["pytest-mock>=3"]
```

```python report file=tests/test_client.py
from unittest.mock import MagicMock


def test_client() -> None:
    client = MagicMock()
```

Use instead:

```python ok file=tests/test_client.py
from pytest_mock import MockerFixture


def test_client(mocker: MockerFixture) -> None:
    client = mocker.MagicMock()
```

## What it does

### Active on

- **Imports of `unittest.mock` in a test module of a project that depends on pytest-mock**: `import unittest.mock`,
  `from unittest import mock` and `from unittest.mock import ...`, which bring the module in whatever the name. A file
  is a test module when its name starts with `test_`, ends with `_test.py` or is `conftest.py`, or when it sits under a
  `tests` or `test` directory. The project depends on pytest-mock when any of these lists `pytest-mock`, with or without
  an environment marker:

  - [`project.dependencies`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies)

    ```toml config
    [project]
    name = "demo"
    dependencies = ["pytest-mock>=3"]
    ```

    ```python report file=tests/test_api.py
    import unittest.mock
    from unittest import mock
    from unittest.mock import MagicMock
    ```

  - [`project.optional-dependencies`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#optional-dependencies),
    in any extra

    ```toml config
    [project]
    name = "demo"

    [project.optional-dependencies]
    test = ["pytest-mock"]
    ```

    ```python report file=tests/test_api.py
    from unittest.mock import MagicMock
    ```

  - [`dependency-groups`](https://peps.python.org/pep-0735/), in any group, including one that another group includes

    ```toml config
    [dependency-groups]
    test = ["pytest-mock>=3", "pytest"]
    dev = [{ include-group = "test" }]
    ```

    ```python report file=tests/test_api.py
    from unittest.mock import MagicMock
    ```

### Exceptions

- **Relative imports**, such as `from .unittest import mock`: they name a module of the project, not the standard
  library.

### Fix

No fix is offered: a test that moves to `mocker` changes its fixtures and its calls to the patched objects, which is a
rewrite of the test.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
  [UP026](https://docs.astral.sh/ruff/rules/deprecated-mock-import/) moves `mock` imports to `unittest.mock`, which this
  rule then reports.
