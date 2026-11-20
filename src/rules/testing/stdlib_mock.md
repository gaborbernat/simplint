Flags imports from [`unittest.mock`](https://docs.python.org/3/library/unittest.mock.html#module-unittest.mock) in a
test module of a project that depends on pytest-mock.

## Why is this bad?

The `mocker` fixture exposes mock constructors and automatically stops patches it starts. For a test that already uses
pytest-mock, `mocker.patch` avoids a nested `with` block or an extra test parameter for each patch decorator. See
[pytest-mock's comparison](https://pytest-mock.readthedocs.io/en/latest/remarks.html#why-bother-with-a-plugin).

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
  `from unittest import mock` and `from unittest.mock import ...`, which bring the module in whatever the name. See
  [test-class](SMP801.html) for which files count as test modules. The project depends on pytest-mock when any of these
  lists `pytest-mock`, with or without an environment marker:

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

Add the `mocker` fixture to the test and update its calls by hand.

## Ruff overlap

Ruff's [banned-api](https://docs.astral.sh/ruff/rules/banned-api/) (TID251) reports an import that `banned-api` bans, by
its module or a parent of it or by the full name of every member it brings in, and this rule leaves those imports to it,
whether or not the project selects TID251. A module ban covers an import of that module or a submodule. A member ban
covers that member only, so an import that also brings in a mock member no ban names keeps this rule's report.

## Other tools

- Ruff's [UP026](https://docs.astral.sh/ruff/rules/deprecated-mock-import/) moves `mock` imports to `unittest.mock`,
  which this rule then reports.
