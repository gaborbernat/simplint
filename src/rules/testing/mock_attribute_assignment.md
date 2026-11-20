Flags attributes set on a mock right after it is created.

## Why is this bad?

Setting attributes after construction spreads the mock's configuration over several statements.
[`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock) and its subclasses accept attributes
such as `return_value` and `side_effect` as keyword arguments.
[`create_autospec`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.create_autospec) passes extra
keywords to the mock it creates.

## Example

```python report
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user)
fetch.return_value = user
fetch.side_effect = None
```

Use instead:

```python ok
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user, return_value=user, side_effect=None)
```

## What it does

### Active on

- **`mock.attr = value` statements directly after the mock is created**: the mock must be assigned to a plain name, with
  or without an annotation, from a call to
  [`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock), `MagicMock`, `AsyncMock`,
  `NonCallableMock`, `NonCallableMagicMock` or
  [`create_autospec`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.create_autospec). The call must
  resolve to [`unittest.mock`](https://docs.python.org/3/library/unittest.mock.html#module-unittest.mock), the `mock`
  backport or a parameter annotated as `pytest_mock.MockerFixture`. Only that run of statements is reported, since each
  one could be a keyword argument of the call.

  - A `create_autospec` call, with `return_value` and `side_effect` set on the following lines

    ```python report
    from unittest.mock import create_autospec

    fetch = create_autospec(fetch_user)
    fetch.return_value = user
    fetch.side_effect = None
    ```

  - A `MagicMock` call assigned to a plain name

    ```python report
    from unittest.mock import MagicMock

    client = MagicMock()
    client.timeout = 5
    ```

  - A fixture parameter annotated as `MockerFixture`, here calling `mocker.MagicMock()`

    ```python report
    from pytest_mock import MockerFixture


    def test_client(mocker: MockerFixture):
        client = mocker.MagicMock()
        client.timeout = 5
    ```

### Exceptions

The run stops at:

- **Any other statement, including `mock.child.attr = value`**: it is no longer a plain attribute of the new mock.

  ```python ok
  from unittest.mock import MagicMock

  client = MagicMock()
  client.child.attr = 1
  ```

- **An attribute the constructor treats specially**: `cls`, `self`, `spec`, `spec_set`, `wraps`, `name`, `parent`,
  `unsafe`, `instance`, `_name`, `_parent`, `_spec_state`, `_new_name`, `_new_parent`, `_spec_as_instance` or
  `_eat_self`. These constructor arguments configure the mock itself, and `cls` and `self` are the constructor's own
  parameters, so passing one differs from setting the attribute or fails.

  ```python ok
  from unittest.mock import MagicMock

  client = MagicMock()
  client.name = "api"
  ```

  ```python ok
  from unittest.mock import MagicMock

  request = MagicMock()
  request.cls = int
  ```

  ```python report
  from unittest.mock import MagicMock

  request = MagicMock()
  request.timeout = 5
  request.cls = int
  ```

  ```python fixed
  from unittest.mock import MagicMock

  request = MagicMock(timeout=5)
  request.cls = int
  ```

- **An attribute already set earlier in the run**: a call can pass each keyword only once, so the run stops at the
  second assignment. A later round of fixes folds it in, replacing the keyword's value.

  ```python report
  from unittest.mock import MagicMock

  client = MagicMock()
  client.timeout = 5
  client.timeout = 6
  ```

  ```python fixed
  from unittest.mock import MagicMock

  client = MagicMock(timeout=6)
  ```

- **A value that refers to the mock itself**: the mock does not exist yet while its constructor's arguments run.

  ```python ok
  from unittest.mock import MagicMock

  client = MagicMock()
  client.child = client.copy()
  ```

### Fix

The fix moves the assignments into the constructor call as keyword arguments and deletes them. An assignment to a
keyword the call already passes replaces that keyword's value.

```python report
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user)
fetch.return_value = user
fetch.side_effect = None
```

```python fixed
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user, return_value=user, side_effect=None)
```

```python report
from unittest.mock import MagicMock

client = MagicMock(timeout=1)
client.timeout = 5
```

```python fixed
from unittest.mock import MagicMock

client = MagicMock(timeout=5)
```

A comment after the last argument does not hide its comma, so no second comma is added:

```python report
from unittest.mock import MagicMock

# fmt: off
client = MagicMock(
    spec=dict,  # the real type
)
client.timeout = 3
# fmt: on
```

```python fixed
from unittest.mock import MagicMock

# fmt: off
client = MagicMock(
    spec=dict,  # the real type
 timeout=3)
# fmt: on
```

**Unsafe fix** for a `create_autospec` call that sets an attribute other than `return_value` or `side_effect`: when the
spec is a function, `create_autospec` returns a wrapper function and hands the other keywords to the mock inside it, so
the test no longer sees them on the object it holds.

```python report
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user)
fetch.call_count = 0
```

```python fixed
from unittest.mock import create_autospec

fetch = create_autospec(fetch_user, call_count=0)
```

No fix is offered when:

- the call has a `**kwargs` argument, which may already carry any of the attributes, so the merged call could repeat a
  keyword;

  ```python report
  from unittest.mock import MagicMock

  client = MagicMock(**options)
  client.timeout = 5
  ```

- an existing keyword to be replaced has a value other than a literal, so dropping it could change what runs;

- an assignment shares a line with another statement;

- a comment sits on its lines.

  ```python report
  from unittest.mock import MagicMock

  client = MagicMock()
  client.timeout = 5  # seconds
  ```
