## What it does

Checks modules that define [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) for
top-level functions and classes whose names have no leading underscore and are missing from `__all__`. simplint reads
`__all__` from assignments, `+=`, `.extend([...])` and `.append("name")` with string literals. When any other statement
builds it, such as `list(...)` or `.remove(...)`, or when it is bound inside a nested block such as an `if`, `try` or
function, nothing is reported.

These modules are not checked:

- modules without `__all__`
- test modules: `test_*.py`, `*_test.py`, `conftest.py`, and files under a `tests` or `test` directory

These names are skipped:

- decorated functions and classes, since the decorator may register them
- names another module in the run imports, which [SMP113](SMP113.html) asks you to add to `__all__`, or reaches by a
  dotted string such as a `mocker.patch` target
- [pytest hooks](https://docs.pytest.org/en/stable/reference/reference.html#hooks) (`pytest_*`),
  [`load_tests`](https://docs.python.org/3/library/unittest.html#load-tests-protocol), and
  [`setup_module` and `teardown_module`](https://docs.pytest.org/en/stable/how-to/xunit_setup.html#module-level-setup-teardown)
- `setup`, the entry point Sphinx calls in an [extension](https://www.sphinx-doc.org/en/master/extdev/index.html)

[SMP102](SMP102.html) applies the same convention to module variables.

## Why is this bad?

`__all__` declares the public API. A name that looks public but is missing from it leaves readers unsure whether they
may depend on it. A leading underscore answers that question at every use
([PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces)).

## Example

```python
def exported() -> int:
    return helper()


def helper() -> int:
    return 1


__all__ = ["exported"]
```

Use instead:

```python
def exported() -> int:
    return _helper()


def _helper() -> int:
    return 1


__all__ = ["exported"]
```

## Fix safety

The fix renames the name to `_name` everywhere in the file. It is always unsafe: importers outside the run keep the old
name. If the name is meant to be public, add it to `__all__` instead.

No fix is offered when:

- the new name is already used as a name in the file
- the old name appears in a string, other than an entry of an `__all__` list assigned with `=` or `+=`
- the old name also names something a text rename cannot tell apart: a parameter, a local variable, a class attribute, a
  keyword argument or an attribute access
