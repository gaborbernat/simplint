Flags attributes set on a mock right after it is created. The mock constructors take any attribute as a keyword
argument, so the whole setup fits in one call.

## Why is this bad?

Setting attributes after construction spreads the mock's configuration over several statements. The mock constructors
take any attribute as a keyword argument and set it on the new mock, so the whole setup fits in one call.

## Example

```python report
fetch = create_autospec(fetch_user)
fetch.return_value = user
fetch.side_effect = None
```

Use instead:

```python ok
fetch = create_autospec(fetch_user, return_value=user, side_effect=None)
```

## What it does

### Active on

- **`mock.attr = value` statements directly after the mock is created**: the mock must be assigned to a plain name, with
  or without an annotation, from a call to
  [`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock), `MagicMock`, `AsyncMock`,
  `NonCallableMock`, `NonCallableMagicMock` or
  [`create_autospec`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.create_autospec). The call
  matches by the last part of its name, so `mocker.MagicMock()` counts too. Only that run of statements is reported,
  since each one could be a keyword argument of the call.

  - A `create_autospec` call, with `return_value` and `side_effect` set on the following lines

    ```python report
    fetch = create_autospec(fetch_user)
    fetch.return_value = user
    fetch.side_effect = None
    ```

  - A `MagicMock` call assigned to a plain name

    ```python report
    client = MagicMock()
    client.timeout = 5
    ```

  - An annotated assignment, here from `mocker.MagicMock()`

    ```python report
    client: MagicMock = mocker.MagicMock()
    client.timeout = 5
    ```

### Exceptions

The run stops at:

- **Any other statement, including `mock.child.attr = value`**: it is no longer a plain attribute of the new mock.

  ```python ok
  client = MagicMock()
  client.child.attr = 1
  ```

- **An attribute the constructor treats specially**: `spec`, `spec_set`, `wraps`, `name`, `parent`, `unsafe`,
  `instance`, `_name` or `_parent`. These constructor arguments configure the mock itself, so passing one differs from
  setting the attribute.

  ```python ok
  client = MagicMock()
  client.name = "api"
  ```

- **An attribute already set earlier in the run**: a call can pass each keyword only once, so the run stops at the
  second assignment. A later round of fixes folds it in, replacing the keyword's value.

  ```python report
  client = MagicMock()
  client.timeout = 5
  client.timeout = 6
  ```

  ```python fixed
  client = MagicMock(timeout=6)
  ```

- **A value that refers to the mock itself**: the mock does not exist yet while its constructor's arguments run.

  ```python ok
  client = MagicMock()
  client.child = client.copy()
  ```

### Fix

The fix moves the assignments into the constructor call as keyword arguments and deletes them. An assignment to a
keyword the call already passes replaces that keyword's value.

```python report
fetch = create_autospec(fetch_user)
fetch.return_value = user
fetch.side_effect = None
```

```python fixed
fetch = create_autospec(fetch_user, return_value=user, side_effect=None)
```

```python report
client = MagicMock(timeout=1)
client.timeout = 5
```

```python fixed
client = MagicMock(timeout=5)
```

A comment after the last argument does not hide its comma, so no second comma is added:

```python report
# fmt: off
client = MagicMock(
    spec=dict,  # the real type
)
client.timeout = 3
# fmt: on
```

```python fixed
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
fetch = create_autospec(fetch_user)
fetch.call_count = 0
```

```python fixed
fetch = create_autospec(fetch_user, call_count=0)
```

No fix is offered when:

- the call has a `**kwargs` argument, which may already carry any of the attributes, so the merged call could repeat a
  keyword;

  ```python report
  client = MagicMock(**options)
  client.timeout = 5
  ```

- an assignment shares a line with another statement;

- a comment sits on its lines.

  ```python report
  client = MagicMock()
  client.timeout = 5  # seconds
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
