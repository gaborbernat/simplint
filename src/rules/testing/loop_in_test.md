Flags `for` loops in a test function that run the code under test once per case from a table the test file lists, and
assert on each result, in a test module.

## Why is this bad?

A loop of assertions stops at its first failure, so the report names one case and hides the others, and the test does
not show which cases it covered. A parameterized test runs every case as its own test, and a failure names the case that
broke.

## Example

```python report file=tests/test_squares.py
def test_squares() -> None:
    for number, square in [(2, 4), (3, 9), (4, 16)]:
        assert square_of(number) == square
```

Use instead:

```python ok file=tests/test_squares.py
import pytest


@pytest.mark.parametrize(
    ("number", "square"),
    [
        pytest.param(2, 4, id="two"),
        pytest.param(3, 9, id="three"),
        pytest.param(4, 16, id="four"),
    ],
)
def test_square(number: int, square: int) -> None:
    assert square_of(number) == square
```

## What it does

No fix is offered: moving the cases into parameters changes the test's signature and its ids, which the rule cannot
choose.

A parameter list is fixed when pytest collects the test, so the rule reports only loops whose cases are known by then
and that hand each case to the code under test. Three things must hold:

- **The cases are a table the file lists**: a literal, a module constant, or a name the test assigns once to one of
  these and never fills in later; `enumerate`, `zip`, `sorted`, `reversed`, `list`, `tuple`, `set`, `frozenset`, the
  `itertools` combinations, `.items()`, `.keys()`, `.values()`, `.split()`, slices and `+` over such tables count too.
- **The loop body asserts**: an `assert` statement, or a `self.assert...` call of `unittest`, outside a subtest.
- **Each case reaches the code under test**: a statement other than an assertion reads the loop's target, or an
  assertion passes it to a call, as `assert parse(name)` and `self.assertRaises(ValueError, parse, name)` do.

### Active on

- **Tables of literals and constants**: a list, tuple, set, dict or string written in the loop, or a module constant.

  ```python report file=tests/test_constant.py
  CASES = [(2, 4), (3, 9)]


  def test_squares() -> None:
      for number, square in CASES:
          assert square_of(number) == square
  ```

- **A name the test assigns a table to**, and the builtins that number, pair or reorder it.

  ```python report file=tests/test_local.py
  def test_names() -> None:
      cases = {"two": 2, "three": 3}
      for index, (name, number) in enumerate(cases.items()):
          assert parse(name) == number + index - index
  ```

- **`self.assert...` calls in a `unittest` test method**: `self.assertEqual`, `self.assertTrue` and the rest of the
  family check like an `assert` statement.

  ```python report file=tests/test_case.py
  import unittest


  class TestSquares(unittest.TestCase):
      def test_squares(self) -> None:
          for number, square in [(2, 4), (3, 9)]:
              self.assertEqual(square_of(number), square)
  ```

- **Nested loops**: the outermost loop over a table is reported once, since its nested loops check the same cases.

  ```python report file=tests/test_nested_loops.py
  def test_grid() -> None:
      for row in (0, 1):
          for column in (0, 1):
              assert cell(row, column) is not None
  ```

### Exceptions

- **Subtests**: `with self.subTest(...)` in `unittest`, and `with subtests.test(...)` with pytest's `subtests` fixture,
  run every case to the end and name the failing one in the report, which is what a parameterized test gives. They are
  the way to parameterize a `unittest` test, so assertions inside them are left alone; an assertion of the loop outside
  the subtest still counts.

  ```python ok file=tests/test_subtest.py
  import unittest


  class TestSquares(unittest.TestCase):
      def test_squares(self) -> None:
          for number, square in [(2, 4), (3, 9)]:
              with self.subTest(number=number):
                  self.assertEqual(square_of(number), square)
  ```

- **Values the test computes**: the items of a result, a fixture or a parameter, an attribute such as `module.names`,
  and a list the test fills in are not known when the test is collected. The loop checks that one result.

  ```python ok file=tests/test_items.py
  def test_items() -> None:
      for item in items():
          assert item.valid
  ```

  ```python ok file=tests/test_attribute.py
  import array


  def test_typecodes() -> None:
      for typecode in array.typecodes:
          assert isinstance(typecode, str)
  ```

- **`range` loops**: a sweep over numbers, or a loop that steps through some state, checks a property or a sequence
  rather than a set of named cases.

  ```python ok file=tests/test_range.py
  def test_round_trip() -> None:
      for byte in range(256):
          assert decode(encode(byte)) == byte
  ```

- **Loops that do not hand the case to the code**: a loop that never reads its target repeats one check, and one whose
  assertions only compare each item with a result computed before the loop checks that result. A single assertion on the
  collection, such as `assert {"a", "b"} <= names`, reports every missing item at once.

  ```python ok file=tests/test_repeat.py
  def test_stable() -> None:
      for _ in ("first", "second"):
          assert square_of(2) == 4
  ```

  ```python ok file=tests/test_membership.py
  def test_names() -> None:
      names = parse("a b")
      for name in ("a", "b"):
          assert name in names
  ```

- **`while` and `async for` loops**: they wait for a condition or read a stream, which is not a table of cases.

  ```python ok file=tests/test_while.py
  def test_drain() -> None:
      while not queue.empty():
          assert queue.get()
  ```

- **Assertions in a `try` of the loop, or in its `else` clause**: a retry around an assertion waits for a condition to
  hold, and an assertion in `else` runs once, after a search found nothing.

  ```python ok file=tests/test_retry.py
  def test_ready() -> None:
      for delay in (0.1, 0.2, 0.4):
          try:
              assert ready()
          except AssertionError:
              time.sleep(delay)
          else:
              break
  ```

  ```python ok file=tests/test_search.py
  def test_found() -> None:
      for name in ("a", "b"):
          if parse(name):
              break
      else:
          assert False
  ```

- **Assertions in nested functions and classes**: they run only when the function is called, not as the loop's own
  checks.

  ```python ok file=tests/test_callbacks.py
  def test_callbacks() -> None:
      for number in (1, 2):

          def check() -> None:
              assert number >= 0

          register(check)
  ```

- **Comprehensions that check a collection**: an assertion over a comprehension, such as `assert all(...)`, is a single
  check, so it is not a loop.

  ```python ok file=tests/test_all.py
  def test_all_positive() -> None:
      assert all(square_of(number) > 0 for number in (1, 2))
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#11060](https://github.com/astral-sh/ruff/issues/11060).
- [Sourcery](https://sourcery.ai) flags any loop in a test through
  [no-loop-in-tests](https://docs.sourcery.ai/reference/refactorings/python/no-loop-in-tests/). This rule flags only
  loops that assert over a table of cases.
