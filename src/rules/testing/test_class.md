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

Each method is dedented by its class body's indentation, not by the file's first indented block, so a file that indents
another block by a different width still gets valid code.

```python report file=tests/test_thing.py
# fmt: off
def helper():
  return 1


class TestThing:
    def test_one(self):
        assert helper() == 2
# fmt: on
```

```python fixed file=tests/test_thing.py
# fmt: off
def helper():
  return 1


def test_thing_one():
    assert helper() == 2
# fmt: on
```

A method that uses its class's type parameters gets them on its function, with their bounds, defaults and the other
parameters they name. A method that names none gets none. Defaults on type parameters need Python 3.13, so below that
target a method that uses a defaulted parameter gets no fix.

```toml config
[project]
requires-python = ">=3.12"
```

```python report file=tests/test_box.py
class TestBox[T: int]:
    def test_default(self, value: T | None = None) -> None:
        assert value is None
```

```python fixed file=tests/test_box.py
def test_box_default[T: int](value: T | None = None) -> None:
    assert value is None
```

No fix is offered when:

- the class has bases or decorators, or a docstring, which module-level functions have nowhere to keep;

  ```python report file=tests/test_based.py
  class TestParser(BaseParserTests):
      def test_handles_empty(self) -> None:
          assert parse("") == []
  ```

- a comment in the class is neither inside a method nor on the lines directly above one, such as a comment on the class
  line, which no function would carry;

  ```python report file=tests/test_noted.py
  class TestParser:  # grouped for the report
      def test_handles_empty(self) -> None:
          assert parse("") == []
  ```

- a method's decorators or default values name a type parameter of its class, which is not bound where they are
  evaluated;

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report file=tests/test_generic.py
  class TestBox[T]:
      def test_holds_value(self, kind: type = list[T]) -> None:
          assert Box[int]().get(1) == 1
  ```

- another class extends it, or the file refers to it by name, so it must stay a class;

- a method keeps its name, as in `class Test`, and the module already binds that name, so the flattened function would
  replace it;

  ```python report file=tests/test_shadow.py
  def test_value() -> None:
      assert 1 == 2


  class Test:
      def test_value(self) -> None:
          assert 1 == 1
  ```

  ```python fixed file=tests/test_shadow.py
  def test_value() -> None:
      assert 1 == 2


  class Test:
      def test_value(self) -> None:
          assert 1 == 1
  ```

- the class contains a string that spans several lines, other than a method's docstring, whose indentation `ruff format`
  rewrites anyway;

- the class body holds anything besides methods whose names start with `test`;

- a method uses `self`, has no `self` as its first parameter, or takes positional-only parameters;

  ```python report file=tests/test_uses_self.py
  class TestParser:
      def test_handles_empty(self) -> None:
          assert self.parse("") == []
  ```

- a new name is already used in the file, or an earlier class of the file gets the same name; the earlier class keeps
  its fix;

  ```python report file=tests/test_parse.py
  class TestParse:
      def test_error_value(self) -> None:
          assert parse("1") == 1


  class TestParseError:
      def test_value(self) -> None:
          assert parse("2") == 3
  ```

  ```python fixed file=tests/test_parse.py
  def test_parse_error_value() -> None:
      assert parse("1") == 1


  class TestParseError:
      def test_value(self) -> None:
          assert parse("2") == 3
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
