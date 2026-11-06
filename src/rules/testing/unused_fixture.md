Flags a pytest fixture that no test or other fixture requests, so pytest never runs it.

## Why is this bad?

A fixture that nothing requests is setup code that never runs, and it reads as if a test depends on it. When a test is
later rewritten to use the fixture, the author finds it already written but broken or stale. Removing it keeps the
fixtures a reader sees to the ones tests use.

## Example

```python report file=tests/test_orders.py
import pytest


@pytest.fixture
def order() -> str:
    return "order-1"


def test_order_id() -> None:
    assert len("order-1") == 7
```

Use instead:

```python ok file=tests/test_orders.py
import pytest


@pytest.fixture
def order() -> str:
    return "order-1"


def test_order_id(order: str) -> None:
    assert order == "order-1"
```

## What it does

A fixture is requested when a test or another fixture takes its name as a parameter, when a string in a test module
spells its name, as a `usefixtures` marker, a `getfixturevalue` call or a list of `params` does, when a test module
imports it or reads its `__name__`, or when the fixture is set with `name=` and requested by that name. Requests are
collected from every test module of the run, including `conftest.py` files, so a fixture in a `conftest.py` is used by
any test below it, and from the `test*` functions and fixtures of any other module, which pytest collects when its
`python_files` setting names the module.

### Active on

- **Fixtures in test modules**: a module-level function decorated with `pytest.fixture`, in a test module or a
  `conftest.py`, that no request in the run names.

  ```python report file=tests/conftest.py
  import pytest


  @pytest.fixture
  def db_url() -> str:
      return "sqlite://"
  ```

- **Fixtures named with `name=`**: the name the fixture is requested by is the one `name=` gives.

  ```python report file=tests/test_orders.py
  import pytest


  @pytest.fixture(name="order_id")
  def _order() -> str:
      return "order-1"


  def test_order_id() -> None:
      assert True
  ```

### Exceptions

- **Requested by a test or a fixture**: a parameter, a `usefixtures` marker or a `getfixturevalue` call keeps the
  fixture, wherever in the run it is written.

  ```python ok file=tests/conftest.py
  import pytest


  @pytest.fixture
  def db_url() -> str:
      return "sqlite://"
  ```

  ```python context file=tests/test_database.py
  def test_connect(db_url: str) -> None:
      assert db_url
  ```

- **Requested by another fixture**: a fixture a fixture takes as a parameter is in use.

  ```python ok file=tests/test_orders.py
  import pytest


  @pytest.fixture
  def base() -> str:
      return "order"


  @pytest.fixture
  def order(base: str) -> str:
      return base + "-1"


  def test_order_id(order: str) -> None:
      assert order == "order-1"
  ```

- **Requested by a name the test builds**: a fixture that picks another by `request.getfixturevalue(request.param)`
  requests every name its `params` spell.

  ```python ok file=tests/test_storage.py
  import pytest


  @pytest.fixture
  def sqlite_url() -> str:
      return "sqlite://"


  @pytest.fixture(params=["sqlite_url"])
  def url(request: pytest.FixtureRequest) -> str:
      return request.getfixturevalue(request.param)


  def test_url(url: str) -> None:
      assert url
  ```

- **Requested through the function**: a test module that imports the fixture, or passes `fixture.__name__` to
  `usefixtures`, requests it.

  ```python ok file=tests/fixtures.py
  import pytest


  @pytest.fixture
  def db_url() -> str:
      return "sqlite://"
  ```

  ```python context file=tests/test_database.py
  import pytest

  from tests.fixtures import db_url


  @pytest.mark.usefixtures(db_url.__name__)
  def test_connect() -> None:
      assert True
  ```

- **Requested from a module pytest collects by its settings**: a `test*` function or a fixture outside the test modules,
  in a module that `python_files` names, requests the fixtures it takes.

  ```python ok file=tests/conftest.py
  import pytest


  @pytest.fixture
  def runner() -> str:
      return "asgi"
  ```

  ```python context file=benchmarks/bench_requests.py
  def test_requests(runner: str) -> None:
      assert runner
  ```

- **Fixtures of a `conftest.py` when the tests load plugins**: a test module that sets `pytest_plugins` loads fixtures
  that may request one the `conftest.py` overrides, by a name no file of the run spells. pytest's own plugins, such as
  `pytester`, request none, so loading only them keeps nothing.

  ```python ok file=tests/conftest.py
  import pytest

  pytest_plugins = ["sphinx.testing.fixtures"]


  @pytest.fixture
  def sphinx_test_tempdir() -> str:
      return "/tmp"
  ```

- **Autouse fixtures**: pytest runs an `autouse` fixture for every test without a request, so it is never unused.

  ```python ok file=tests/test_orders.py
  import pytest


  @pytest.fixture(autouse=True)
  def clock() -> None:
      return None


  def test_order_id() -> None:
      assert True
  ```

- **Fixtures outside test modules**: a fixture in a module that is not a test module is not checked, since pytest only
  finds fixtures in test modules and `conftest.py` files.

  ```python ok file=app/fixtures.py
  import pytest


  @pytest.fixture
  def db_url() -> str:
      return "sqlite://"
  ```

### Fix

The rule has no fix. A fixture may be requested by a pytest plugin or by a name the run builds at run time, which the
rule cannot see, so the author decides whether to remove it. A plugin installed as a dependency, rather than named in
`pytest_plugins`, may request a `conftest.py` fixture that overrides one of its own, such as pytest-django's
`django_db_setup` or anyio's `anyio_backend`; suppress the report on such an override.

## Other tools

- [pytest-deadfixtures](https://github.com/jllorencetti/pytest-deadfixtures) lists the unused fixtures of a pytest run,
  with no fix.
