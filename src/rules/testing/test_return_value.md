Flags a `return` with a value in a test function pytest collects.

## Why is this bad?

pytest ignores what a test returns: a test that returns `False` passes. A comparison returned from a test is almost
always an `assert` the author meant to write, so the test checks nothing. pytest only emits a
[`PytestReturnNotNoneWarning`](https://docs.pytest.org/en/stable/reference/reference.html#pytest.PytestReturnNotNoneWarning),
which scrolls past in the warnings summary.

## Example

```python report file=tests/test_total.py
def test_total():
    return total([1, 2]) == 3
```

Use instead:

```python ok file=tests/test_total_asserted.py
def test_total():
    assert total([1, 2]) == 3
```

## What it does

### Active on

- **Collected tests**: `test*` functions at module level and in test classes, other than fixtures, in a module whose
  name pytest's [`python_files`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files)
  collects. Every `return` with a value is reported, including one inside a branch; a nested function or class is
  skipped, as its `return` is its own.

  ```python report file=tests/test_branch.py
  def test_branch():
      if ready():
          return True
      assert recover()
  ```

### Exceptions

- **`return None` and a bare `return`**: pytest reads them as no value.

- **A returned call or `await`**: a test that returns `check_total()` delegates to a helper, which asserts and returns
  `None`.

  ```python ok file=tests/test_delegates.py
  def test_total():
      return check_total([1, 2])
  ```

- **Generators**: a test that yields is not run as a plain test, and its `return` ends the iteration.

- **`test_suite`**: the function a unittest runner calls to build the suite returns it on purpose.

### Fix

When the value is a comparison or a boolean `and` or `or`, the fix turns the `return` into an `assert`. **Unsafe fix:**
the test then fails when the value is false, where before it passed.

```python report file=tests/test_total.py
def test_total():
    return total([1, 2]) == 3
```

```python fixed file=tests/test_total.py
def test_total():
    assert total([1, 2]) == 3
```

Any other value is reported without a fix, as it may not be what the test should check.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
