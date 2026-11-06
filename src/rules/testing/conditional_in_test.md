Flags `if` statements, `match` statements and conditional expressions in test functions.

## Why is this bad?

A branching test runs a different check depending on its input or environment, so a passing run does not say which path
was verified. With one [parametrized](https://docs.pytest.org/en/stable/how-to/parametrize.html) case per path, pytest
reports each path on its own.

## Example

```python report file=tests/test_paths.py
import sys


def test_separator() -> None:
    if sys.platform == "win32":
        assert separator() == "\\"
    else:
        assert separator() == "/"
```

Use instead:

```python ok file=tests/test_paths.py
import sys

import pytest


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

## What it does

No fix is offered: splitting a test into cases needs knowledge of what each branch tests.

### Active on

- **`if` statements, `match` statements and conditional expressions in a test function**: a test function is a function
  or method whose name starts with `test`, in a test module as defined in [test-class](SMP801.html). Each branch picks
  which check runs.

  ```python report file=tests/test_if.py
  def test_separator() -> None:
      if sys.platform == "win32":
          assert separator() == "\\"
      else:
          assert separator() == "/"
  ```

  ```python report file=tests/test_match.py
  def test_kind(kind: str) -> None:
      match kind:
          case "a":
              assert parse(kind)
          case _:
              assert not parse(kind)
  ```

  ```python report file=tests/test_ternary.py
  def test_length() -> None:
      assert (1 if sys.platform == "win32" else 2) == separator_length()
  ```

### Exceptions

- **Branches inside nested functions, classes and lambdas**: they run only when that code is called, not as part of the
  test's own path.

  ```python ok file=tests/test_nested.py
  def test_callbacks() -> None:
      def check(value: int) -> None:
          if value:
              assert value

      class Helper:
          def pick(self, value: int) -> int:
              return 1 if value else 2

      assert (lambda value: 1 if value else 2)(0) == 2
      check(1)
      assert Helper().pick(0) == 2
  ```

- **`if` clauses in comprehensions**: they filter the items a comprehension builds, rather than pick which check runs.

  ```python ok file=tests/test_comprehension.py
  def test_filtered(items: list[int]) -> None:
      assert [item for item in items if item] == [1, 2]
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#11060](https://github.com/astral-sh/ruff/issues/11060).
- [Sourcery](https://sourcery.ai) flags conditionals in tests through
  [no-conditionals-in-tests](https://docs.sourcery.ai/reference/refactorings/python/no-conditionals-in-tests/).
