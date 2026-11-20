Flags a test whose body contains no assertion, exception or other check the rule recognizes.

## Why is this bad?

In the example below, `parse()` could return the wrong node and the test would still pass. Assert on the node the parser
should produce.

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

- **Test functions and methods.** The filename must match
  [`python_files`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files). The rule checks
  non-fixture `test*` functions at module level, in `Test*` classes and in direct
  [`unittest.TestCase`](https://docs.python.org/3/library/unittest.html#unittest.TestCase) subclasses. It uses the
  default class and function prefixes even if pytest has custom naming patterns.

These count as checks:

- an `assert` or a `raise` statement;
- a call whose name, ignoring case and leading underscores, contains `assert`, `raises` or `warns`, or starts with
  `check`, `verify`, `expect`, `validate`, `ensure`, `compare` or `fail`, so `self.assertEqual`,
  `mock.assert_called_once()`, `np.testing.assert_allclose` and Playwright's `expect` count;
- `pytest.fail`, `pytest.skip`, `pytest.xfail`, `pytest.importorskip` and `pytest.deprecated_call`;
- pytester's line matchers, such as `result.stdout.fnmatch_lines(...)`;
- a warning filter set to `error`, by
  [`warnings.simplefilter("error")`](https://docs.python.org/3/library/warnings.html#warnings.simplefilter),
  [`warnings.catch_warnings(action="error")`](https://docs.python.org/3/library/warnings.html#warnings.catch_warnings)
  or a `filterwarnings("error")` mark, which turns matching warnings into errors;
- a call of a test parameter, as a callable fixture may check what it runs, a call of a name imported from a test
  package (a relative import, or a module path with `test` in it, such as `conftest` or `numpy.testing`), and a call
  through `super()`, which can delegate a check to a base class;
- a call of a function, class or method the module defines whose body checks, followed through further calls; a method
  of `self` the class does not define is taken to check, as a base test class's helpers do;
- a function the module defines, or a method of the test's class, passed to a call as a callback, such as
  `bus.handlers.append(self._check)`, if the callback contains a check. The rule does not trace whether the callback
  runs.

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

- **Tests decorated with `skip` or `xfail`.** The rule skips these even when their decorators have a condition.

- **Benchmarks**: the rule skips tests with a `benchmark` or `aio_benchmark` parameter.

  ```python ok file=tests/test_benchmark.py
  def test_parse_speed(benchmark):
      benchmark(parse, "a = 1")
  ```

### Fix

No fix is offered, as only the author knows what the test should check.
