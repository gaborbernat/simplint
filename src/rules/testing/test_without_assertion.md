Flags a test function pytest collects that contains no check: no `assert`, no `raise`, and no call that can fail the
test.

## Why is this bad?

A test without a check passes for any code that does not raise, including code that returns the wrong result. Ned
Batchelder's [Getting Started Testing](https://nedbatchelder.com/text/test0.html) makes the point that a test is only
worth as much as what it asserts. A test that only runs the code reads as coverage of its behavior, while it covers no
more than the absence of an exception.

## Example

```python report file=tests/test_parser.py
def test_parse():
    parse("a = 1")
```

Use instead:

```python ok file=tests/test_parser_checked.py
def test_parse():
    assert parse("a = 1") == Assign("a", 1)
```

## What it does

### Active on

- **Collected tests**: `test*` functions at module level and in test classes, other than fixtures, in a module whose
  name pytest's [`python_files`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files)
  collects, with no check anywhere in their body.

These count as checks:

- an `assert` or a `raise` statement;
- a call whose name contains `assert`, `raises` or `warns`, or starts with `check`, `verify`, `expect`, `validate`,
  `ensure`, `compare` or `fail`, so `self.assertEqual`, `mock.assert_called_once()`, `np.testing.assert_allclose` and
  Playwright's `expect` count;
- `pytest.fail`, `pytest.skip`, `pytest.xfail`, `pytest.importorskip` and `pytest.deprecated_call`;
- pytester's line matchers, such as `result.stdout.fnmatch_lines(...)`;
- a warning filter set to `error`, by `warnings.simplefilter("error")`, `warnings.catch_warnings(action="error")` or a
  `filterwarnings("error")` mark, which fails the test on any warning;
- a call of a test parameter, as a callable fixture may check what it runs, a call of a name imported from a test
  package (a relative import, or a module path with `test` in it, such as `conftest` or `numpy.testing`), and a call
  through `super()`, which runs a base class's test;
- a call of a function, class or method the module defines whose body checks, followed through further calls; a method
  of `self` the class does not define is taken to check, as a base test class's helpers do.

```python report file=tests/test_helpers.py
def test_runs():
    prepare()


def prepare():
    load("config.toml")
```

### Exceptions

- **Smoke and regression tests**: a test whose name, docstring or comments mention `smoke`, `sanity`, `crash`,
  `segfault`, `raise`, `error`, `hang`, `leak`, `works` or `import` says it only checks the code runs cleanly.

  ```python ok file=tests/test_smoke.py
  def test_large_input():
      # regression test for gh-123: this used to segfault
      parse("x" * 10_000_000)
  ```

- **Tests marked `skip` or `xfail`**: such a test is a placeholder.

- **Benchmarks**: a test that requests pytest-benchmark's `benchmark` fixture measures rather than checks.

  ```python ok file=tests/test_benchmark.py
  def test_parse_speed(benchmark):
      benchmark(parse, "a = 1")
  ```

### Fix

No fix is offered, as only the author knows what the test should check.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
