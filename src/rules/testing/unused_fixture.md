Flags a pytest fixture when the lint run finds no request for its name.

## Why is this bad?

A fixture left behind after a test changes still looks like part of its setup. Request it in the test that needs it, or
remove it if no test uses it.

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

The rule looks for the fixture's registered name across all files in the lint run. In test modules, it counts function
parameters, imports, reads of `fixture.__name__` and string literals equal to the name. These include strings passed to
`usefixtures` or `getfixturevalue`. In other modules, it counts parameters of `test*` functions and fixtures. A matching
name anywhere in the run is enough, even if pytest would resolve it to a different fixture.

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

- **References to the function.** A test module that imports the fixture or reads `fixture.__name__` can use it
  indirectly.

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

- **Parameters outside test modules.** A `test*` function or fixture can request a fixture even in a file with a
  nonstandard name.

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
  that may request one the `conftest.py` overrides, by a name no file of the run spells. The rule still checks
  `conftest.py` if the only plugins listed are `pytester` or names starting with `_pytest`.

  ```python ok file=tests/conftest.py
  import pytest

  pytest_plugins = ["sphinx.testing.fixtures"]


  @pytest.fixture
  def sphinx_test_tempdir() -> str:
      return "/tmp"
  ```

- **Autouse fixtures**: pytest applies autouse fixtures to tests that can see them. The rule skips a fixture with an
  `autouse` argument unless it is the literal `False`.

  ```python ok file=tests/test_orders.py
  import pytest


  @pytest.fixture(autouse=True)
  def clock() -> None:
      return None


  def test_order_id() -> None:
      assert True
  ```

- **Fixtures outside test modules**: shared fixture libraries and plugins may have users outside the lint run. The rule
  checks definitions only in [test modules](SMP801.html).

  ```python ok file=app/fixtures.py
  import pytest


  @pytest.fixture
  def db_url() -> str:
      return "sqlite://"
  ```

### Fix

Check for plugin requests and names built at runtime before removing a fixture. If an installed plugin requests a
`conftest.py` override that simplint cannot see, suppress the report on that fixture. The rule accepts `anyio_backend`,
`event_loop_policy`, `django_db_setup`, `django_db_modify_db_settings`, `django_db_use_migrations`, `django_db_keepdb`
and `django_db_createdb` by name.

## Other tools

- [pytest-deadfixtures](https://github.com/jllorencetti/pytest-deadfixtures) lists the unused fixtures of a pytest run,
  with no fix.
