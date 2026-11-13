Flags `if` statements, `match` statements and conditional expressions in test functions.

## Why is this bad?

A branching test runs a different check depending on its input, so a passing run does not say which path was verified.
Google's testing guide asks for tests that are
[correct at a glance, with no conditionals or loops](https://abseil.io/resources/swe-book/html/ch12.html), since a bug
in test logic is easy to miss. With one [parametrized](https://docs.pytest.org/en/stable/how-to/parametrize.html) case
per path, pytest reports each path on its own.

## Example

```python report file=tests/test_parse.py
def test_parse(text: str) -> None:
    if text.startswith("-"):
        assert parse(text) < 0
    else:
        assert parse(text) >= 0
```

Use instead:

```python ok file=tests/test_parse.py
import pytest


@pytest.mark.parametrize(
    ("text", "negative"),
    [
        pytest.param("-1", True, id="negative"),
        pytest.param("1", False, id="positive"),
    ],
)
def test_parse(text: str, negative: bool) -> None:
    assert (parse(text) < 0) is negative
```

## What it does

No fix is offered: splitting a test into cases needs knowledge of what each branch tests.

### Active on

- **`if` statements, `match` statements and conditional expressions in a test function**: a test function is a function
  or method whose name starts with `test`, in a test module as defined in [test-class](SMP801.html). Each branch picks
  which check runs.

  ```python report file=tests/test_if.py
  def test_parse(text: str) -> None:
      if text.startswith("-"):
          assert parse(text) < 0
      else:
          assert parse(text) >= 0
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
  def test_length(text: str) -> None:
      assert (1 if text else 2) == width(text)
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

- **Branches on the interpreter or its environment**: a test whose condition reads only `sys.platform`,
  `sys.version_info`, `sys.implementation`, `os.name`, `os.environ`, `os.getenv`, the `platform` module, literals, or a
  constant that names such a check, such as `IS_WINDOWS` or `ON_PYPY`. The branch follows the machine running the test,
  so it cannot become a parameter.

  ```python ok file=tests/test_exe.py
  import sys


  def test_exe_name() -> None:
      name = f"python{'.exe' if sys.platform == 'win32' else ''}"
      assert name.startswith("python")
  ```

- **Branches on what a loop received**: inside a `while` loop, a condition that reads a name the loop body assigns;
  inside a `for` loop over anything but a literal collection or `range()`, a condition that reads the loop variable.
  Such a loop drives an exchange, such as a handshake or a stream of events, and its branches follow what arrives.

  ```python ok file=tests/test_stream.py
  def test_stream(connection: Connection) -> None:
      for event in connection.events():
          if event.type == "delta":
              assert event.text
  ```

- **`if` clauses and conditional items in comprehensions**: they shape the items a comprehension builds, rather than
  pick which check runs.

  ```python ok file=tests/test_comprehension.py
  def test_filtered(items: list[int]) -> None:
      assert [item for item in items if item] == [1, 2]
  ```

  ```python ok file=tests/test_comprehension_item.py
  def test_labels(tags: list[str | None]) -> None:
      assert [tag if tag is not None else "-" for tag in tags] == ["a", "-"]
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#11060](https://github.com/astral-sh/ruff/issues/11060).
- [Sourcery](https://sourcery.ai) flags conditionals in tests through
  [no-conditionals-in-tests](https://docs.sourcery.ai/reference/refactorings/python/no-conditionals-in-tests/).
