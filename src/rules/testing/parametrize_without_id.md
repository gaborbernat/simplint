## What it does

Checks for rows in a literal [`pytest.mark.parametrize`](https://docs.pytest.org/en/stable/how-to/parametrize.html) list
or tuple that are not
[`pytest.param(..., id=...)`](https://docs.pytest.org/en/stable/reference/reference.html#pytest-param); `id=None` counts
as no id. Rows built elsewhere, such as a module constant passed as the argument values, are not checked. Decorators
that pass `ids=` are skipped.

## Why is this bad?

Without an id,
[pytest builds one](https://docs.pytest.org/en/stable/example/parametrize.html#different-options-for-test-ids) from the
values, such as `test_increment[1-2]`, or from the argument name and position, such as `value0`, for values it does not
print. A written id says what the case covers and stays the same when the values change.

## Example

```python
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

```python
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

## Fix safety

No fix is offered: a meaningful id describes what the case tests, which cannot be derived from its values.
