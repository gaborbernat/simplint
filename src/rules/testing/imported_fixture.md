Flags a pytest fixture that another module imports, or that its module lists in `__all__`.

## Why is this bad?

A fixture defined in `conftest.py` is available to tests in that directory and its subdirectories. A plugin can also
provide fixtures. If a test module imports a fixture, pytest can discover it there, but the test requests it through a
parameter. The import then looks unused. Removing it can make the test fail with a missing fixture. Listing a fixture in
`__all__` encourages the same import. See
[pytest's fixture lookup rules](https://docs.pytest.org/en/stable/reference/fixtures.html#fixture-availability) and
[Ruff's unused-import rule](https://docs.astral.sh/ruff/rules/unused-import/).

## Example

```python context file=tests/test_orders.py
from tests.helpers import order


def test_order_id(order: str) -> None:
    assert order == "order-1"
```

```python report file=tests/helpers.py
import pytest


@pytest.fixture
def order() -> str:
    return "order-1"
```

Use instead:

```python context file=tests/test_orders.py
def test_order_id(order: str) -> None:
    assert order == "order-1"
```

```python ok file=tests/conftest.py
import pytest


@pytest.fixture
def order() -> str:
    return "order-1"
```

## What it does

### Active on

- **Fixtures another module imports**: a module-level function decorated with `pytest.fixture` that another file in the
  run imports by name, as in the example above.

- **Fixtures `__all__` lists**: the module offers the fixture for import.

  ```python report file=tests/helpers.py
  import pytest

  __all__ = ["order"]


  @pytest.fixture
  def order() -> str:
      return "order-1"
  ```

### Related rules

[missing-from-all](SMP401.html) leaves fixtures out of the names it asks `__all__` to list.
