Flags a pytest fixture that another module imports, or that its module lists in `__all__`.

## Why is this bad?

pytest finds fixtures by name, through `conftest.py` files and plugins
([pytest docs](https://docs.pytest.org/en/stable/how-to/fixtures.html#scope-sharing-fixtures-across-classes-modules-packages-or-session)).
Importing a fixture into a test module bypasses that lookup: the import binds the fixture function, the test parameter
of the same name shadows it, and the import looks unused. Ruff's
[unused-import](https://docs.astral.sh/ruff/rules/unused-import/) then removes it, or a reader does, and the tests stop
finding the fixture. Listing a fixture in `__all__` invites that same import.

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
