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

- **`from unittest.mock import create_autospec` alone**: making a mock from a real object is separate from managing a
  patch. An import that also brings in `patch` or another mock helper is still reported.

  ```toml config
  [project]
  name = "demo"
  dependencies = ["pytest-mock>=3"]
  ```

  ```python ok file=tests/test_client.py
  from unittest.mock import create_autospec

  from app.client import Client


  def test_client() -> None:
      client = create_autospec(Client)
  ```

- **Relative imports**, such as `from .unittest import mock`: they name a module of the project, not the standard
  library.

  ```toml config
  [project]
  name = "demo"
  dependencies = ["pytest-mock>=3"]
  ```

  ```python ok file=tests/test_client.py
  from .unittest import mock


  def test_client() -> None:
      client = mock.MagicMock()
  ```

### Fix

No fix is offered: a test that moves to `mocker` changes its fixtures and its calls to the patched objects, which is a
rewrite of the test.

## Ruff overlap

Ruff's [banned-api](https://docs.astral.sh/ruff/rules/banned-api/) (TID251) reports an import that `banned-api` bans, by
its module or a parent of it or by the full name of every member it brings in, and this rule leaves those imports to it,
whether or not the project selects TID251. A module ban covers an import of that module or a submodule. A member ban
covers that member only, so an import that also brings in a mock member no ban names keeps this rule's report.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
  [UP026](https://docs.astral.sh/ruff/rules/deprecated-mock-import/) moves `mock` imports to `unittest.mock`, which this
  rule then reports.
