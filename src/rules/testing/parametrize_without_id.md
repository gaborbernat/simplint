Flags rows of a literal `pytest.mark.parametrize` list that are not `pytest.param(..., id=...)`.

## Why is this bad?

Without an id,
[pytest builds one](https://docs.pytest.org/en/stable/example/parametrize.html#different-options-for-test-ids) from the
values, such as `test_increment[1-2]`, or from the argument name and position, such as `value0`, for values it does not
print. A written id says what the case covers and stays the same when the values change.

## Example

```python report file=tests/test_increment.py
import pytest


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (1, 2),
        (-1, 0),
    ],
)
def test_increment(value: int, expected: int) -> None:
    assert value + 1 == expected
```

Use instead:

```python ok file=tests/test_increment.py
import pytest


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        pytest.param(1, 2, id="positive"),
        pytest.param(-1, 0, id="negative"),
    ],
)
def test_increment(value: int, expected: int) -> None:
    assert value + 1 == expected
```

## What it does

No fix is offered: a meaningful id describes what the case tests, which cannot be derived from its values.

### Active on

- **Rows in a literal [`pytest.mark.parametrize`](https://docs.pytest.org/en/stable/how-to/parametrize.html) list or
  tuple that are not
  [`pytest.param(..., id=...)`](https://docs.pytest.org/en/stable/reference/reference.html#pytest-param)**: without one,
  pytest makes up the id.

  ```python report
  import pytest


  @pytest.mark.parametrize(
      ("value", "expected"),
      [
          (1, 2),
          (-1, 0),
      ],
  )
  def test_increment(value: int, expected: int) -> None:
      assert value + 1 == expected
  ```

- **Rows whose `id` is `None`**: `id=None` counts as no id.

  ```python report
  import pytest


  @pytest.mark.parametrize(("value", "expected"), [pytest.param(1, 2, id=None)])
  def test_increment(value: int, expected: int) -> None:
      assert value + 1 == expected
  ```

### Exceptions

- **Rows built elsewhere**, such as a module constant passed as the argument values: the decorator holds no rows to
  check.

  ```python ok
  import pytest

  CASES = [(1, 2), (-1, 0)]


  @pytest.mark.parametrize(("value", "expected"), CASES)
  def test_increment(value: int, expected: int) -> None:
      assert value + 1 == expected
  ```

- **Decorators that pass `ids=`**: that argument already names every case.

  ```python ok
  import pytest


  @pytest.mark.parametrize(("value", "expected"), [(1, 2), (-1, 0)], ids=["positive", "negative"])
  def test_increment(value: int, expected: int) -> None:
      assert value + 1 == expected
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open pull request
  [#22136](https://github.com/astral-sh/ruff/pull/22136) would flag boolean-only parametrize lists without `ids=`, a
  subset of this rule. Open issue [#14994](https://github.com/astral-sh/ruff/issues/14994) asks the opposite, replacing
  `ids=` with `pytest.param(id=...)`.
