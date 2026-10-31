## What it does

Checks test modules for top-level classes whose name starts with `Test` and that define a method whose name starts with
`test`. With its default
[`python_classes`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_classes) and
[`python_functions`](https://docs.pytest.org/en/stable/reference/reference.html#confval-python_functions) settings,
these are the classes pytest collects tests from. A file is a test module when its name starts with `test_`, ends with
`_test.py` or is `conftest.py`, or when it sits under a `tests` or `test` directory.

## Why is this bad?

A test class adds a level of nesting and a `self` that most tests never use. It also invites shared state through
attributes and [setup methods](https://docs.pytest.org/en/stable/how-to/xunit_setup.html). Module-level functions with
the class name folded into a prefix keep the grouping visible in the test name.

## Example

```python
class TestParser:
    def test_handles_empty(self) -> None:
        assert parse("") == []
```

Use instead:

```python
def test_parser_handles_empty() -> None:
    assert parse("") == []
```

## Fix safety

The fix is always unsafe: it renames every test, which changes its
[node ID](https://docs.pytest.org/en/stable/how-to/usage.html#nodeids) and breaks `-k` selections or deselect lists that
name it. The prefix is the class name with one leading `Test` removed, in snake case with acronyms kept together, so
`TestHTTPClient.test_get` becomes `test_http_client_get`. A class named just `Test` keeps its method names.

No fix is offered when:

- the class has bases, decorators or a docstring;
- another class extends it, or the file refers to it by name;
- the class contains a string that spans several lines;
- the class body holds anything besides methods whose names start with `test`;
- a method uses `self`, has no `self` as its first parameter, or takes positional-only parameters;
- a new name is already used in the file.
