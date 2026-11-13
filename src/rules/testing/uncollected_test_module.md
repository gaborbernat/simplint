Flags a test module that defines tests but whose file name pytest's
[`python_files`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_files) patterns do not match,
so pytest never collects it.

## Why is this bad?

pytest only
[collects](https://docs.pytest.org/en/stable/explanation/goodpractices.html#conventions-for-python-test-discovery)
`test_*.py` and `*_test.py` files unless the configuration says otherwise. A module named `tests/parser_tests.py` or
`tests/check_parser.py` holds tests that never run, and nothing says so: the run passes, and the coverage report shows
their lines as never run.

## Example

```python report file=tests/check_parser.py
def test_parse():
    assert parse("a = 1")
```

```python ok file=tests/test_lexer.py
def test_lex():
    assert lex("a")
```

Use instead: rename `tests/check_parser.py` to `tests/test_parser.py`.

## What it does

### Active on

- **Modules with tests in a collected directory**: a module of the test suite, other than `conftest.py` and
  `__init__.py`, that defines a `test*` function at module level or in a test class, whose name the `python_files`
  patterns do not match, and whose directory holds at least one module they do match. The patterns come from the same
  configuration files pytest reads, and default to `test_*.py` and `*_test.py`.

### Exceptions

- **Directories pytest does not collect from**: without a collected module next to it, a module is taken as data, such
  as the inputs of a type checker's tests.

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

- **Tests another module imports**: they run where that module is collected.

### Fix

No fix is offered, as renaming a module changes its imports elsewhere.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
