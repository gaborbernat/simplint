Flags test classes in test modules: top-level `Test` classes with `test` methods. A test class adds a level of nesting
and a `self` that most tests never use, and invites shared state.

## Why is this bad?

A test class adds a level of nesting and a `self` that most tests never use. It also invites shared state through
attributes and [setup methods](https://docs.pytest.org/en/stable/how-to/xunit_setup.html). Module-level functions with
the class name folded into a prefix keep the grouping visible in the test name.

## Example

```python report file=tests/test_parser.py
class TestParser:
    def test_handles_empty(self) -> None:
        assert parse("") == []
```

Use instead:

```python ok file=tests/test_parser.py
def test_parser_handles_empty() -> None:
    assert parse("") == []
```

## What it does

### Active on

- **Top-level classes in a test module whose name starts with `Test` and that define a method whose name starts with
  `test`**: with its default
  [`python_classes`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_classes) and
  [`python_functions`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_functions) settings,
  these are the classes pytest collects tests from. A file is a test module when its name starts with `test_`, ends with
  `_test.py` or is `conftest.py`, or when it sits under a `tests` or `test` directory. **Unsafe fix:** always, as it
  renames every test, which changes its [node ID](https://docs.pytest.org/en/stable/how-to/usage.html#nodeids) and
  breaks `-k` selections or deselect lists that name it.

  ```python report file=tests/test_parser.py
  class TestParser:
      def test_handles_empty(self) -> None:
          assert parse("") == []
  ```

### Fix

The fix turns each method into a module-level function whose name carries a prefix: the class name with one leading
`Test` removed, in snake case with acronyms kept together, so `TestHTTPClient.test_get` becomes `test_http_client_get`.
A class named just `Test` keeps its method names.

```python report file=tests/test_http.py
class TestHTTPClient:
    def test_get(self) -> None:
        assert get("/") == 200
```

```python fixed file=tests/test_http.py
def test_http_client_get() -> None:
    assert get("/") == 200
```

No fix is offered when:

- the class has bases or decorators, or a docstring, which module-level functions have nowhere to keep;

  ```python report file=tests/test_based.py
  class TestParser(BaseParserTests):
      def test_handles_empty(self) -> None:
          assert parse("") == []
  ```

- another class extends it, or the file refers to it by name, so it must stay a class;

- the class contains a string that spans several lines, other than a method's docstring, whose indentation `ruff format`
  rewrites anyway;

- the class body holds anything besides methods whose names start with `test`;

- a method uses `self`, has no `self` as its first parameter, or takes positional-only parameters;

  ```python report file=tests/test_uses_self.py
  class TestParser:
      def test_handles_empty(self) -> None:
          assert self.parse("") == []
  ```

- a new name is already used in the file.
