Flags a module that defines tests but whose filename falls outside the configured
[`python_files` patterns](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files).

## Why is this bad?

By default, pytest
[discovers](https://docs.pytest.org/en/stable/explanation/goodpractices.html#conventions-for-python-test-discovery)
files named `test_*.py` or `*_test.py`. A plain `pytest` run can pass without running any of the tests in
`tests/check_parser.py`. Passing that filename explicitly still runs them.

## Example

```python report file=tests/check_parser.py
def test_parse():
    assert parse("a = 1")
```

```python ok file=tests/test_lexer.py
def test_lex():
    assert lex("a")
```

Rename `tests/check_parser.py` to `tests/test_parser.py`.

## What it does

### Active on

- **Modules with tests in a collected directory**: a module of the test suite, other than `conftest.py` and
  `__init__.py`, that defines a `test*` function at module level or in a test class, whose name the `python_files`
  patterns do not match, and whose directory holds at least one module they do match. simplint reads the patterns from
  its supported pytest configuration files and defaults to `test_*.py` and `*_test.py`.

### Exceptions

- **Directories without a matching sibling.** A directory of test inputs, such as samples for a type checker, may
  contain functions named `test_*`. The rule requires a sibling `.py` file matching `python_files`, excluding
  `conftest.py`.

- **Modules kept out of the run on purpose**: a module with a `__main__` guard is a script, a module whose name appears
  in a string of the project is data another test hands to `pytester`, and a module whose docstring names its own file
  says how to run it explicitly.

  ```python ok file=tests/live_check.py
  """Live tests against the real service: `pytest tests/live_check.py`."""


  def test_live():
      assert ping()
  ```

  ```python ok file=tests/test_offline.py
  def test_offline():
      assert not ping(offline=True)
  ```

- **Imported tests.** If another module imports a test by name, pytest may collect it there. The rule then skips the
  source module.

### Fix

No fix is offered, as renaming a module changes its imports elsewhere.
