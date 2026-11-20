Flags a `return` with a value in a test function pytest collects.

## Why is this bad?

A plain pytest test that returns `False` can pass even when the comparison it returned is false. pytest emits a
[`PytestReturnNotNoneWarning`](https://docs.pytest.org/en/stable/how-to/assert.html#return-not-none) by default. Use
`assert` to make the comparison determine the test result.

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

- **Test functions and methods.** The filename must match
  [`python_files`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files). The rule looks for
  non-fixture `test*` functions at module level, in `Test*` classes and in direct
  [`unittest.TestCase`](https://docs.python.org/3/library/unittest.html#unittest.TestCase) subclasses. It uses these
  prefixes even if pytest has custom class or function naming patterns. The rule also checks returns inside branches. It
  skips nested functions and classes.

  ```python report file=tests/test_branch.py
  def test_branch():
      if ready():
          return True
      assert recover()
  ```

### Exceptions

- **`return None` and a bare `return`**: pytest reads them as no value.

- **A returned call or `await`**: a helper such as `check_total()` may assert and return `None`. The rule skips any
  returned call or `await` without inferring its result.

  ```python ok file=tests/test_delegates.py
  def test_total():
      return check_total([1, 2])
  ```

- **Generators**: a test that yields is not run as a plain test, and its `return` ends the iteration.

- **`test_suite`**: some unittest suites use this name for a factory that returns the suite.

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
