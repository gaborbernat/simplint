## What it does

Checks test functions for `if` statements, `match` statements and conditional expressions. A test function is a function
or method whose name starts with `test`, in a test module as defined in [SMP200](SMP200.html). Branches inside nested
functions, classes and lambdas, and `if` clauses in comprehensions, are not reported.

## Why is this bad?

A branching test runs a different check depending on its input or environment, so a passing run does not say which path
was verified. With one [parametrized](https://docs.pytest.org/en/stable/how-to/parametrize.html) case per path, pytest
reports each path on its own.

## Example

```python
def test_separator() -> None:
    if sys.platform == "win32":
        assert separator() == "\\"
    else:
        assert separator() == "/"
```

Use instead:

```python
@pytest.mark.parametrize(
    ("platform", "expected"),
    [
        pytest.param("win32", "\\", id="windows"),
        pytest.param("linux", "/", id="linux"),
    ],
)
def test_separator(monkeypatch: pytest.MonkeyPatch, platform: str, expected: str) -> None:
    monkeypatch.setattr(sys, "platform", platform)
    assert separator() == expected
```

## Fix safety

No fix is offered: splitting a test into cases needs knowledge of what each branch tests.
